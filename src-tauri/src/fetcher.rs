use crate::auth::{pvp_headers, AuthContext};
use crate::client_version::Region;
use crate::http::pvp_client;
use crate::match_state::RawPlayer;
use crate::model::{HistoryEntry, PlayerRow};
use crate::static_cache::StaticData;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Mmr {
    pub tier: u32,
    pub rr: u32,
    pub peak: u32,
    pub wins: u32,
    pub games: u32,
    pub leaderboard: u32,
}

pub fn parse_mmr(json: &Value) -> Mmr {
    let latest = json.get("LatestCompetitiveUpdate");
    let tier = latest
        .and_then(|l| l.get("TierAfterUpdate"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let rr = latest
        .and_then(|l| l.get("RankedRatingAfterUpdate"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let season_id = latest
        .and_then(|l| l.get("SeasonID"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut peak = tier;
    let mut wins = 0;
    let mut games = 0;
    let mut leaderboard = 0;
    let seasons = json
        .get("QueueSkills")
        .and_then(|q| q.get("competitive"))
        .and_then(|c| c.get("SeasonalInfoBySeasonID"))
        .and_then(|s| s.as_object());
    if let Some(seasons) = seasons {
        for (id, info) in seasons {
            // Peak comes from the tiers actually achieved (WinsByTier keys),
            // not the season-end tier, which can be lower than the peak.
            if let Some(wins_by_tier) = info.get("WinsByTier").and_then(|w| w.as_object()) {
                for key in wins_by_tier.keys() {
                    if let Ok(t) = key.parse::<u32>() {
                        peak = peak.max(t);
                    }
                }
            }
            if let Some(t) = info.get("CompetitiveTier").and_then(|v| v.as_u64()) {
                peak = peak.max(t as u32);
            }
            if id == season_id {
                wins = info
                    .get("NumberOfWinsWithPlacements")
                    .or_else(|| info.get("NumberOfWins"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                games = info.get("NumberOfGames").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                leaderboard = info
                    .get("LeaderboardRank")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
            }
        }
    }
    Mmr {
        tier,
        rr,
        peak,
        wins,
        games,
        leaderboard,
    }
}

pub fn win_rate(mmr: &Mmr) -> u32 {
    if mmr.games > 0 {
        mmr.wins * 100 / mmr.games
    } else {
        0
    }
}

pub fn parse_names(json: &Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if let Some(arr) = json.as_array() {
        for entry in arr {
            let puuid = entry.get("Subject").and_then(|v| v.as_str());
            let name = entry.get("GameName").and_then(|v| v.as_str());
            let tag = entry.get("TagLine").and_then(|v| v.as_str());
            if let (Some(puuid), Some(name), Some(tag)) = (puuid, name, tag) {
                out.insert(puuid.to_string(), format!("{name}#{tag}"));
            }
        }
    }
    out
}

pub async fn fetch_names(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    puuids: &[String],
) -> HashMap<String, String> {
    let url = format!("{}/name-service/v2/players", region.pd_base());
    let body: Option<Value> = async {
        pvp_client()
            .put(&url)
            .headers(pvp_headers(ctx, version))
            .json(puuids)
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()
    }
    .await;
    body.map(|v| parse_names(&v)).unwrap_or_default()
}

/// Returns None when the request itself failed (so callers can keep the last
/// known value instead of showing blank/unranked data).
pub async fn fetch_mmr(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    puuid: &str,
) -> Option<Mmr> {
    let url = format!("{}/mmr/v1/players/{}", region.pd_base(), puuid);
    let body: Option<Value> = async {
        pvp_client()
            .get(&url)
            .headers(pvp_headers(ctx, version))
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()
    }
    .await;
    body.map(|v| parse_mmr(&v))
}

pub fn parse_account_level(json: &Value) -> u32 {
    json.get("Progress")
        .and_then(|p| p.get("Level"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32
}

pub async fn fetch_account_level(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    puuid: &str,
) -> u32 {
    let url = format!("{}/account-xp/v1/players/{}", region.pd_base(), puuid);
    let body: Option<Value> = async {
        pvp_client()
            .get(&url)
            .headers(pvp_headers(ctx, version))
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()
    }
    .await;
    body.map(|v| parse_account_level(&v)).unwrap_or(0)
}

/// The signed-in user's equipped player card id, from their loadout.
pub async fn fetch_loadout_card(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    puuid: &str,
) -> String {
    let url = format!(
        "{}/personalization/v2/players/{}/playerloadout",
        region.pd_base(),
        puuid
    );
    let body: Option<Value> = async {
        pvp_client()
            .get(&url)
            .headers(pvp_headers(ctx, version))
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()
    }
    .await;
    body.and_then(|v| {
        v.get("Identity")
            .and_then(|i| i.get("PlayerCardID"))
            .and_then(|c| c.as_str())
            .map(String::from)
    })
    .unwrap_or_default()
}

pub fn parse_history(json: &Value, sd: &StaticData) -> Vec<HistoryEntry> {
    let matches = match json.get("Matches").and_then(|m| m.as_array()) {
        Some(arr) => arr,
        None => return Vec::new(),
    };
    matches
        .iter()
        .map(|m| {
            let tier = m
                .get("TierAfterUpdate")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            HistoryEntry {
                map: sd.map_name(m.get("MapID").and_then(|v| v.as_str()).unwrap_or("")),
                rr_change: m
                    .get("RankedRatingEarned")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32,
                tier,
                rank_name: sd.rank_name(tier),
            }
        })
        .collect()
}

pub async fn fetch_history(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    sd: &StaticData,
) -> Vec<HistoryEntry> {
    let url = format!(
        "{}/mmr/v1/players/{}/competitiveupdates?startIndex=0&endIndex=10&queue=competitive",
        region.pd_base(),
        ctx.puuid
    );
    let body: Option<Value> = async {
        pvp_client()
            .get(&url)
            .headers(pvp_headers(ctx, version))
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()
    }
    .await;
    body.map(|v| parse_history(&v, sd)).unwrap_or_default()
}

/// Build a row for the signed-in user, for display when not in a match.
pub async fn build_self(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    sd: &StaticData,
) -> Option<PlayerRow> {
    // If the rank request fails, return None so the caller keeps the last
    // known profile instead of flashing unranked.
    let mmr = fetch_mmr(ctx, region, version, &ctx.puuid).await?;
    let puuids = [ctx.puuid.clone()];
    let names = fetch_names(ctx, region, version, &puuids).await;
    let level = fetch_account_level(ctx, region, version, &ctx.puuid).await;
    let card_id = fetch_loadout_card(ctx, region, version, &ctx.puuid).await;
    Some(PlayerRow {
        puuid: ctx.puuid.clone(),
        name: names.get(&ctx.puuid).cloned().unwrap_or_default(),
        player_card: sd.card_art(&card_id),
        agent: String::new(),
        agent_icon: String::new(),
        team: String::new(),
        party_id: String::new(),
        hidden_name: false,
        rank_tier: mmr.tier,
        rank_name: sd.rank_name(mmr.tier),
        rank_icon: sd.rank_icon(mmr.tier),
        rr: mmr.rr,
        peak_rank_name: sd.rank_name(mmr.peak),
        peak_rank_tier: mmr.peak,
        win_rate: win_rate(&mmr),
        wins: mmr.wins,
        games: mmr.games,
        leaderboard: mmr.leaderboard,
        account_level: level,
    })
}

pub async fn build_rows(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    players: &[RawPlayer],
    sd: &StaticData,
    last_rows: &[PlayerRow],
) -> Vec<PlayerRow> {
    let puuids: Vec<String> = players.iter().map(|p| p.puuid.clone()).collect();
    let names = fetch_names(ctx, region, version, &puuids).await;

    let self_team = players
        .iter()
        .find(|p| p.puuid == ctx.puuid)
        .map(|p| p.team.clone());

    let mut rows = Vec::with_capacity(players.len());
    for p in players {
        let fetched = fetch_mmr(ctx, region, version, &p.puuid).await;
        let rank_failed = fetched.is_none();
        let mmr = fetched.unwrap_or_default();
        let team = match (&self_team, p.team.is_empty()) {
            (_, true) => String::new(),
            (Some(mine), _) if &p.team == mine => "Ally".to_string(),
            (Some(_), _) => "Enemy".to_string(),
            (None, _) => p.team.clone(),
        };
        let is_self = p.puuid == ctx.puuid;
        let hidden_name = p.incognito && !is_self;
        let name = if hidden_name {
            String::new()
        } else {
            names.get(&p.puuid).cloned().unwrap_or_default()
        };
        let account_level = if p.hide_level && !is_self {
            0
        } else {
            p.account_level
        };
        let mut row = PlayerRow {
            puuid: p.puuid.clone(),
            name,
            player_card: sd.card_art(&p.player_card_id),
            agent: sd.agent_name(&p.character_id),
            agent_icon: sd.agent_icon(&p.character_id),
            team,
            party_id: p.party_id.clone(),
            hidden_name,
            rank_tier: mmr.tier,
            rank_name: sd.rank_name(mmr.tier),
            rank_icon: sd.rank_icon(mmr.tier),
            rr: mmr.rr,
            peak_rank_name: sd.rank_name(mmr.peak),
            peak_rank_tier: mmr.peak,
            win_rate: win_rate(&mmr),
            wins: mmr.wins,
            games: mmr.games,
            leaderboard: mmr.leaderboard,
            account_level,
        };
        // On a failed rank request, keep the player's last known rank data.
        if rank_failed {
            if let Some(prev) = last_rows.iter().find(|r| r.puuid == p.puuid) {
                row.rank_tier = prev.rank_tier;
                row.rank_name = prev.rank_name.clone();
                row.rank_icon = prev.rank_icon.clone();
                row.rr = prev.rr;
                row.peak_rank_name = prev.peak_rank_name.clone();
                row.peak_rank_tier = prev.peak_rank_tier;
                row.win_rate = prev.win_rate;
                row.wins = prev.wins;
                row.games = prev.games;
                row.leaderboard = prev.leaderboard;
            }
        }
        rows.push(row);
    }
    rows
}

/// Rebuild rows for the current match from cached rank data, refreshing only
/// the cheap fields (agent, team, level) that come from the match payload.
/// Avoids re-fetching ranks every poll while still updating agent picks.
pub fn refresh_rows(
    cached: &[PlayerRow],
    raw: &[RawPlayer],
    sd: &StaticData,
    self_puuid: &str,
) -> Vec<PlayerRow> {
    let self_team = raw
        .iter()
        .find(|p| p.puuid == self_puuid)
        .map(|p| p.team.clone());

    raw.iter()
        .map(|p| {
            let mut row = cached
                .iter()
                .find(|r| r.puuid == p.puuid)
                .cloned()
                .unwrap_or_else(|| PlayerRow {
                    puuid: p.puuid.clone(),
                    ..Default::default()
                });
            let is_self = p.puuid == self_puuid;
            row.team = match (&self_team, p.team.is_empty()) {
                (_, true) => String::new(),
                (Some(mine), _) if &p.team == mine => "Ally".to_string(),
                (Some(_), _) => "Enemy".to_string(),
                (None, _) => p.team.clone(),
            };
            row.agent = sd.agent_name(&p.character_id);
            row.agent_icon = sd.agent_icon(&p.character_id);
            row.player_card = sd.card_art(&p.player_card_id);
            if !(p.hide_level && !is_self) {
                row.account_level = p.account_level;
            }
            row
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mmr_current_and_peak() {
        // Peak must come from WinsByTier (tiers achieved), not the lower
        // season-end CompetitiveTier. Here the player ended at 23 but peaked 25.
        let v: Value = serde_json::from_str(
            r#"{
              "LatestCompetitiveUpdate":{"TierAfterUpdate":23,"RankedRatingAfterUpdate":42,"SeasonID":"s2"},
              "QueueSkills":{"competitive":{"SeasonalInfoBySeasonID":{
                "s1":{"CompetitiveTier":18,"WinsByTier":{"17":3,"18":5}},
                "s2":{"CompetitiveTier":23,"WinsByTier":{"24":4,"25":2},"NumberOfWins":7,"NumberOfWinsWithPlacements":10,"NumberOfGames":15,"LeaderboardRank":0}
              }}}}"#,
        )
        .unwrap();
        let m = parse_mmr(&v);
        assert_eq!(m.tier, 23);
        assert_eq!(m.rr, 42);
        assert_eq!(m.peak, 25);
        // wins must include placement wins (10), not the lower NumberOfWins (7)
        assert_eq!(m.wins, 10);
        assert_eq!(m.games, 15);
        assert_eq!(win_rate(&m), 66);
    }

    #[test]
    fn parses_mmr_handles_missing() {
        let v: Value = serde_json::from_str("{}").unwrap();
        assert_eq!(parse_mmr(&v), Mmr::default());
    }

    #[test]
    fn parses_history_rr_changes() {
        use std::collections::HashMap;
        let sd = StaticData {
            tiers: HashMap::from([(18u32, "Diamond 1".to_string())]),
            maps: HashMap::from([("/Game/Maps/Bonsai/Bonsai".to_string(), "Split".to_string())]),
            ..Default::default()
        };
        let v: Value = serde_json::from_str(
            r#"{"Matches":[
                {"MapID":"/Game/Maps/Bonsai/Bonsai","RankedRatingEarned":18,"TierAfterUpdate":18},
                {"MapID":"/Game/Maps/Bonsai/Bonsai","RankedRatingEarned":-21,"TierAfterUpdate":18}
            ]}"#,
        )
        .unwrap();
        let hist = parse_history(&v, &sd);
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0].rr_change, 18);
        assert_eq!(hist[0].map, "Split");
        assert_eq!(hist[0].rank_name, "Diamond 1");
        assert_eq!(hist[1].rr_change, -21);
    }

    #[test]
    fn parses_account_level() {
        let v: Value = serde_json::from_str(r#"{"Progress":{"Level":237,"XP":1200}}"#).unwrap();
        assert_eq!(parse_account_level(&v), 237);
        assert_eq!(parse_account_level(&serde_json::json!({})), 0);
    }

    #[test]
    fn parses_names() {
        let v: Value =
            serde_json::from_str(r#"[{"Subject":"p1","GameName":"Ace","TagLine":"NA1"}]"#).unwrap();
        let m = parse_names(&v);
        assert_eq!(m.get("p1").unwrap(), "Ace#NA1");
    }
}
