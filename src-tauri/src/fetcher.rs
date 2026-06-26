use crate::auth::{pvp_headers, AuthContext};
use crate::client_version::Region;
use crate::http::pvp_client;
use crate::match_state::RawPlayer;
use crate::model::{HistoryEntry, PlayerRow, ScoreEntry};
use crate::static_cache::StaticData;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

// Authed GET that parses JSON, backing off and retrying on HTTP 429. Reads
// Retry-After (seconds) for the wait, defaulting to 5, plus a 1s buffer. Gives
// up after 3 retries. Transport errors and parse errors return None.
async fn get_json_retry(url: &str, ctx: &AuthContext, version: &str) -> Option<Value> {
    for _ in 0..3 {
        let resp = pvp_client()
            .get(url)
            .headers(pvp_headers(ctx, version))
            .send()
            .await
            .ok()?;
        if resp.status().as_u16() == 429 {
            let secs = resp
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(5);
            tokio::time::sleep(Duration::from_secs(secs + 1)).await;
            continue;
        }
        return resp.json().await.ok();
    }
    None
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Mmr {
    pub tier: u32,
    pub rr: u32,
    pub peak: u32,
    pub peak_season: String,
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
    let mut peak_season = season_id.to_string();
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
            let mut season_peak = 0;
            if let Some(wins_by_tier) = info.get("WinsByTier").and_then(|w| w.as_object()) {
                for key in wins_by_tier.keys() {
                    if let Ok(t) = key.parse::<u32>() {
                        season_peak = season_peak.max(t);
                    }
                }
            }
            if let Some(t) = info.get("CompetitiveTier").and_then(|v| v.as_u64()) {
                season_peak = season_peak.max(t as u32);
            }
            if season_peak > peak {
                peak = season_peak;
                peak_season = id.clone();
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
        peak_season,
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
    let body = get_json_retry(&url, ctx, version).await;
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
            let map_id = m.get("MapID").and_then(|v| v.as_str()).unwrap_or("");
            HistoryEntry {
                map: sd.map_name(map_id),
                map_image: sd.map_image(map_id),
                rr_change: m
                    .get("RankedRatingEarned")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32,
                tier,
                rank_name: sd.rank_name(tier),
                ..Default::default()
            }
        })
        .collect()
}

/// Summed (head, body, leg) shots for a player across every round's damage.
pub fn shot_counts(detail: &Value, puuid: &str) -> (u64, u64, u64) {
    let (mut head, mut body, mut leg) = (0u64, 0u64, 0u64);
    if let Some(rounds) = detail.get("roundResults").and_then(|r| r.as_array()) {
        for round in rounds {
            if let Some(stats) = round.get("playerStats").and_then(|p| p.as_array()) {
                for ps in stats {
                    if ps.get("subject").and_then(|v| v.as_str()) != Some(puuid) {
                        continue;
                    }
                    if let Some(damage) = ps.get("damage").and_then(|d| d.as_array()) {
                        for d in damage {
                            head += d.get("headshots").and_then(|v| v.as_u64()).unwrap_or(0);
                            body += d.get("bodyshots").and_then(|v| v.as_u64()).unwrap_or(0);
                            leg += d.get("legshots").and_then(|v| v.as_u64()).unwrap_or(0);
                        }
                    }
                }
            }
        }
    }
    (head, body, leg)
}

/// Headshot percentage for a player, summed over every round's shot damage,
/// the same way vry computes it.
pub fn headshot_pct(detail: &Value, puuid: &str) -> u32 {
    let (head, body, leg) = shot_counts(detail, puuid);
    let total = head + body + leg;
    if total > 0 {
        (head * 100 / total) as u32
    } else {
        0
    }
}

#[derive(Clone, Default)]
pub struct MatchStats {
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub acs: u32,
    pub hs: u32,
    pub self_rounds: u32,
    pub enemy_rounds: u32,
    pub won: bool,
    pub agent_icon: String,
    pub agent_name: String,
    pub scoreboard: Vec<ScoreEntry>,
}

pub fn parse_match_stats(detail: &Value, puuid: &str, sd: &StaticData) -> MatchStats {
    let players = detail.get("players").and_then(|p| p.as_array());
    let me = match players.and_then(|arr| {
        arr.iter()
            .find(|p| p.get("subject").and_then(|v| v.as_str()) == Some(puuid))
    }) {
        Some(m) => m,
        None => return MatchStats::default(),
    };
    let character_id = me.get("characterId").and_then(|v| v.as_str()).unwrap_or("");
    let team_id = me.get("teamId").and_then(|v| v.as_str()).unwrap_or("");
    let stat = |k: &str| {
        me.get("stats")
            .and_then(|s| s.get(k))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32
    };
    let rounds = stat("roundsPlayed").max(1);
    let acs = stat("score") / rounds;

    let mut won = false;
    let mut self_rounds = 0;
    let mut enemy_rounds = 0;
    if let Some(teams) = detail.get("teams").and_then(|t| t.as_array()) {
        for t in teams {
            let r = t.get("roundsWon").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            if t.get("teamId").and_then(|v| v.as_str()) == Some(team_id) {
                self_rounds = r;
                won = t.get("won").and_then(|v| v.as_bool()).unwrap_or(false);
            } else {
                enemy_rounds = enemy_rounds.max(r);
            }
        }
    }

    // Full scoreboard for every player, from the same response.
    let mut scoreboard = Vec::new();
    if let Some(arr) = players {
        for p in arr {
            let subject = p.get("subject").and_then(|v| v.as_str()).unwrap_or("");
            let team = p.get("teamId").and_then(|v| v.as_str()).unwrap_or("");
            let agent = p.get("characterId").and_then(|v| v.as_str()).unwrap_or("");
            let g = |k: &str| {
                p.get("stats")
                    .and_then(|s| s.get(k))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32
            };
            let rp = g("roundsPlayed").max(1);
            let game_name = p.get("gameName").and_then(|v| v.as_str()).unwrap_or("");
            let tag = p.get("tagLine").and_then(|v| v.as_str()).unwrap_or("");
            scoreboard.push(ScoreEntry {
                name: if game_name.is_empty() {
                    String::new()
                } else {
                    format!("{game_name}#{tag}")
                },
                agent_icon: sd.agent_icon(agent),
                kills: g("kills"),
                deaths: g("deaths"),
                assists: g("assists"),
                acs: g("score") / rp,
                hs: headshot_pct(detail, subject),
                ally: team == team_id,
                is_self: subject == puuid,
            });
        }
        scoreboard.sort_by(|a, b| b.ally.cmp(&a.ally).then(b.acs.cmp(&a.acs)));
    }

    MatchStats {
        kills: stat("kills"),
        deaths: stat("deaths"),
        assists: stat("assists"),
        acs,
        hs: headshot_pct(detail, puuid),
        self_rounds,
        enemy_rounds,
        won,
        agent_icon: sd.agent_icon(character_id),
        agent_name: sd.agent_name(character_id),
        scoreboard,
    }
}

async fn fetch_match_detail(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    match_id: &str,
) -> Option<Value> {
    let url = format!("{}/match-details/v1/matches/{}", region.pd_base(), match_id);
    get_json_retry(&url, ctx, version).await
}

/// Signed run of recent results and the RR sum over them. Walks Matches from
/// the most recent: streak is the count of leading games sharing the first
/// non-zero RR sign (positive for a win run, negative for a loss run, 0 if the
/// latest game broke even). rr_trend sums RankedRatingEarned over all matches.
fn streak_and_trend(matches: &[Value]) -> (i32, i32) {
    let earned = |m: &Value| m.get("RankedRatingEarned").and_then(|v| v.as_i64()).unwrap_or(0);

    let rr_trend: i64 = matches.iter().map(earned).sum();

    let mut streak = 0i32;
    let mut sign = 0i32;
    for m in matches {
        let s = match earned(m) {
            x if x > 0 => 1,
            x if x < 0 => -1,
            _ => 0,
        };
        if sign == 0 {
            if s == 0 {
                break;
            }
            sign = s;
            streak = s;
        } else if s == sign {
            streak += sign;
        } else {
            break;
        }
    }
    (streak, rr_trend as i32)
}

// Recent win/loss record over the full Matches list, by RR sign.
fn win_loss(matches: &[Value]) -> (u32, u32) {
    let (mut wins, mut losses) = (0u32, 0u32);
    for m in matches {
        match m.get("RankedRatingEarned").and_then(|v| v.as_i64()).unwrap_or(0) {
            x if x > 0 => wins += 1,
            x if x < 0 => losses += 1,
            _ => {}
        }
    }
    (wins, losses)
}

// Number of recent competitive matches to aggregate K/D and headshot% over.
const RECENT_GAMES: usize = 10;
// Match-detail request budget split across the roster, so a big lobby fetches
// fewer games per player and stays under the rate limit.
const RECENT_GAMES_BUDGET: usize = 50;
// Caps so a heavy roster load cannot burst into a rate limit: match-detail
// requests in flight per player, and players fetched at once across the roster.
// Product is the max requests in flight (3 x 2 = 6), kept low like vry, which
// fetches fully sequentially and leans on backoff.
const COMBAT_DETAIL_CONCURRENCY: usize = 2;
const COMBAT_PLAYER_CONCURRENCY: usize = 3;

/// Recent competitive form for a player: kills, deaths and headshot% aggregated
/// over their last `games` competitive matches, plus streak, RR trend and
/// win/loss record over the full recent Matches list. None when the history request
/// fails or there are no comp matches; match details that fail to load are
/// skipped and the rest are still aggregated.
pub async fn fetch_player_recent(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    puuid: &str,
    games: usize,
) -> Option<(u32, u32, u32, i32, i32, u32, u32)> {
    let url = format!(
        "{}/mmr/v1/players/{}/competitiveupdates?startIndex=0&endIndex=10&queue=competitive",
        region.pd_base(),
        puuid
    );
    let cu = get_json_retry(&url, ctx, version).await?;
    let matches = cu.get("Matches").and_then(|m| m.as_array())?;
    if matches.is_empty() {
        return None;
    }
    let (streak, rr_trend) = streak_and_trend(matches);
    let (recent_wins, recent_losses) = win_loss(matches);

    let ids: Vec<String> = matches
        .iter()
        .take(games)
        .filter_map(|m| m.get("MatchID").and_then(|v| v.as_str()).map(String::from))
        .collect();

    // Fetch the match details in small concurrent chunks rather than all at
    // once, so a heavy roster does not burst into a rate limit.
    let mut details: Vec<Option<Value>> = Vec::with_capacity(ids.len());
    for chunk in ids.chunks(COMBAT_DETAIL_CONCURRENCY) {
        let part = futures::future::join_all(
            chunk.iter().map(|id| fetch_match_detail(ctx, region, version, id)),
        )
        .await;
        details.extend(part);
    }

    let (mut total_kills, mut total_deaths) = (0u64, 0u64);
    let (mut head, mut body, mut leg) = (0u64, 0u64, 0u64);
    for detail in details.into_iter().flatten() {
        let me = detail
            .get("players")
            .and_then(|p| p.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|p| p.get("subject").and_then(|v| v.as_str()) == Some(puuid))
            });
        if let Some(me) = me {
            let stat = |k: &str| {
                me.get("stats")
                    .and_then(|s| s.get(k))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
            };
            total_kills += stat("kills");
            total_deaths += stat("deaths");
        }
        let (h, b, l) = shot_counts(&detail, puuid);
        head += h;
        body += b;
        leg += l;
    }

    let shots = head + body + leg;
    let hs_pct = if shots > 0 { (head * 100 / shots) as u32 } else { 0 };
    Some((
        total_kills as u32,
        total_deaths as u32,
        hs_pct,
        streak,
        rr_trend,
        recent_wins,
        recent_losses,
    ))
}

// Default melee weapon and its skin socket, used to read a player's equipped
// melee skin from the core-game loadouts.
const MELEE_WEAPON: &str = "2f59173c-4bed-b6c3-2191-dea9b58be9c7";
const MELEE_SKIN_SOCKET: &str = "bcef87d6-209b-46c6-8b19-fbe40bd95abc";

// Vandal weapon, sharing the same skin socket as the melee.
const VANDAL_WEAPON: &str = "9c82e19d-4575-0200-1a81-3eacf00cf872";

/// Skin level id equipped on a given weapon in a loadout entry.
fn weapon_skin_id<'a>(entry: &'a Value, weapon: &str) -> Option<&'a str> {
    loadout_items(entry)?
        .get(weapon)?
        .get("Sockets")?
        .get(MELEE_SKIN_SOCKET)?
        .get("Item")?
        .get("ID")?
        .as_str()
}

fn loadout_items(entry: &Value) -> Option<&Value> {
    entry
        .get("Items")
        .or_else(|| entry.get("Loadout").and_then(|l| l.get("Items")))
}

fn loadout_subject(entry: &Value) -> Option<&str> {
    entry
        .get("Subject")
        .or_else(|| entry.get("Loadout").and_then(|l| l.get("Subject")))
        .and_then(|v| v.as_str())
}

fn melee_skin_id(entry: &Value) -> Option<&str> {
    weapon_skin_id(entry, MELEE_WEAPON)
}

/// Equipped Vandal skin name, art, and tier color for a loadout entry. The
/// default skin (named "Vandal") and any missing lookup yield all-empty fields.
fn vandal_fields(entry: &Value, sd: &StaticData) -> (String, String, String) {
    let id = match weapon_skin_id(entry, VANDAL_WEAPON) {
        Some(id) if !id.is_empty() => id,
        _ => return Default::default(),
    };
    let info = match sd.skin_info(id) {
        Some(i) => i,
        None => return Default::default(),
    };
    if info.name == "Vandal" {
        return Default::default();
    }
    (
        info.name.clone(),
        info.image.clone(),
        sd.tier_color(&info.tier_uuid),
    )
}

/// True when the loadout entry runs a melee skin other than the stock one. The
/// equipped id is resolved through the skins map; the stock melee resolves to
/// the name "Melee", anything else is premium.
fn has_premium_melee(entry: &Value, sd: &StaticData) -> bool {
    let id = match melee_skin_id(entry) {
        Some(id) if !id.is_empty() => id,
        _ => return false,
    };
    match sd.skin_info(id) {
        Some(info) => !info.name.is_empty() && info.name != "Melee",
        None => false,
    }
}

/// Core-game loadouts for every player, aligned to the match's Players order.
async fn fetch_loadouts(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    match_id: &str,
) -> Option<Vec<Value>> {
    let url = format!(
        "{}/core-game/v1/matches/{}/loadouts",
        region.glz_base(),
        match_id
    );
    let body: Value = async {
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
    .await?;
    body.get("Loadouts").and_then(|l| l.as_array()).cloned()
}

// Smurf score thresholds, kept together so the heuristic stays tunable.
const SMURF_DIAMOND_TIER: u32 = 18;
const SMURF_ASCENDANT_TIER: u32 = 21;
const SMURF_LOW_LEVEL: u32 = 45;
const SMURF_FEW_GAMES: u32 = 40;
const SMURF_MIN_WR_GAMES: u32 = 5;
const SMURF_STRONG_WR: u32 = 60;
const SMURF_PREMIUM_LEVEL: u32 = 30;
const SMURF_MAX: u32 = 100;

/// Heuristic 0 to 100 read on how likely a ranked account is a smurf. Unranked
/// players score 0. A hidden account level (0) only contributes through the
/// winrate and games terms, never the level-based ones.
fn smurf_score(
    account_level: u32,
    rank_tier: u32,
    games: u32,
    win_rate: u32,
    premium_skins: bool,
) -> u32 {
    if rank_tier == 0 {
        return 0;
    }
    let mut score = 0u32;
    // High rank carried on a low account level.
    if rank_tier >= SMURF_DIAMOND_TIER && account_level > 0 && account_level < SMURF_LOW_LEVEL {
        score += (SMURF_LOW_LEVEL - account_level) * 2;
    }
    // Few games played to reach a high rank.
    if rank_tier >= SMURF_ASCENDANT_TIER && games > 0 && games <= SMURF_FEW_GAMES {
        score += SMURF_FEW_GAMES - games;
    }
    // Strong winrate with a meaningful sample.
    if games >= SMURF_MIN_WR_GAMES && win_rate >= SMURF_STRONG_WR {
        score += win_rate - SMURF_STRONG_WR;
    }
    // Premium melee on an otherwise fresh account.
    if premium_skins && account_level > 0 && account_level < SMURF_PREMIUM_LEVEL {
        score += 10;
    }
    score.min(SMURF_MAX)
}

pub async fn fetch_history(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    sd: &StaticData,
    cache: &mut HashMap<String, MatchStats>,
) -> Vec<HistoryEntry> {
    let url = format!(
        "{}/mmr/v1/players/{}/competitiveupdates?startIndex=0&endIndex=15&queue=competitive",
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
    let json = match body {
        Some(j) => j,
        None => return Vec::new(),
    };

    let mut entries = parse_history(&json, sd);
    let ids: Vec<String> = json
        .get("Matches")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .map(|m| m.get("MatchID").and_then(|v| v.as_str()).unwrap_or("").to_string())
                .collect()
        })
        .unwrap_or_default();

    for (entry, id) in entries.iter_mut().zip(ids) {
        if id.is_empty() {
            continue;
        }
        let stats = match cache.get(&id) {
            Some(s) => Some(s.clone()),
            None => match fetch_match_detail(ctx, region, version, &id).await {
                Some(detail) => {
                    let s = parse_match_stats(&detail, &ctx.puuid, sd);
                    cache.insert(id.clone(), s.clone());
                    Some(s)
                }
                None => None,
            },
        };
        if let Some(s) = stats {
            entry.agent_icon = s.agent_icon;
            entry.agent_name = s.agent_name;
            entry.kills = s.kills;
            entry.deaths = s.deaths;
            entry.assists = s.assists;
            entry.acs = s.acs;
            entry.hs = s.hs;
            entry.self_rounds = s.self_rounds;
            entry.enemy_rounds = s.enemy_rounds;
            entry.won = s.won;
            entry.scoreboard = s.scoreboard;
            entry.has_stats = true;
        }
    }
    entries
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
    let (last_kills, last_deaths, last_hs, streak, rr_trend, recent_wins, recent_losses, has_combat) =
        match fetch_player_recent(ctx, region, version, &ctx.puuid, RECENT_GAMES).await {
            Some((k, d, h, s, t, w, l)) => (k, d, h, s, t, w, l, true),
            None => (0, 0, 0, 0, 0, 0, 0, false),
        };
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
        peak_rank_icon: sd.rank_icon(mmr.peak),
        peak_act: sd.season_label(&mmr.peak_season),
        win_rate: win_rate(&mmr),
        wins: mmr.wins,
        games: mmr.games,
        leaderboard: mmr.leaderboard,
        account_level: level,
        last_kills,
        last_deaths,
        last_hs,
        has_combat,
        streak,
        rr_trend,
        recent_wins,
        recent_losses,
        smurf_score: 0,
        party_size: 0,
        encounters: 0,
        encounter_wins: 0,
        encounter_losses: 0,
        locked: false,
        premium_skins: false,
        vandal_skin: String::new(),
        vandal_image: String::new(),
        vandal_tier_color: String::new(),
    })
}

pub async fn build_rows(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    players: &[RawPlayer],
    sd: &StaticData,
    last_rows: &[PlayerRow],
    fetch_combat: bool,
    match_id: Option<&str>,
) -> Vec<PlayerRow> {
    let puuids: Vec<String> = players.iter().map(|p| p.puuid.clone()).collect();
    let names = fetch_names(ctx, region, version, &puuids).await;

    let self_team = players
        .iter()
        .find(|p| p.puuid == ctx.puuid)
        .map(|p| p.team.clone());

    // One core-game call covers premium-skin detection for all ten players.
    // Map by puuid when present, else fall back to the Players-order index.
    let mut premium_by_puuid: HashMap<String, bool> = HashMap::new();
    let mut premium_by_index: Vec<bool> = Vec::new();
    // Equipped Vandal skin (name, image, tier color) from the same loadouts.
    type Vandal = (String, String, String);
    let mut vandal_by_puuid: HashMap<String, Vandal> = HashMap::new();
    let mut vandal_by_index: Vec<Vandal> = Vec::new();
    if let Some(mid) = match_id {
        if let Some(loadouts) = fetch_loadouts(ctx, region, version, mid).await {
            for entry in &loadouts {
                let premium = has_premium_melee(entry, sd);
                premium_by_index.push(premium);
                let vandal = vandal_fields(entry, sd);
                vandal_by_index.push(vandal.clone());
                if let Some(subj) = loadout_subject(entry) {
                    premium_by_puuid.insert(subj.to_string(), premium);
                    vandal_by_puuid.insert(subj.to_string(), vandal);
                }
            }
        }
    }

    // Fetch each player's rank concurrently. Last-match combat stats (K/D, HS)
    // are an extra request per player, so they are only fetched when the user
    // opts in, matching vry's behaviour.
    let fetched = futures::future::join_all(players.iter().map(|p| {
        let puuid = p.puuid.clone();
        async move {
            let mmr = fetch_mmr(ctx, region, version, &puuid).await;
            let combat = if fetch_combat {
                fetch_player_recent(ctx, region, version, &puuid, RECENT_GAMES).await
            } else {
                None
            };
            (mmr, combat)
        }
    }))
    .await;

    let mut rows = Vec::with_capacity(players.len());
    for (i, (p, (mmr_opt, combat))) in players.iter().zip(fetched).enumerate() {
        let rank_failed = mmr_opt.is_none();
        let mmr = mmr_opt.unwrap_or_default();
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
        let (last_kills, last_deaths, last_hs, streak, rr_trend, recent_wins, recent_losses, has_combat) =
            match combat {
                Some((k, d, h, s, t, w, l)) => (k, d, h, s, t, w, l, true),
                None => (0, 0, 0, 0, 0, 0, 0, false),
            };
        let party_size = if p.party_id.is_empty() {
            1
        } else {
            players.iter().filter(|q| q.party_id == p.party_id).count() as u32
        };
        let premium_skins = premium_by_puuid
            .get(&p.puuid)
            .copied()
            .or_else(|| premium_by_index.get(i).copied())
            .unwrap_or(false);
        let (vandal_skin, vandal_image, vandal_tier_color) = vandal_by_puuid
            .get(&p.puuid)
            .or_else(|| vandal_by_index.get(i))
            .cloned()
            .unwrap_or_default();
        let smurf = smurf_score(account_level, mmr.tier, mmr.games, win_rate(&mmr), premium_skins);
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
            peak_rank_icon: sd.rank_icon(mmr.peak),
            peak_act: sd.season_label(&mmr.peak_season),
            win_rate: win_rate(&mmr),
            wins: mmr.wins,
            games: mmr.games,
            leaderboard: mmr.leaderboard,
            account_level,
            last_kills,
            last_deaths,
            last_hs,
            has_combat,
            streak,
            rr_trend,
            recent_wins,
            recent_losses,
            smurf_score: smurf,
            party_size,
            encounters: 0,
            encounter_wins: 0,
            encounter_losses: 0,
            locked: p.locked,
            premium_skins,
            vandal_skin,
            vandal_image,
            vandal_tier_color,
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
                row.peak_act = prev.peak_act.clone();
                row.win_rate = prev.win_rate;
                row.wins = prev.wins;
                row.games = prev.games;
                row.leaderboard = prev.leaderboard;
            }
        }
        // Keep last-known combat stats if this fetch did not get them.
        if !row.has_combat {
            if let Some(prev) = last_rows.iter().find(|r| r.puuid == p.puuid) {
                if prev.has_combat {
                    row.last_kills = prev.last_kills;
                    row.last_deaths = prev.last_deaths;
                    row.last_hs = prev.last_hs;
                    row.streak = prev.streak;
                    row.rr_trend = prev.rr_trend;
                    row.recent_wins = prev.recent_wins;
                    row.recent_losses = prev.recent_losses;
                    row.has_combat = true;
                }
            }
        }
        rows.push(row);
    }
    rows
}

/// Second pass over an already-built roster: fill in K/D, headshot, streak and
/// RR trend from each player's recent games. Throttled across the roster so a
/// heavy load cannot burst. Players with no recent comp match keep has_combat
/// false. Run after the roster is already on screen so ranks show immediately.
pub async fn enrich_combat(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    rows: &mut [PlayerRow],
    puuids: &[String],
) {
    // Split the match-detail budget across the whole roster so a big lobby
    // fetches fewer games per player and stays under the rate limit, even when
    // only a subset is being filled in this pass.
    let games = (RECENT_GAMES_BUDGET / rows.len().max(1)).clamp(1, RECENT_GAMES);
    let mut updates: Vec<(String, Option<(u32, u32, u32, i32, i32, u32, u32)>)> =
        Vec::with_capacity(puuids.len());
    for chunk in puuids.chunks(COMBAT_PLAYER_CONCURRENCY) {
        let part = futures::future::join_all(chunk.iter().map(|puuid| async move {
            let recent = fetch_player_recent(ctx, region, version, puuid, games).await;
            (puuid.clone(), recent)
        }))
        .await;
        updates.extend(part);
    }
    for (puuid, recent) in updates {
        if let Some((k, d, h, s, t, w, l)) = recent {
            if let Some(row) = rows.iter_mut().find(|r| r.puuid == puuid) {
                row.last_kills = k;
                row.last_deaths = d;
                row.last_hs = h;
                row.streak = s;
                row.rr_trend = t;
                row.recent_wins = w;
                row.recent_losses = l;
                row.has_combat = true;
            }
        }
    }
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
            row.locked = p.locked;
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
        assert_eq!(m.peak_season, "s2");
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

    fn rr_matches(values: &[i64]) -> Vec<Value> {
        values
            .iter()
            .map(|v| serde_json::json!({ "RankedRatingEarned": v }))
            .collect()
    }

    #[test]
    fn streak_counts_leading_win_run() {
        // Three wins then a loss: +3 streak, trend is the full sum.
        let (streak, trend) = streak_and_trend(&rr_matches(&[20, 18, 15, -12]));
        assert_eq!(streak, 3);
        assert_eq!(trend, 41);
    }

    #[test]
    fn streak_counts_leading_loss_run() {
        let (streak, trend) = streak_and_trend(&rr_matches(&[-10, -15, 20]));
        assert_eq!(streak, -2);
        assert_eq!(trend, -5);
    }

    #[test]
    fn streak_zero_when_latest_breaks_even() {
        let (streak, trend) = streak_and_trend(&rr_matches(&[0, 20, 18]));
        assert_eq!(streak, 0);
        assert_eq!(trend, 38);
    }

    #[test]
    fn smurf_flags_obvious_smurf() {
        // Level 22, Immortal (tier 24), 25 games, 70% winrate.
        let score = smurf_score(22, 24, 25, 70, false);
        assert!(score >= 50, "expected a high smurf score, got {score}");
    }

    #[test]
    fn smurf_clears_normal_account() {
        // Level 200, Gold (tier 12), 300 games, 50% winrate.
        assert_eq!(smurf_score(200, 12, 300, 50, false), 0);
    }
}
