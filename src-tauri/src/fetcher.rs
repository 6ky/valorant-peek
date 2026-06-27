use crate::auth::{pvp_headers, AuthContext};
use crate::client_version::Region;
use crate::match_state::RawPlayer;
use crate::model::{HistoryEntry, PlayerRow, ScoreEntry};
use crate::static_cache::StaticData;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

// Authed GET that parses JSON, backing off and retrying on HTTP 429. Reads
// Retry-After (seconds) for the wait, defaulting to 5, plus a 1s buffer. Gives
// up after 3 retries. Transport errors and parse errors return None.
async fn get_json_retry(url: &str, ctx: &AuthContext, version: &str) -> Option<Value> {
    crate::http::get_json_retry(url, pvp_headers(ctx, version)).await
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

/// The active act's season id, from the content service. Used to key the
/// current rank so a player who has not played this act reads as Unranked
/// instead of showing their last act's rank. None on any failure.
pub async fn fetch_current_act(ctx: &AuthContext, region: &Region, version: &str) -> Option<String> {
    let url = format!("{}/content-service/v3/content", region.shared_base());
    let body = crate::http::get_json_retry(&url, pvp_headers(ctx, version)).await?;
    let seasons = body.get("Seasons").and_then(|s| s.as_array())?;
    seasons
        .iter()
        .find(|s| {
            s.get("IsActive").and_then(|v| v.as_bool()) == Some(true)
                && s.get("Type").and_then(|v| v.as_str()) == Some("act")
        })
        .and_then(|s| s.get("ID").and_then(|v| v.as_str()).map(String::from))
}

pub fn parse_mmr(json: &Value, current_act: &str) -> Mmr {
    let seasons = json
        .get("QueueSkills")
        .and_then(|q| q.get("competitive"))
        .and_then(|c| c.get("SeasonalInfoBySeasonID"))
        .and_then(|s| s.as_object());

    let mut tier = 0u32;
    let mut rr = 0u32;
    let mut wins = 0u32;
    let mut games = 0u32;
    let mut leaderboard = 0u32;
    // Season the current rank was read from, used to seed the peak scan.
    let mut current_season = current_act.to_string();

    if !current_act.is_empty() {
        // Read the current act's entry. When it is absent the player has not
        // placed this act, so everything stays zero (Unranked).
        if let Some(info) = seasons.and_then(|s| s.get(current_act)) {
            tier = info.get("CompetitiveTier").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            // Tiers 1 and 2 are unused legacy slots, 0 is unranked.
            if tier <= 2 {
                tier = 0;
            }
            rr = info.get("RankedRating").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            games = info.get("NumberOfGames").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            wins = info
                .get("NumberOfWinsWithPlacements")
                .or_else(|| info.get("NumberOfWins"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            leaderboard = info.get("LeaderboardRank").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        }
    } else {
        // Content fetch failed; degrade to the last competitive game's tier and
        // rr. games/wins/leaderboard are read from the matching season below.
        let latest = json.get("LatestCompetitiveUpdate");
        tier = latest
            .and_then(|l| l.get("TierAfterUpdate"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        rr = latest
            .and_then(|l| l.get("RankedRatingAfterUpdate"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        current_season = latest
            .and_then(|l| l.get("SeasonID"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
    }

    // Peak comes from the tiers actually achieved (WinsByTier keys), not the
    // season-end tier, which can be lower than the peak. Seeded from the
    // current rank so the active act counts toward the peak too.
    let mut peak = tier;
    let mut peak_season = current_season.clone();
    if let Some(seasons) = seasons {
        for (id, info) in seasons {
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
            // In the fallback path, the current fields come from the season that
            // matches the last competitive game.
            if current_act.is_empty() && id == &current_season {
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
    let body =
        crate::http::put_json_retry(&url, pvp_headers(ctx, version), &serde_json::json!(puuids)).await;
    body.map(|v| parse_names(&v)).unwrap_or_default()
}

/// Returns None when the request itself failed (so callers can keep the last
/// known value instead of showing blank/unranked data).
pub async fn fetch_mmr(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    puuid: &str,
    current_act: &str,
) -> Option<Mmr> {
    let url = format!("{}/mmr/v1/players/{}", region.pd_base(), puuid);
    let body = get_json_retry(&url, ctx, version).await;
    body.map(|v| parse_mmr(&v, current_act))
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
    let body = get_json_retry(&url, ctx, version).await;
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
    let body = get_json_retry(&url, ctx, version).await;
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
                ranked: true,
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

/// Raw damage a player dealt across every round, summed from each round's damage
/// entries, mirroring shot_counts.
fn round_damage(detail: &Value, puuid: &str) -> u64 {
    let mut total = 0u64;
    if let Some(rounds) = detail.get("roundResults").and_then(|r| r.as_array()) {
        for round in rounds {
            if let Some(stats) = round.get("playerStats").and_then(|p| p.as_array()) {
                for ps in stats {
                    if ps.get("subject").and_then(|v| v.as_str()) != Some(puuid) {
                        continue;
                    }
                    if let Some(damage) = ps.get("damage").and_then(|d| d.as_array()) {
                        for d in damage {
                            total += d.get("damage").and_then(|v| v.as_u64()).unwrap_or(0);
                        }
                    }
                }
            }
        }
    }
    total
}

// A trade counts only when the avenger kills within this window of the player's death.
const TRADE_WINDOW_MS: i64 = 3000;

/// KAST tally for a player: (qualifying rounds, rounds present). A round counts
/// toward KAST when the player got a Kill, an Assist, Survived, or was Traded
/// within TRADE_WINDOW_MS of dying. Rounds the player was absent for are skipped.
fn kast_rounds(detail: &Value, puuid: &str) -> (u32, u32) {
    let (mut qualifying, mut present) = (0u32, 0u32);
    let rounds = match detail.get("roundResults").and_then(|r| r.as_array()) {
        Some(r) => r,
        None => return (0, 0),
    };
    for round in rounds {
        let stats = match round.get("playerStats").and_then(|p| p.as_array()) {
            Some(s) => s,
            None => continue,
        };
        let here = stats
            .iter()
            .any(|ps| ps.get("subject").and_then(|v| v.as_str()) == Some(puuid));
        if !here {
            continue;
        }
        present += 1;

        // (killer, victim, roundTime, assistants) for every kill in the round.
        let mut kills: Vec<(&str, &str, i64, Vec<&str>)> = Vec::new();
        for ps in stats {
            if let Some(arr) = ps.get("kills").and_then(|k| k.as_array()) {
                for k in arr {
                    let killer = k.get("killer").and_then(|v| v.as_str()).unwrap_or("");
                    let victim = k.get("victim").and_then(|v| v.as_str()).unwrap_or("");
                    let time = k.get("roundTime").and_then(|v| v.as_i64()).unwrap_or(0);
                    let assists = k
                        .get("assistants")
                        .and_then(|a| a.as_array())
                        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                        .unwrap_or_default();
                    kills.push((killer, victim, time, assists));
                }
            }
        }

        let got_kill = kills.iter().any(|(killer, ..)| *killer == puuid);
        let assisted = kills.iter().any(|(.., a)| a.contains(&puuid));
        let death = kills.iter().find(|(_, victim, ..)| *victim == puuid);
        let survived = death.is_none();
        let traded = match death {
            Some((killer, _, t, _)) => kills.iter().any(|(_, victim, vt, _)| {
                *victim == *killer && *vt >= *t && *vt <= *t + TRADE_WINDOW_MS
            }),
            None => false,
        };

        if got_kill || assisted || survived || traded {
            qualifying += 1;
        }
    }
    (qualifying, present)
}

#[derive(Clone, Default)]
pub struct MatchStats {
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub acs: u32,
    pub adr: u32,
    pub kast: u32,
    pub hs: u32,
    pub self_rounds: u32,
    pub enemy_rounds: u32,
    pub won: bool,
    pub map: String,
    pub map_image: String,
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
    let match_info = detail.get("matchInfo");
    let map_path = match_info
        .and_then(|m| m.get("mapId"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let queue_id = match_info
        .and_then(|m| m.get("queueId"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // Score races like deathmatch report one pseudo-round, so per-round stats
    // come out nonsensical. Leave them blank for those modes.
    let rounds_based = crate::presence::has_rounds(queue_id);
    let stat = |k: &str| {
        me.get("stats")
            .and_then(|s| s.get(k))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32
    };
    let rounds = stat("roundsPlayed").max(1);
    let acs = if rounds_based { stat("score") / rounds } else { 0 };
    let adr = if rounds_based {
        (round_damage(detail, puuid) / rounds as u64) as u32
    } else {
        0
    };
    let kast = if rounds_based {
        let (q, p) = kast_rounds(detail, puuid);
        if p > 0 {
            q * 100 / p
        } else {
            0
        }
    } else {
        0
    };
    let hs = if rounds_based { headshot_pct(detail, puuid) } else { 0 };

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
                acs: if rounds_based { g("score") / rp } else { 0 },
                hs: if rounds_based { headshot_pct(detail, subject) } else { 0 },
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
        adr,
        kast,
        hs,
        self_rounds,
        enemy_rounds,
        won,
        map: sd.map_name(map_path),
        map_image: sd.map_image(map_path),
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

// Number of recent competitive matches to aggregate K/D, ACS and headshot% over.
const RECENT_GAMES: usize = 10;
// Caps so a heavy roster load cannot burst into a rate limit: match-detail
// requests in flight per player, and players fetched at once across the roster.
// Product is the max requests in flight (3 x 2 = 6), kept low like vry, which
// fetches fully sequentially and leans on backoff.
const COMBAT_DETAIL_CONCURRENCY: usize = 2;
const COMBAT_PLAYER_CONCURRENCY: usize = 3;

/// Recent competitive form for a player: kills, deaths and headshot% aggregated
/// over their last `games` competitive matches, plus streak, RR trend and
/// win/loss record over the full recent Matches list, and the set of other
/// players who shared this player's party in any of those matches. None when the
/// history request fails or there are no comp matches; match details that fail to
/// load are skipped and the rest are still aggregated.
pub struct RecentForm {
    pub kills: u32,
    pub deaths: u32,
    pub hs: u32,
    pub acs: u32,
    pub adr: u32,
    pub kast: u32,
    pub assists: u32,
    pub streak: i32,
    pub rr_trend: i32,
    pub wins: u32,
    pub losses: u32,
    pub mates: Vec<String>,
    // The player's real account level, read from match details where it is not
    // zeroed for players who hide it in the live game.
    pub level: u32,
}

pub async fn fetch_player_recent(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    puuid: &str,
    games: usize,
) -> Option<RecentForm> {
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
    let (mut total_assists, mut total_score, mut total_damage) = (0u64, 0u64, 0u64);
    let (mut total_kast, mut total_rounds) = (0u64, 0u64);
    let mut mate_counts: HashMap<String, u32> = HashMap::new();
    let mut level = 0u32;
    for detail in details.into_iter().flatten() {
        let players = detail.get("players").and_then(|p| p.as_array());
        let me = players.and_then(|arr| {
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
            total_assists += stat("assists");
            total_score += stat("score");
            total_damage += round_damage(&detail, puuid);
            let (qual, present) = kast_rounds(&detail, puuid);
            total_kast += qual as u64;
            total_rounds += present as u64;
            // Account level lives at the top of the player object here and is the
            // real value even for players who hid it live. Keep the highest seen,
            // which is the most recent.
            level = level.max(me.get("accountLevel").and_then(|v| v.as_u64()).unwrap_or(0) as u32);
            // Count how many recent matches each other player shared this
            // player's party in. Co-queuing once is often incidental, so the
            // caller only treats repeat co-queues as a likely current premade.
            let my_party = me.get("partyId").and_then(|v| v.as_str()).unwrap_or("");
            if !my_party.is_empty() {
                if let Some(arr) = players {
                    for other in arr {
                        let subj = other.get("subject").and_then(|v| v.as_str()).unwrap_or("");
                        let party = other.get("partyId").and_then(|v| v.as_str()).unwrap_or("");
                        if subj != puuid && !subj.is_empty() && party == my_party {
                            *mate_counts.entry(subj.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
        let (h, b, l) = shot_counts(&detail, puuid);
        head += h;
        body += b;
        leg += l;
    }

    let shots = head + body + leg;
    let hs_pct = if shots > 0 { (head * 100 / shots) as u32 } else { 0 };
    let rounds = total_rounds.max(1);
    Some(RecentForm {
        kills: total_kills as u32,
        deaths: total_deaths as u32,
        hs: hs_pct,
        acs: (total_score / rounds) as u32,
        adr: (total_damage / rounds) as u32,
        kast: (total_kast * 100 / rounds) as u32,
        assists: total_assists as u32,
        streak,
        rr_trend,
        wins: recent_wins,
        losses: recent_losses,
        mates: mate_counts
            .into_iter()
            .filter(|(_, count)| *count >= 2)
            .map(|(subj, _)| subj)
            .collect(),
        level,
    })
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
    let body = get_json_retry(&url, ctx, version).await?;
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

/// Recent matches for the chosen queue (0 competitive, 1 unrated, 2 all). Each
/// row's combat stats come from its match detail; competitive rows also carry
/// the RR change and rank from competitiveupdates.
pub async fn fetch_history(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    sd: &StaticData,
    cache: &mut HashMap<String, MatchStats>,
    queue: u8,
) -> Vec<HistoryEntry> {
    let (mut entries, ids) = if queue == 0 {
        comp_history(ctx, region, version, sd).await
    } else {
        mode_history(ctx, region, version, queue).await
    };

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
            entry.adr = s.adr;
            entry.kast = s.kast;
            entry.hs = s.hs;
            entry.self_rounds = s.self_rounds;
            entry.enemy_rounds = s.enemy_rounds;
            entry.won = s.won;
            // Non-competitive rows have no map yet; competitive already has it
            // from the update feed and the match detail agrees.
            if !s.map.is_empty() {
                entry.map = s.map;
                entry.map_image = s.map_image;
            }
            entry.scoreboard = s.scoreboard;
            entry.has_stats = true;
        }
    }
    entries
}

// Competitive history from competitiveupdates: rows carry tier and RR change.
async fn comp_history(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    sd: &StaticData,
) -> (Vec<HistoryEntry>, Vec<String>) {
    let url = format!(
        "{}/mmr/v1/players/{}/competitiveupdates?startIndex=0&endIndex=15&queue=competitive",
        region.pd_base(),
        ctx.puuid
    );
    let json = match get_json_retry(&url, ctx, version).await {
        Some(j) => j,
        None => return (Vec::new(), Vec::new()),
    };
    let entries = parse_history(&json, sd);
    let ids = json
        .get("Matches")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .map(|m| m.get("MatchID").and_then(|v| v.as_str()).unwrap_or("").to_string())
                .collect()
        })
        .unwrap_or_default();
    (entries, ids)
}

// Recent matches for a non-competitive queue from match-history. There is no RR,
// so each row is left unranked and labelled with its mode; stats come from the
// match detail in the shared enrichment pass. queue 1 is unrated, any other
// value returns every mode.
async fn mode_history(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    queue: u8,
) -> (Vec<HistoryEntry>, Vec<String>) {
    let filter = if queue == 1 { "&queue=unrated" } else { "" };
    let url = format!(
        "{}/match-history/v1/history/{}?startIndex=0&endIndex=15{}",
        region.pd_base(),
        ctx.puuid,
        filter
    );
    let json = match get_json_retry(&url, ctx, version).await {
        Some(j) => j,
        None => return (Vec::new(), Vec::new()),
    };
    let history = match json.get("History").and_then(|h| h.as_array()) {
        Some(h) => h,
        None => return (Vec::new(), Vec::new()),
    };
    let mut entries = Vec::with_capacity(history.len());
    let mut ids = Vec::with_capacity(history.len());
    for m in history {
        let id = m.get("MatchID").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let queue_id = m.get("QueueID").and_then(|v| v.as_str()).unwrap_or("");
        entries.push(HistoryEntry {
            rank_name: crate::presence::mode_name(queue_id),
            ranked: false,
            ..Default::default()
        });
        ids.push(id);
    }
    (entries, ids)
}

/// Build a row for the signed-in user, for display when not in a match.
pub async fn build_self(
    ctx: &AuthContext,
    region: &Region,
    version: &str,
    current_act: &str,
    sd: &StaticData,
    party_map: &HashMap<String, String>,
) -> Option<PlayerRow> {
    // If the rank request fails, return None so the caller keeps the last
    // known profile instead of flashing unranked.
    let mmr = fetch_mmr(ctx, region, version, &ctx.puuid, current_act).await?;
    let puuids = [ctx.puuid.clone()];
    let names = fetch_names(ctx, region, version, &puuids).await;
    let level = fetch_account_level(ctx, region, version, &ctx.puuid).await;
    let card_id = fetch_loadout_card(ctx, region, version, &ctx.puuid).await;
    let (last_kills, last_deaths, last_hs, streak, rr_trend, recent_wins, recent_losses, has_combat) =
        match fetch_player_recent(ctx, region, version, &ctx.puuid, RECENT_GAMES).await {
            Some(r) => (r.kills, r.deaths, r.hs, r.streak, r.rr_trend, r.wins, r.losses, true),
            None => (0, 0, 0, 0, 0, 0, 0, false),
        };
    Some(PlayerRow {
        puuid: ctx.puuid.clone(),
        name: names.get(&ctx.puuid).cloned().unwrap_or_default(),
        player_card: sd.card_art(&card_id),
        agent: String::new(),
        agent_icon: String::new(),
        team: String::new(),
        party_id: party_map.get(&ctx.puuid).cloned().unwrap_or_default(),
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
        last_acs: 0,
        last_adr: 0,
        last_kast: 0,
        last_assists: 0,
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
    current_act: &str,
    players: &[RawPlayer],
    sd: &StaticData,
    last_rows: &[PlayerRow],
    party_map: &HashMap<String, String>,
    fetch_combat: bool,
    match_id: Option<&str>,
) -> Vec<PlayerRow> {
    let puuids: Vec<String> = players.iter().map(|p| p.puuid.clone()).collect();
    let names = fetch_names(ctx, region, version, &puuids).await;
    // Our own level from the self-only account-xp endpoint, so it still shows
    // when we have hidden it (Riot zeroes the hidden level in the match payload).
    let self_level = fetch_account_level(ctx, region, version, &ctx.puuid).await;

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
    // Fetch ranks in small concurrent batches rather than all at once, so a
    // full lobby does not burst the rank endpoint into a rate limit.
    const MMR_CONCURRENCY: usize = 4;
    let mut fetched = Vec::with_capacity(players.len());
    for chunk in players.chunks(MMR_CONCURRENCY) {
        let part = futures::future::join_all(chunk.iter().map(|p| {
            let puuid = p.puuid.clone();
            async move {
                let mmr = fetch_mmr(ctx, region, version, &puuid, current_act).await;
                let combat = if fetch_combat {
                    fetch_player_recent(ctx, region, version, &puuid, RECENT_GAMES).await
                } else {
                    None
                };
                (mmr, combat)
            }
        }))
        .await;
        fetched.extend(part);
    }

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
        // Riot zeroes a hidden account level in the match payload and there is no
        // way to read another player's hidden level. For ourselves we fall back
        // to the account-xp value, which is ours to read.
        let account_level = if is_self && self_level > 0 {
            self_level
        } else {
            p.account_level
        };
        let (last_kills, last_deaths, last_hs, streak, rr_trend, recent_wins, recent_losses, has_combat) =
            match combat {
                Some(r) => (r.kills, r.deaths, r.hs, r.streak, r.rr_trend, r.wins, r.losses, true),
                None => (0, 0, 0, 0, 0, 0, 0, false),
            };
        // Ally parties come from the presence map; enemies fill in later via
        // match-history inference. Size counts roster members sharing the same
        // non-empty party id.
        let party_id = party_map.get(&p.puuid).cloned().unwrap_or_default();
        let party_size = if party_id.is_empty() {
            1
        } else {
            players
                .iter()
                .filter(|q| party_map.get(&q.puuid).map(|s| s.as_str()) == Some(party_id.as_str()))
                .count() as u32
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
            party_id,
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
            last_acs: 0,
            last_adr: 0,
            last_kast: 0,
            last_assists: 0,
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
                    row.last_acs = prev.last_acs;
                    row.last_adr = prev.last_adr;
                    row.last_kast = prev.last_kast;
                    row.last_assists = prev.last_assists;
                    row.streak = prev.streak;
                    row.rr_trend = prev.rr_trend;
                    row.recent_wins = prev.recent_wins;
                    row.recent_losses = prev.recent_losses;
                    row.has_combat = true;
                }
            }
        }
        // Keep a level recovered from match history across a rebuild, since the
        // live payload zeroes it for players who hide it.
        if row.account_level == 0 {
            if let Some(prev) = last_rows.iter().find(|r| r.puuid == p.puuid) {
                if prev.account_level > 0 {
                    row.account_level = prev.account_level;
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
    mates_map: &mut HashMap<String, HashSet<String>>,
    party_map: &HashMap<String, String>,
) {
    // Aggregate every player's stats over the same number of recent games
    // regardless of lobby size. The combat pass is already throttled (a few
    // players per poll, a couple of match details in flight each), and the 429
    // backoff covers the rest, so a full lobby stays under the rate limit while
    // still averaging over the full window rather than a thin slice.
    let games = RECENT_GAMES;
    let mut updates: Vec<(String, Option<RecentForm>)> = Vec::with_capacity(puuids.len());
    for chunk in puuids.chunks(COMBAT_PLAYER_CONCURRENCY) {
        let part = futures::future::join_all(chunk.iter().map(|puuid| async move {
            let recent = fetch_player_recent(ctx, region, version, puuid, games).await;
            (puuid.clone(), recent)
        }))
        .await;
        updates.extend(part);
    }
    for (puuid, recent) in updates {
        if let Some(r) = recent {
            mates_map.insert(puuid.clone(), r.mates.iter().cloned().collect());
            if let Some(row) = rows.iter_mut().find(|row| row.puuid == puuid) {
                row.last_kills = r.kills;
                row.last_deaths = r.deaths;
                row.last_hs = r.hs;
                row.last_acs = r.acs;
                row.last_adr = r.adr;
                row.last_kast = r.kast;
                row.last_assists = r.assists;
                row.streak = r.streak;
                row.rr_trend = r.rr_trend;
                row.recent_wins = r.wins;
                row.recent_losses = r.losses;
                // Fill in a hidden player's level from their match history, since
                // the live game zeroes it. Leave a visible live level alone.
                if row.account_level == 0 && r.level > 0 {
                    row.account_level = r.level;
                }
                row.has_combat = true;
            }
        }
    }
    apply_parties(rows, party_map, mates_map);
}

/// Resolve every row's party from two sources: presence party ids (authoritative
/// for allies) and match-history mates (inferred for enemies). Presence ids are
/// never overwritten. Rows with no presence id are grouped by union-find: any two
/// rows that appeared in each other's recent mate sets join the same component,
/// which gets a synthetic id keyed on its smallest puuid. party_size is then the
/// count of rows sharing each resolved non-empty id.
fn apply_parties(
    rows: &mut [PlayerRow],
    party_map: &HashMap<String, String>,
    mates_map: &HashMap<String, HashSet<String>>,
) {
    let n = rows.len();
    // Authoritative presence id per row, empty when the player has none.
    let pres: Vec<String> = rows
        .iter()
        .map(|r| party_map.get(&r.puuid).cloned().unwrap_or_default())
        .collect();

    // Union-find over row indices, used only for rows lacking a presence id.
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    fn union(parent: &mut [usize], a: usize, b: usize) {
        let (ra, rb) = (find(parent, a), find(parent, b));
        if ra != rb {
            parent[ra.max(rb)] = ra.min(rb);
        }
    }
    for a in 0..n {
        if !pres[a].is_empty() {
            continue;
        }
        for b in (a + 1)..n {
            if !pres[b].is_empty() {
                continue;
            }
            let a_has_b = mates_map
                .get(&rows[a].puuid)
                .map(|s| s.contains(&rows[b].puuid))
                .unwrap_or(false);
            let b_has_a = mates_map
                .get(&rows[b].puuid)
                .map(|s| s.contains(&rows[a].puuid))
                .unwrap_or(false);
            if a_has_b || b_has_a {
                union(&mut parent, a, b);
            }
        }
    }

    // Smallest puuid per component, used to mint a stable synthetic id.
    let mut root_min: HashMap<usize, String> = HashMap::new();
    for i in 0..n {
        if !pres[i].is_empty() {
            continue;
        }
        let r = find(&mut parent, i);
        let e = root_min.entry(r).or_insert_with(|| rows[i].puuid.clone());
        if rows[i].puuid < *e {
            *e = rows[i].puuid.clone();
        }
    }
    // Count component members so singletons keep an empty id.
    let mut comp_size: HashMap<usize, u32> = HashMap::new();
    for i in 0..n {
        if pres[i].is_empty() {
            *comp_size.entry(find(&mut parent, i)).or_insert(0) += 1;
        }
    }

    for i in 0..n {
        if !pres[i].is_empty() {
            rows[i].party_id = pres[i].clone();
        } else {
            let r = find(&mut parent, i);
            if comp_size.get(&r).copied().unwrap_or(0) >= 2 {
                rows[i].party_id = format!("h:{}", root_min[&r]);
            } else {
                rows[i].party_id = String::new();
            }
        }
    }

    // Recompute sizes from the resolved ids.
    let mut id_count: HashMap<String, u32> = HashMap::new();
    for r in rows.iter() {
        if !r.party_id.is_empty() {
            *id_count.entry(r.party_id.clone()).or_insert(0) += 1;
        }
    }
    for r in rows.iter_mut() {
        r.party_size = if r.party_id.is_empty() {
            1
        } else {
            id_count.get(&r.party_id).copied().unwrap_or(1)
        };
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
            // The live payload reports 0 for a hidden level, so only let a real
            // value update it. Otherwise this would wipe a level recovered from
            // match history every poll.
            if p.account_level > 0 {
                row.account_level = p.account_level;
            }
            row
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(puuid: &str) -> PlayerRow {
        PlayerRow {
            puuid: puuid.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn infers_enemy_party_from_shared_history() {
        let mut rows = vec![row("a"), row("b"), row("c")];
        let party_map = HashMap::new();
        let mut mates: HashMap<String, HashSet<String>> = HashMap::new();
        mates.insert("a".to_string(), HashSet::from(["b".to_string()]));
        apply_parties(&mut rows, &party_map, &mates);
        assert_eq!(rows[0].party_id, "h:a");
        assert_eq!(rows[1].party_id, "h:a");
        assert_eq!(rows[0].party_size, 2);
        assert_eq!(rows[1].party_size, 2);
        assert!(rows[2].party_id.is_empty());
        assert_eq!(rows[2].party_size, 1);
    }

    #[test]
    fn presence_party_is_not_overwritten_by_inference() {
        let mut rows = vec![row("a"), row("b")];
        let mut party_map = HashMap::new();
        party_map.insert("a".to_string(), "real-party".to_string());
        // History would otherwise pair a with b, but a has a presence id.
        let mut mates: HashMap<String, HashSet<String>> = HashMap::new();
        mates.insert("a".to_string(), HashSet::from(["b".to_string()]));
        apply_parties(&mut rows, &party_map, &mates);
        assert_eq!(rows[0].party_id, "real-party");
        assert_eq!(rows[0].party_size, 1);
        assert!(rows[1].party_id.is_empty());
        assert_eq!(rows[1].party_size, 1);
    }

    #[test]
    fn lone_player_stays_size_one() {
        let mut rows = vec![row("a")];
        apply_parties(&mut rows, &HashMap::new(), &HashMap::new());
        assert!(rows[0].party_id.is_empty());
        assert_eq!(rows[0].party_size, 1);
    }

    #[test]
    fn kast_counts_kill_survive_and_trade() {
        // Round 1: player p got a kill (qualifies).
        // Round 2: p died with no kill, assist, or trade (does not qualify).
        // Round 3: p died but teammate t traded the killer e within 3000ms
        // (qualifies). p is present in every round.
        let v: Value = serde_json::from_str(
            r#"{"roundResults":[
              {"playerStats":[
                {"subject":"p","kills":[{"killer":"p","victim":"e","roundTime":1000,"assistants":[]}]},
                {"subject":"e","kills":[]}
              ]},
              {"playerStats":[
                {"subject":"p","kills":[]},
                {"subject":"e","kills":[{"killer":"e","victim":"p","roundTime":2000,"assistants":[]}]}
              ]},
              {"playerStats":[
                {"subject":"p","kills":[]},
                {"subject":"t","kills":[{"killer":"t","victim":"e","roundTime":2500,"assistants":[]}]},
                {"subject":"e","kills":[{"killer":"e","victim":"p","roundTime":1500,"assistants":[]}]}
              ]}
            ]}"#,
        )
        .unwrap();
        assert_eq!(kast_rounds(&v, "p"), (2, 3));
    }

    #[test]
    fn parses_mmr_current_and_peak() {
        // Current rank comes from the active act's CompetitiveTier and
        // RankedRating, not the last-game TierAfterUpdate. Peak comes from
        // WinsByTier (tiers achieved), not the lower season-end tier: the player
        // ended s2 at 23 but peaked 25.
        let v: Value = serde_json::from_str(
            r#"{
              "LatestCompetitiveUpdate":{"TierAfterUpdate":23,"RankedRatingAfterUpdate":42,"SeasonID":"s2"},
              "QueueSkills":{"competitive":{"SeasonalInfoBySeasonID":{
                "s1":{"CompetitiveTier":18,"WinsByTier":{"17":3,"18":5}},
                "s2":{"CompetitiveTier":23,"RankedRating":55,"WinsByTier":{"24":4,"25":2},"NumberOfWins":7,"NumberOfWinsWithPlacements":10,"NumberOfGames":15,"LeaderboardRank":0}
              }}}}"#,
        )
        .unwrap();
        let m = parse_mmr(&v, "s2");
        assert_eq!(m.tier, 23);
        assert_eq!(m.rr, 55);
        assert_eq!(m.peak, 25);
        assert_eq!(m.peak_season, "s2");
        // wins must include placement wins (10), not the lower NumberOfWins (7)
        assert_eq!(m.wins, 10);
        assert_eq!(m.games, 15);
        assert_eq!(win_rate(&m), 66);
    }

    #[test]
    fn parses_mmr_unranked_when_act_absent() {
        // Player has prior-act history but no entry for the current act, so the
        // current rank reads Unranked while peak still reflects past acts.
        let v: Value = serde_json::from_str(
            r#"{
              "LatestCompetitiveUpdate":{"TierAfterUpdate":23,"RankedRatingAfterUpdate":42,"SeasonID":"s2"},
              "QueueSkills":{"competitive":{"SeasonalInfoBySeasonID":{
                "s1":{"CompetitiveTier":18,"WinsByTier":{"17":3,"18":5}},
                "s2":{"CompetitiveTier":23,"WinsByTier":{"24":4,"25":2}}
              }}}}"#,
        )
        .unwrap();
        let m = parse_mmr(&v, "s3");
        assert_eq!(m.tier, 0);
        assert_eq!(m.rr, 0);
        assert_eq!(m.games, 0);
        assert_eq!(m.wins, 0);
        // peak still reflects the best tier ever reached
        assert_eq!(m.peak, 25);
        assert_eq!(m.peak_season, "s2");
    }

    #[test]
    fn parses_mmr_falls_back_without_act() {
        // Empty current_act means the content fetch failed, so degrade to the
        // last competitive game's tier and rr.
        let v: Value = serde_json::from_str(
            r#"{
              "LatestCompetitiveUpdate":{"TierAfterUpdate":23,"RankedRatingAfterUpdate":42,"SeasonID":"s2"},
              "QueueSkills":{"competitive":{"SeasonalInfoBySeasonID":{
                "s2":{"CompetitiveTier":23,"WinsByTier":{"24":4,"25":2},"NumberOfWinsWithPlacements":10,"NumberOfGames":15}
              }}}}"#,
        )
        .unwrap();
        let m = parse_mmr(&v, "");
        assert_eq!(m.tier, 23);
        assert_eq!(m.rr, 42);
        assert_eq!(m.games, 15);
        assert_eq!(m.wins, 10);
        assert_eq!(m.peak, 25);
    }

    #[test]
    fn parses_mmr_handles_missing() {
        let v: Value = serde_json::from_str("{}").unwrap();
        assert_eq!(parse_mmr(&v, ""), Mmr::default());
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
