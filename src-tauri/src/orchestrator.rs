use crate::auth::fetch_auth;
use crate::client_version::{detect_region_from_log, fetch_client_version, Region};
use crate::discord::{resolve_app_id, Rpc};
use crate::fetcher::{build_rows, build_self, fetch_history, refresh_rows, MatchStats};
use std::collections::HashMap;
use crate::lockfile::read_lockfile;
use crate::match_state::current_state;
use crate::model::{MatchState, MatchView, PlayerRow};
use crate::presence::{fetch_self_presence, is_ffa, mode_name};
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
        players: rows,
        me: None,
        history: Vec::new(),
        stale,
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

async fn poll_once(
    sd: &StaticData,
    region: &Region,
    version: &mut Option<String>,
    last: &MatchView,
    last_match_id: &mut Option<String>,
    last_loop_state: &mut String,
    self_tick: &mut u32,
    match_cache: &mut HashMap<String, MatchStats>,
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
    let with_me = |view: MatchView| MatchView {
        me: me.clone(),
        history: history.clone(),
        ..view
    };

    let presence = fetch_self_presence(&lf, &ctx.puuid).await;
    let queue_id = presence
        .as_ref()
        .map(|p| p.queue_id.clone())
        .unwrap_or_default();
    let mode = mode_name(&queue_id);
    let loop_state = presence.as_ref().map(|p| p.loop_state.as_str()).unwrap_or("");

    // Detect the game state transition from the local presence (a free call).
    // Riot's match servers are only touched when we are actually in a pregame
    // or game, and in-game only once per match, like vry does it.
    let in_match = loop_state == "PREGAME" || loop_state == "INGAME";
    let was_ingame = last_loop_state.as_str() == "INGAME";
    *last_loop_state = loop_state.to_string();

    if !in_match {
        *last_match_id = None;
        return Some(with_me(assemble_view(MatchState::Menu, mode, Vec::new(), false)));
    }

    // The in-game roster is fixed, so fetch it once on entry and then reuse it.
    // Pregame is short and re-fetched so agent locks appear as they happen.
    let ingame_steady = loop_state == "INGAME" && was_ingame && !last.players.is_empty();
    if ingame_steady {
        return Some(with_me(assemble_view(
            MatchState::CoreGame,
            mode,
            last.players.clone(),
            false,
        )));
    }

    let (state, match_id, raw) = current_state(&ctx, region, &v).await;
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
        let fetched = build_rows(&ctx, region, &v, &raw, sd, &last.players).await;
        *last_match_id = match_id;
        fetched
    };
    if is_ffa(&queue_id) {
        for row in &mut rows {
            row.team.clear();
        }
    }
    Some(with_me(assemble_view(state, mode, rows, false)))
}

pub async fn run_loop(app: AppHandle, rpc_enabled: Arc<AtomicBool>) {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("static");
    let static_data = load_or_fetch(&cache_dir).await;
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
