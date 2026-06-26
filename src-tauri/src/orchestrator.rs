use crate::auth::fetch_auth;
use crate::client_version::{detect_region_from_log, fetch_client_version, Region};
use crate::discord::{resolve_app_id, Rpc};
use crate::encounter::EncounterStore;
use crate::fetcher::{build_rows, build_self, fetch_history, refresh_rows, MatchStats};
use std::collections::HashMap;
use crate::lockfile::read_lockfile;
use crate::match_state::current_state;
use crate::model::{MatchState, MatchView, PlayerRow};
use crate::presence::{describe_activity, fetch_self_presence, is_ffa, mode_name};
use crate::static_cache::{load_or_fetch, StaticData};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub fn assemble_view(
    state: MatchState,
    mode: String,
    rows: Vec<PlayerRow>,
    stale: bool,
) -> MatchView {
    MatchView {
        state,
        mode,
        activity: String::new(),
        players: rows,
        me: None,
        history: Vec::new(),
        stale,
        phase_time: 0,
        map: String::new(),
        map_image: String::new(),
        ally_score: 0,
        enemy_score: 0,
    }
}

fn resolve_region() -> Region {
    if let Ok(region) = std::env::var("VAL_REGION") {
        let shard = std::env::var("VAL_SHARD").unwrap_or_else(|_| region.clone());
        return Region { region, shard };
    }
    detect_region_from_log().unwrap_or_else(|| Region {
        region: "na".to_string(),
        shard: "na".to_string(),
    })
}

// Refresh the self profile roughly every this many polls (3s each), instead of
// every poll, to stay well under Riot's rate limits.
const SELF_REFRESH_EVERY: u32 = 10;

fn unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

async fn poll_once(
    sd: &StaticData,
    region: &Region,
    version: &mut Option<String>,
    last: &MatchView,
    last_match_id: &mut Option<String>,
    last_loop_state: &mut String,
    self_tick: &mut u32,
    match_cache: &mut HashMap<String, MatchStats>,
    encounters: &mut EncounterStore,
    fetch_combat: bool,
) -> Option<MatchView> {
    let lf = match read_lockfile() {
        Ok(lf) => lf,
        Err(_) => {
            *last_match_id = None;
            return Some(assemble_view(MatchState::NoGame, String::new(), Vec::new(), false));
        }
    };
    let ctx = fetch_auth(&lf).await.ok()?;
    if version.is_none() {
        *version = fetch_client_version().await.ok();
    }
    let v = version.clone()?;

    // Refresh the profile only periodically (or if we have none yet). Keep the
    // last known value on a transient failure instead of flashing unranked.
    *self_tick = self_tick.wrapping_add(1);
    let refresh_self = last.me.is_none() || *self_tick % SELF_REFRESH_EVERY == 0;
    let me = if refresh_self {
        build_self(&ctx, region, &v, sd).await.or_else(|| last.me.clone())
    } else {
        last.me.clone()
    };
    let history = if refresh_self {
        let fresh = fetch_history(&ctx, region, &v, sd, match_cache).await;
        if fresh.is_empty() {
            last.history.clone()
        } else {
            fresh
        }
    } else {
        last.history.clone()
    };

    // Once a recorded match shows up in our own history its result is known, so
    // credit the win or loss to everyone who was in it.
    if refresh_self {
        for id in encounters.pending_ids() {
            if let Some(stats) = match_cache.get(&id) {
                encounters.apply_outcome(&id, stats.won);
            }
        }
    }

    let presence = fetch_self_presence(&lf, &ctx.puuid).await;
    let queue_id = presence
        .as_ref()
        .map(|p| p.queue_id.clone())
        .unwrap_or_default();
    let mode = mode_name(&queue_id);
    let loop_state = presence.as_ref().map(|p| p.loop_state.as_str()).unwrap_or("");
    let activity = presence
        .as_ref()
        .map(|p| describe_activity(p, &mode))
        .unwrap_or_else(|| "Idle".to_string());

    let with_me = |view: MatchView| MatchView {
        me: me.clone(),
        history: history.clone(),
        activity: activity.clone(),
        ..view
    };

    // Detect the game state transition from the local presence (a free call).
    // Riot's match servers are only touched when we are actually in a pregame
    // or game, and in-game only once per match, like vry does it.
    let in_match = loop_state == "PREGAME" || loop_state == "INGAME";
    let entered = last_loop_state.as_str() != loop_state;
    *last_loop_state = loop_state.to_string();

    if !in_match {
        *last_match_id = None;
        return Some(with_me(assemble_view(MatchState::Menu, mode, Vec::new(), false)));
    }

    // Hit Riot's match endpoints on the transition into a pregame or game, then
    // keep retrying the cheap glz endpoints until the roster for the current
    // phase is actually loaded. When a game starts, the core-game endpoint lags
    // behind the presence flip to INGAME, so a single fetch at the transition
    // often still returns only the pregame allies. Without this, that stale
    // ally-only roster would be reused for the whole match and the enemies would
    // never appear. Once the full core-game roster is loaded we settle and reuse
    // it, so this does not turn into continuous polling.
    let loaded = !last.players.is_empty()
        && (loop_state != "INGAME" || last.state == MatchState::CoreGame);
    if !entered && loaded {
        return Some(with_me(assemble_view(last.state, mode, last.players.clone(), false)));
    }

    let cs = current_state(&ctx, region, &v).await;
    let state = cs.state;
    let match_id = cs.match_id;
    let raw = cs.players;
    let phase_time = cs.phase_time;
    if raw.is_empty() {
        *last_match_id = None;
        return Some(with_me(assemble_view(state, mode, Vec::new(), false)));
    }

    // Fetch every player's rank only when the match changes. Within the same
    // match, refresh the cheap fields (agent, team) and keep cached ranks.
    let same_match = match_id.is_some() && *last_match_id == match_id && !last.players.is_empty();
    let mut rows = if same_match {
        refresh_rows(&last.players, &raw, sd, &ctx.puuid)
    } else {
        // Premium-skin detection needs the core-game match id; pregame has none.
        let cg_match_id = if state == MatchState::CoreGame {
            match_id.as_deref()
        } else {
            None
        };
        let mut fetched =
            build_rows(&ctx, region, &v, &raw, sd, &last.players, fetch_combat, cg_match_id).await;
        // Show how often we have seen each player and our record with them, then
        // record this game so later lobbies can show it. Reading the prior count
        // before recording keeps the current match out of the shown total.
        let now = unix_secs();
        for row in &mut fetched {
            if row.puuid == ctx.puuid {
                continue;
            }
            let (seen, wins, losses) = encounters.prior(&row.puuid);
            row.encounters = seen;
            row.encounter_wins = wins;
            row.encounter_losses = losses;
        }
        if state == MatchState::CoreGame {
            if let Some(mid) = match_id.as_deref() {
                let roster: Vec<(String, String, u32)> = fetched
                    .iter()
                    .filter(|r| r.puuid != ctx.puuid)
                    .map(|r| (r.puuid.clone(), r.name.clone(), r.rank_tier))
                    .collect();
                encounters.record_seen(mid, &roster, now);
            }
        }
        *last_match_id = match_id;
        fetched
    };
    if is_ffa(&queue_id) {
        for row in &mut rows {
            row.team.clear();
        }
    }
    let mut view = with_me(assemble_view(state, mode, rows, false));
    view.phase_time = phase_time;
    if let Some(p) = presence.as_ref() {
        view.map = sd.map_name(&p.party_owner_match_map);
        view.map_image = sd.map_image(&p.party_owner_match_map);
        view.ally_score = p.ally_score;
        view.enemy_score = p.enemy_score;
    }
    Some(view)
}

pub async fn run_loop(
    app: AppHandle,
    rpc_enabled: Arc<AtomicBool>,
    combat_enabled: Arc<AtomicBool>,
) {
    let base_dir = app
        .path()
        .app_cache_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let cache_dir = base_dir.join("static");
    let static_data = load_or_fetch(&cache_dir).await;
    let mut encounters = EncounterStore::load(base_dir.join("encounters.json"));
    let region = resolve_region();
    let mut version = fetch_client_version().await.ok();
    let mut last = assemble_view(MatchState::NoGame, String::new(), Vec::new(), false);

    let start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mut rpc = Rpc::new(resolve_app_id(), start);
    let mut last_match_id: Option<String> = None;
    let mut last_loop_state = String::new();
    let mut self_tick = 0u32;
    let mut match_cache: HashMap<String, MatchStats> = HashMap::new();

    loop {
        let view = match poll_once(
            &static_data,
            &region,
            &mut version,
            &last,
            &mut last_match_id,
            &mut last_loop_state,
            &mut self_tick,
            &mut match_cache,
            &mut encounters,
            combat_enabled.load(Ordering::Relaxed),
        )
        .await
        {
            Some(view) => {
                last = view.clone();
                view
            }
            // Surface a stale badge only if we had a populated table; otherwise
            // this is idle time with no game ready.
            None if last.players.is_empty() => {
                assemble_view(MatchState::NoGame, String::new(), Vec::new(), false)
            }
            None => {
                let mut stale = last.clone();
                stale.stale = true;
                stale
            }
        };

        let _ = app.emit("match-view", &view);
        rpc.update(&view, rpc_enabled.load(Ordering::Relaxed));
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembles_view_passthrough() {
        let v = assemble_view(MatchState::CoreGame, "Competitive".to_string(), Vec::new(), false);
        assert_eq!(v.state, MatchState::CoreGame);
        assert_eq!(v.mode, "Competitive");
        assert!(!v.stale);
        assert!(v.players.is_empty());
    }
}
