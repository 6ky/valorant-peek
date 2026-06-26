use crate::auth::{pvp_headers, AuthContext};
use crate::client_version::Region;
use crate::http::pvp_client;
use crate::model::MatchState;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPlayer {
    pub puuid: String,
    pub team: String,
    pub character_id: String,
    pub party_id: String,
    pub account_level: u32,
    pub incognito: bool,
    pub hide_level: bool,
    pub player_card_id: String,
    // Agent select only: true once the player has locked their agent. Absent
    // in coregame, where it defaults to false.
    pub locked: bool,
}

pub fn parse_match_players(json: &Value) -> Vec<RawPlayer> {
    let arr = match json.get("Players").and_then(|p| p.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };
    arr.iter()
        .map(|p| {
            let s = |k: &str| p.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let identity = p.get("PlayerIdentity");
            let id_u64 = |k: &str| {
                identity
                    .and_then(|id| id.get(k))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
            };
            let id_bool = |k: &str| {
                identity
                    .and_then(|id| id.get(k))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            };
            let id_str = |k: &str| {
                identity
                    .and_then(|id| id.get(k))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            };
            RawPlayer {
                puuid: s("Subject"),
                team: s("TeamID"),
                character_id: s("CharacterID"),
                party_id: s("PartyID"),
                account_level: id_u64("AccountLevel") as u32,
                incognito: id_bool("Incognito"),
                hide_level: id_bool("HideAccountLevel"),
                player_card_id: id_str("PlayerCardID"),
                locked: s("CharacterSelectionState") == "locked",
            }
        })
        .collect()
}

async fn match_id(client: &reqwest::Client, url: &str, headers: &reqwest::header::HeaderMap) -> Option<String> {
    let resp = client.get(url).headers(headers.clone()).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: Value = resp.json().await.ok()?;
    v.get("MatchID").and_then(|m| m.as_str()).map(String::from)
}

async fn fetch_doc(client: &reqwest::Client, url: &str, headers: &reqwest::header::HeaderMap) -> Option<Value> {
    let resp = client.get(url).headers(headers.clone()).send().await.ok()?;
    resp.json().await.ok()
}

/// Snapshot of the local player's current match phase.
pub struct CurrentState {
    pub state: MatchState,
    pub match_id: Option<String>,
    pub players: Vec<RawPlayer>,
    // Seconds left in the agent select countdown, 0 outside of pregame.
    pub phase_time: u32,
}

pub async fn current_state(ctx: &AuthContext, region: &Region, version: &str) -> CurrentState {
    let client = pvp_client();
    let headers = pvp_headers(ctx, version);

    let cg_player = format!("{}/core-game/v1/players/{}", region.glz_base(), ctx.puuid);
    if let Some(mid) = match_id(&client, &cg_player, &headers).await {
        let murl = format!("{}/core-game/v1/matches/{}", region.glz_base(), mid);
        let players = fetch_doc(&client, &murl, &headers)
            .await
            .map(|v| parse_match_players(&v))
            .unwrap_or_default();
        return CurrentState {
            state: MatchState::CoreGame,
            match_id: Some(mid),
            players,
            phase_time: 0,
        };
    }

    let pg_player = format!("{}/pregame/v1/players/{}", region.glz_base(), ctx.puuid);
    if let Some(mid) = match_id(&client, &pg_player, &headers).await {
        let murl = format!("{}/pregame/v1/matches/{}", region.glz_base(), mid);
        let doc = fetch_doc(&client, &murl, &headers).await;
        let players = doc
            .as_ref()
            .and_then(|v| v.get("AllyTeam"))
            .map(parse_match_players)
            .unwrap_or_default();
        let phase_time = doc
            .as_ref()
            .and_then(|v| v.get("PhaseTimeRemainingNS"))
            .and_then(|v| v.as_u64())
            .map(|ns| (ns / 1_000_000_000) as u32)
            .unwrap_or(0);
        return CurrentState {
            state: MatchState::PreGame,
            match_id: Some(mid),
            players,
            phase_time,
        };
    }

    CurrentState {
        state: MatchState::Menu,
        match_id: None,
        players: Vec::new(),
        phase_time: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_coregame_players() {
        let v: Value = serde_json::from_str(
            r#"{"Players":[
                {"Subject":"p1","TeamID":"Blue","CharacterID":"add6443a-41bd-e414-f6ad-e58d267f4e95","PartyID":"party-a","PlayerIdentity":{"AccountLevel":120}},
                {"Subject":"p2","TeamID":"Red","CharacterID":""}
            ]}"#,
        )
        .unwrap();
        let players = parse_match_players(&v);
        assert_eq!(players.len(), 2);
        assert_eq!(players[0].puuid, "p1");
        assert_eq!(players[0].team, "Blue");
        assert_eq!(players[0].party_id, "party-a");
        assert_eq!(players[0].account_level, 120);
        assert_eq!(players[1].team, "Red");
        assert_eq!(players[1].party_id, "");
        assert_eq!(players[1].account_level, 0);
        assert!(!players[0].locked);
    }

    #[test]
    fn reads_pregame_lock_state() {
        let v: Value = serde_json::from_str(
            r#"{"Players":[
                {"Subject":"p1","CharacterID":"x","CharacterSelectionState":"locked"},
                {"Subject":"p2","CharacterID":"","CharacterSelectionState":"selected"},
                {"Subject":"p3"}
            ]}"#,
        )
        .unwrap();
        let players = parse_match_players(&v);
        assert!(players[0].locked);
        assert!(!players[1].locked);
        assert!(!players[2].locked);
    }

    #[test]
    fn empty_when_no_players_key() {
        let v: Value = serde_json::from_str(r#"{"foo":1}"#).unwrap();
        assert!(parse_match_players(&v).is_empty());
    }
}
