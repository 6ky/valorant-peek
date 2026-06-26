use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchState {
    NoGame,
    Menu,
    PreGame,
    CoreGame,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayerRow {
    pub puuid: String,
    pub name: String,
    pub player_card: String,
    pub agent: String,
    pub agent_icon: String,
    pub team: String,
    pub party_id: String,
    pub hidden_name: bool,
    pub rank_tier: u32,
    pub rank_name: String,
    pub rank_icon: String,
    pub rr: u32,
    pub peak_rank_name: String,
    pub peak_rank_tier: u32,
    pub peak_rank_icon: String,
    pub peak_act: String,
    pub win_rate: u32,
    pub wins: u32,
    pub games: u32,
    pub leaderboard: u32,
    pub account_level: u32,
    pub last_kills: u32,
    pub last_deaths: u32,
    pub last_hs: u32,
    pub has_combat: bool,
    // Signed run of recent competitive results: positive is a win streak,
    // negative a loss streak, zero when the last result broke the run.
    pub streak: i32,
    // Sum of RR gained or lost over the last handful of competitive games.
    pub rr_trend: i32,
    // Composite 0 to 100 read on how likely this account is a smurf.
    pub smurf_score: u32,
    // Number of players in this match sharing this player's party (1 is solo).
    pub party_size: u32,
    // How many earlier matches we have seen this player in, and the record of
    // those matches from the signed-in user's point of view.
    pub encounters: u32,
    pub encounter_wins: u32,
    pub encounter_losses: u32,
    // Agent select only: whether this player has locked their agent.
    pub locked: bool,
    // Whether this player runs premium weapon skins (a smurf tiebreaker).
    pub premium_skins: bool,
    // Equipped Vandal skin: display name, art, and content-tier accent color.
    pub vandal_skin: String,
    pub vandal_image: String,
    pub vandal_tier_color: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScoreEntry {
    pub name: String,
    pub agent_icon: String,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub acs: u32,
    pub hs: u32,
    pub ally: bool,
    pub is_self: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub map: String,
    pub rr_change: i32,
    pub tier: u32,
    pub rank_name: String,
    pub agent_icon: String,
    pub agent_name: String,
    pub map_image: String,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub acs: u32,
    pub hs: u32,
    pub self_rounds: u32,
    pub enemy_rounds: u32,
    pub won: bool,
    pub has_stats: bool,
    pub scoreboard: Vec<ScoreEntry>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MatchView {
    pub state: MatchState,
    pub mode: String,
    pub activity: String,
    pub players: Vec<PlayerRow>,
    pub me: Option<PlayerRow>,
    pub history: Vec<HistoryEntry>,
    pub stale: bool,
    // Seconds left in the current agent select phase, zero outside of pregame.
    pub phase_time: u32,
    // Current map and live round score while in a game.
    pub map: String,
    pub map_image: String,
    pub ally_score: u32,
    pub enemy_score: u32,
}
