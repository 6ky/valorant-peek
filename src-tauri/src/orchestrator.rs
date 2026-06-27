use crate::auth::fetch_auth;
use crate::client_version::{detect_region_from_log, fetch_client_version, Region};
use crate::discord::{resolve_app_id, Rpc};
use crate::encounter::EncounterStore;
use crate::fetcher::{
    build_rows, build_self, enrich_combat, fetch_current_act, fetch_history, refresh_rows,
    MatchStats,
};
use std::collections::{HashMap, HashSet};
use crate::lockfile::read_lockfile;
use crate::match_state::current_state;
use crate::model::{MatchState, MatchView, PlayerRow};
use crate::presence::{describe_activity, fetch_self_presence, is_ffa, mode_name};
use crate::static_cache::{load_or_fetch, StaticData};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;

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
        combat_loading: false,
        history_queue: 0,
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

// How many players to fill combat stats for per poll, so they appear in batches
// rather than all at once after a long wait.
const COMBAT_CHUNK: usize = 3;

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
    current_act: &mut Option<String>,
    last: &MatchView,
    last_match_id: &mut Option<String>,
    last_loop_state: &mut String,
    self_tick: &mut u32,
    match_cache: &mut HashMap<String, MatchStats>,
    encounters: &mut EncounterStore,
    combat_done: &mut Option<String>,
    combat_attempted: &mut HashSet<String>,
    mates_map: &mut HashMap<String, HashSet<String>>,
    fetch_combat: bool,
    history_queue: u8,
    force_history: bool,
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

    // The active act rarely changes, so fetch it once per session. If the
    // content service is unavailable it stays None and parse_mmr falls back to
    // the last competitive game's tier.
    if current_act.is_none() {
        *current_act = fetch_current_act(&ctx, region, &v).await;
    }
    let act = current_act.as_deref().unwrap_or("");

    // Ally party ids come from the local presence feed (covers the whole allied
    // team in a match). A cheap local call, so fetch it up front for build_self,
    // build_rows, and enrich_combat. Enemies are absent here and inferred later.
    let party_map = crate::presence::fetch_party_map(&lf).await;

    // Refresh the profile only periodically (or if we have none yet). Keep the
    // last known value on a transient failure instead of flashing unranked.
    *self_tick = self_tick.wrapping_add(1);
    let refresh_self = last.me.is_none() || *self_tick % SELF_REFRESH_EVERY == 0;
    let me = if refresh_self {
        build_self(&ctx, region, &v, act, sd, &party_map).await.or_else(|| last.me.clone())
    } else {
        last.me.clone()
    };
    // On an explicit mode switch, show the fetched list as-is even when it is
    // empty, so a mode with no games reads as empty instead of keeping the
    // previous mode's matches. On the periodic refresh, keep the last list on an
    // empty result so a transient failure does not blank a populated table.
    let history = if force_history {
        fetch_history(&ctx, region, &v, sd, match_cache, history_queue).await
    } else if refresh_self {
        let fresh = fetch_history(&ctx, region, &v, sd, match_cache, history_queue).await;
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
        history_queue,
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
    // Reuse the roster only during an active game, where agents and ranks are
    // fixed. In agent select we keep refetching so picks, lock state, and the
    // countdown stay current. Even while reusing, the live score and map are
    // refreshed from presence, which we read every poll.
    // Do not settle on a half-loaded roster: a real two-team game must have an
    // enemy present, otherwise the core-game endpoint has only returned the
    // allied half so far and we keep fetching. Free-for-all modes have no teams,
    // so the check does not apply to them.
    let roster_loaded =
        is_ffa(&queue_id) || last.players.iter().any(|r| r.team == "Enemy");
    let settled = roster_loaded
        && !last.players.is_empty()
        && loop_state == "INGAME"
        && last.state == MatchState::CoreGame
        && (!fetch_combat || combat_done.as_deref() == last_match_id.as_deref());
    if !entered && settled {
        let mut view = with_me(assemble_view(last.state, mode, last.players.clone(), false));
        if let Some(p) = presence.as_ref() {
            view.map = sd.map_name(&p.party_owner_match_map);
            view.map_image = sd.map_image(&p.party_owner_match_map);
            view.ally_score = p.ally_score;
            view.enemy_score = p.enemy_score;
        }
        return Some(view);
    }

    let cs = current_state(&ctx, region, &v, loop_state).await;
    let state = cs.state;
    let match_id = cs.match_id;
    let cur_match_id = match_id.clone();
    let raw = cs.players;
    let phase_time = cs.phase_time;
    if raw.is_empty() {
        *last_match_id = None;
        return Some(with_me(assemble_view(state, mode, Vec::new(), false)));
    }

    // Fetch every player's rank only when the match changes, or when new players
    // (the late-arriving enemy half) appear and need their ranks, names and
    // loadouts fetched. Within a fully loaded match, refresh only the cheap
    // fields (agent, team, lock state) and keep the cached ranks.
    let roster_complete = raw
        .iter()
        .all(|p| last.players.iter().any(|r| r.puuid == p.puuid));
    let new_match = match_id.is_some() && *last_match_id != match_id;
    let same_match =
        !new_match && match_id.is_some() && !last.players.is_empty() && roster_complete;
    let mut rows = if same_match {
        refresh_rows(&last.players, &raw, sd, &ctx.puuid)
    } else {
        // Premium-skin detection needs the core-game match id; pregame has none.
        let cg_match_id = if state == MatchState::CoreGame {
            match_id.as_deref()
        } else {
            None
        };
        // Phase one: ranks, names, agents, party, and skins only, so the roster
        // appears fast. Combat stats are filled in by a throttled second pass.
        let mut fetched =
            build_rows(&ctx, region, &v, act, &raw, sd, &last.players, &party_map, false, cg_match_id).await;
        // Show how often we have seen each player and our record with them, then
        // record this game so later lobbies can show it. Reading the prior count
        // before recording keeps the current match out of the shown total.
        for row in &mut fetched {
            if row.puuid == ctx.puuid {
                continue;
            }
            let (seen, wins, losses) = encounters.prior(&row.puuid);
            row.encounters = seen;
            row.encounter_wins = wins;
            row.encounter_losses = losses;
        }
        // Record the encounter and restart the combat pass only when a new match
        // begins, not on a rebuild that just added the late enemy half.
        if new_match {
            if state == MatchState::CoreGame {
                if let Some(mid) = match_id.as_deref() {
                    let now = unix_secs();
                    let roster: Vec<(String, String, u32)> = fetched
                        .iter()
                        .filter(|r| r.puuid != ctx.puuid)
                        .map(|r| (r.puuid.clone(), r.name.clone(), r.rank_tier))
                        .collect();
                    encounters.record_seen(mid, &roster, now);
                }
            }
            *combat_done = None;
            combat_attempted.clear();
            mates_map.clear();
        }
        *last_match_id = match_id;
        fetched
    };

    // Phase two: fill in K/D and headshot progressively, a few players per poll,
    // so the stats appear in batches and a counter can show progress. Each
    // player is attempted once per match. Ranks already show from phase one, so
    // the first batch is left for the next (fast) poll to keep ranks instant.
    let want_combat =
        fetch_combat && (state == MatchState::PreGame || state == MatchState::CoreGame);
    let mut combat_loading = false;
    if want_combat && !rows.is_empty() {
        let pending: Vec<String> = rows
            .iter()
            .filter(|r| !r.has_combat && !combat_attempted.contains(&r.puuid))
            .map(|r| r.puuid.clone())
            .collect();
        if pending.is_empty() {
            *combat_done = cur_match_id.clone();
        } else {
            combat_loading = true;
            if same_match {
                let chunk: Vec<String> = pending.into_iter().take(COMBAT_CHUNK).collect();
                enrich_combat(&ctx, region, &v, &mut rows, &chunk, mates_map, &party_map).await;
                for puuid in &chunk {
                    combat_attempted.insert(puuid.clone());
                }
            }
        }
    }

    if is_ffa(&queue_id) {
        for row in &mut rows {
            row.team.clear();
        }
    }
    let mut view = with_me(assemble_view(state, mode, rows, false));
    view.phase_time = phase_time;
    view.combat_loading = combat_loading;
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
    history_queue: Arc<AtomicU8>,
    wake: Arc<Notify>,
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
    let mut current_act: Option<String> = None;
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
    let mut combat_done: Option<String> = None;
    let mut combat_attempted: HashSet<String> = HashSet::new();
    let mut mates_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut last_history_queue = history_queue.load(Ordering::Relaxed);

    // Wake on presence changes from the local websocket (and on a mode-toggle
    // command), falling back to the poll interval below if neither fires.
    tauri::async_runtime::spawn(crate::websocket::run_presence_socket(wake.clone()));

    loop {
        let combat_on = combat_enabled.load(Ordering::Relaxed);
        let hq = history_queue.load(Ordering::Relaxed);
        let force_history = hq != last_history_queue;
        last_history_queue = hq;
        let view = match poll_once(
            &static_data,
            &region,
            &mut version,
            &mut current_act,
            &last,
            &mut last_match_id,
            &mut last_loop_state,
            &mut self_tick,
            &mut match_cache,
            &mut encounters,
            &mut combat_done,
            &mut combat_attempted,
            &mut mates_map,
            combat_on,
            hq,
            force_history,
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

        // While combat stats are still filling in, poll again almost at once so
        // the next batch lands quickly. In agent select, poll faster so hover and
        // lock changes (which the presence socket does not carry) stay live.
        // Otherwise wait the normal interval, reacting sooner to a websocket
        // presence change.
        if view.combat_loading {
            tokio::time::sleep(Duration::from_millis(250)).await;
        } else {
            let interval = match view.state {
                MatchState::PreGame => Duration::from_millis(1200),
                _ => Duration::from_secs(3),
            };
            tokio::select! {
                _ = tokio::time::sleep(interval) => {}
                _ = wake.notified() => {}
            }
        }
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
