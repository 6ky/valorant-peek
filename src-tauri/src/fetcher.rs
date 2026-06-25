use crate::auth::{pvp_headers, AuthContext};
use crate::client_version::Region;
use crate::http::pvp_client;
use crate::match_state::RawPlayer;
use crate::model::PlayerRow;
use crate::static_cache::StaticData;
use serde_json::Value;
use std::collections::HashMap;

/// Returns (current tier, ranked rating, peak tier).
pub fn parse_mmr(json: &Value) -> (u32, u32, u32) {
    let latest = json.get("LatestCompetitiveUpdate");
    let tier = latest
        .and_then(|l| l.get("TierAfterUpdate"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let rr = latest
        .and_then(|l| l.get("RankedRatingAfterUpdate"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let mut peak = tier;
    let seasons = json
        .get("QueueSkills")
        .and_then(|q| q.get("competitive"))
        .and_then(|c| c.get("SeasonalInfoBySeasonID"))
        .and_then(|s| s.as_object());
    if let Some(seasons) = seasons {
        for info in seasons.values() {
            if let Some(t) = info.get("CompetitiveTier").and_then(|v| v.as_u64()) {
                peak = peak.max(t as u32);
            }
        }
    }
    (tier, rr, peak)
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

pub async fn fetch_mmr(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    puuid: &str,
) -> (u32, u32, u32) {
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
    body.map(|v| parse_mmr(&v)).unwrap_or((0, 0, 0))
}

pub async fn build_rows(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    players: &[RawPlayer],
    sd: &StaticData,
) -> Vec<PlayerRow> {
    let puuids: Vec<String> = players.iter().map(|p| p.puuid.clone()).collect();
    let names = fetch_names(ctx, region, version, &puuids).await;

    let mut rows = Vec::with_capacity(players.len());
    for p in players {
        let (tier, rr, peak) = fetch_mmr(ctx, region, version, &p.puuid).await;
        rows.push(PlayerRow {
            puuid: p.puuid.clone(),
            name: names.get(&p.puuid).cloned().unwrap_or_default(),
            agent: sd.agent_name(&p.character_id),
            team: p.team.clone(),
            party_id: p.party_id.clone(),
            rank_tier: tier,
            rank_name: sd.rank_name(tier),
            rr,
            peak_rank_name: sd.rank_name(peak),
            account_level: p.account_level,
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mmr_current_and_peak() {
        let v: Value = serde_json::from_str(
            r#"{
              "LatestCompetitiveUpdate":{"TierAfterUpdate":18,"RankedRatingAfterUpdate":42},
              "QueueSkills":{"competitive":{"SeasonalInfoBySeasonID":{
                "s1":{"CompetitiveTier":18,"Rank":42},
                "s2":{"CompetitiveTier":24,"Rank":10}
              }}}}"#,
        )
        .unwrap();
        let (tier, rr, peak) = parse_mmr(&v);
        assert_eq!(tier, 18);
        assert_eq!(rr, 42);
        assert_eq!(peak, 24);
    }

    #[test]
    fn parses_mmr_handles_missing() {
        let v: Value = serde_json::from_str("{}").unwrap();
        assert_eq!(parse_mmr(&v), (0, 0, 0));
    }

    #[test]
    fn parses_names() {
        let v: Value =
            serde_json::from_str(r#"[{"Subject":"p1","GameName":"Ace","TagLine":"NA1"}]"#).unwrap();
        let m = parse_names(&v);
        assert_eq!(m.get("p1").unwrap(), "Ace#NA1");
    }
}
