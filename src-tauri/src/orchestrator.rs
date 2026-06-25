use crate::auth::fetch_auth;
use crate::client_version::{fetch_client_version, Region};
use crate::fetcher::build_rows;
use crate::lockfile::read_lockfile;
use crate::match_state::current_state;
use crate::model::{MatchState, MatchView, PlayerRow};
use crate::static_cache::{load_or_fetch, StaticData};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub fn assemble_view(state: MatchState, rows: Vec<PlayerRow>, stale: bool) -> MatchView {
    MatchView {
        state,
        players: rows,
        stale,
    }
}

fn region_from_env() -> Region {
    let region = std::env::var("VAL_REGION").unwrap_or_else(|_| "na".to_string());
    let shard = std::env::var("VAL_SHARD").unwrap_or_else(|_| region.clone());
    Region { region, shard }
}

async fn poll_once(
    sd: &StaticData,
    region: &Region,
    version: &mut Option<String>,
) -> Option<MatchView> {
    let lf = match read_lockfile() {
        Ok(lf) => lf,
        Err(_) => return Some(assemble_view(MatchState::NoGame, Vec::new(), false)),
    };
    let ctx = fetch_auth(&lf).await.ok()?;
    if version.is_none() {
        *version = fetch_client_version().await.ok();
    }
    let v = version.clone()?;

    let (state, _match_id, raw) = current_state(&ctx, region, &v).await;
    if raw.is_empty() {
        return Some(assemble_view(state, Vec::new(), false));
    }
    let rows = build_rows(&ctx, region, &v, &raw, sd).await;
    Some(assemble_view(state, rows, false))
}

pub async fn run_loop(app: AppHandle) {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("static");
    let static_data = load_or_fetch(&cache_dir).await;
    let region = region_from_env();
    let mut version = fetch_client_version().await.ok();
    let mut last = assemble_view(MatchState::NoGame, Vec::new(), false);

    loop {
        match poll_once(&static_data, &region, &mut version).await {
            Some(view) => {
                last = view.clone();
                let _ = app.emit("match-view", &view);
            }
            None => {
                let mut stale = last.clone();
                stale.stale = true;
                let _ = app.emit("match-view", &stale);
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
        let v = assemble_view(MatchState::CoreGame, Vec::new(), false);
        assert_eq!(v.state, MatchState::CoreGame);
        assert!(!v.stale);
        assert!(v.players.is_empty());
    }
}
