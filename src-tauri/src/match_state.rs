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
}

pub fn parse_match_players(json: &Value) -> Vec<RawPlayer> {
    let arr = match json.get("Players").and_then(|p| p.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };
    arr.iter()
        .map(|p| {
            let s = |k: &str| p.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let account_level = p
                .get("PlayerIdentity")
                .and_then(|id| id.get("AccountLevel"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            RawPlayer {
                puuid: s("Subject"),
                team: s("TeamID"),
                character_id: s("CharacterID"),
                party_id: s("PartyID"),
                account_level,
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

async fn players_for(client: &reqwest::Client, url: &str, headers: &reqwest::header::HeaderMap, key: Option<&str>) -> Vec<RawPlayer> {
    let body: Option<Value> = async {
        let resp = client.get(url).headers(headers.clone()).send().await.ok()?;
        resp.json().await.ok()
    }
    .await;
    match (body, key) {
        (Some(v), Some(k)) => v.get(k).map(parse_match_players).unwrap_or_default(),
        (Some(v), None) => parse_match_players(&v),
        _ => Vec::new(),
    }
}

pub async fn current_state(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
) -> (MatchState, Option<String>, Vec<RawPlayer>) {
    let client = pvp_client();
    let headers = pvp_headers(ctx, version);

    let cg_player = format!("{}/core-game/v1/players/{}", region.glz_base(), ctx.puuid);
    if let Some(mid) = match_id(&client, &cg_player, &headers).await {
        let murl = format!("{}/core-game/v1/matches/{}", region.glz_base(), mid);
        let players = players_for(&client, &murl, &headers, None).await;
        return (MatchState::CoreGame, Some(mid), players);
    }

    let pg_player = format!("{}/pregame/v1/players/{}", region.glz_base(), ctx.puuid);
    if let Some(mid) = match_id(&client, &pg_player, &headers).await {
        let murl = format!("{}/pregame/v1/matches/{}", region.glz_base(), mid);
        let players = players_for(&client, &murl, &headers, Some("AllyTeam")).await;
        return (MatchState::PreGame, Some(mid), players);
    }

    (MatchState::Menu, None, Vec::new())
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
    }

    #[test]
    fn empty_when_no_players_key() {
        let v: Value = serde_json::from_str(r#"{"foo":1}"#).unwrap();
        assert!(parse_match_players(&v).is_empty());
    }
}
