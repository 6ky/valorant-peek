use crate::auth::fetch_auth;
use crate::client_version::{detect_region_from_log, fetch_client_version, Region};
use crate::fetcher::{build_rows, build_self};
use crate::lockfile::read_lockfile;
use crate::match_state::current_state;
use crate::model::{MatchState, MatchView, PlayerRow};
use crate::presence::{fetch_self_presence, is_ffa, mode_name};
use crate::static_cache::{load_or_fetch, StaticData};
use std::path::PathBuf;
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

async fn poll_once(
    sd: &StaticData,
    region: &Region,
    version: &mut Option<String>,
) -> Option<MatchView> {
    let lf = match read_lockfile() {
        Ok(lf) => lf,
        Err(_) => return Some(assemble_view(MatchState::NoGame, String::new(), Vec::new(), false)),
    };
    let ctx = fetch_auth(&lf).await.ok()?;
    if version.is_none() {
        *version = fetch_client_version().await.ok();
    }
    let v = version.clone()?;

    let me = build_self(&ctx, region, &v, sd).await;
    let with_me = |view: MatchView| MatchView {
        me: Some(me.clone()),
        ..view
    };

    let presence = fetch_self_presence(&lf, &ctx.puuid).await;
    let queue_id = presence
        .as_ref()
        .map(|p| p.queue_id.clone())
        .unwrap_or_default();
    let mode = mode_name(&queue_id);
    let loop_state = presence.as_ref().map(|p| p.loop_state.as_str()).unwrap_or("");

    // In menus there is no match to read, so skip the heavier glz probe.
    if loop_state == "MENUS" {
        return Some(with_me(assemble_view(MatchState::Menu, mode, Vec::new(), false)));
    }

    let (state, _match_id, raw) = current_state(&ctx, region, &v).await;
    if raw.is_empty() {
        return Some(with_me(assemble_view(state, mode, Vec::new(), false)));
    }
    let mut rows = build_rows(&ctx, region, &v, &raw, sd).await;
    if is_ffa(&queue_id) {
        for row in &mut rows {
            row.team.clear();
        }
    }
    Some(with_me(assemble_view(state, mode, rows, false)))
}

pub async fn run_loop(app: AppHandle) {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("static");
    let static_data = load_or_fetch(&cache_dir).await;
    let region = resolve_region();
    let mut version = fetch_client_version().await.ok();
    let mut last = assemble_view(MatchState::NoGame, String::new(), Vec::new(), false);

    loop {
        match poll_once(&static_data, &region, &mut version).await {
            Some(view) => {
                last = view.clone();
                let _ = app.emit("match-view", &view);
            }
            None => {
                // Only surface a stale badge if we were actually showing a
                // populated table. Otherwise this is just idle time with no
                // game ready, which should read as "waiting".
                if last.players.is_empty() {
                    let idle = assemble_view(MatchState::NoGame, String::new(), Vec::new(), false);
                    let _ = app.emit("match-view", &idle);
                } else {
                    let mut stale = last.clone();
                    stale.stale = true;
                    let _ = app.emit("match-view", &stale);
                }
            }
        }
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
