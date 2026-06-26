use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
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
    pub peak_act: String,
    pub win_rate: u32,
    pub wins: u32,
    pub games: u32,
    pub leaderboard: u32,
    pub account_level: u32,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub map: String,
    pub rr_change: i32,
    pub tier: u32,
    pub rank_name: String,
    pub agent_icon: String,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub acs: u32,
    pub self_rounds: u32,
    pub enemy_rounds: u32,
    pub won: bool,
    pub has_stats: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MatchView {
    pub state: MatchState,
    pub mode: String,
    pub players: Vec<PlayerRow>,
    pub me: Option<PlayerRow>,
    pub history: Vec<HistoryEntry>,
    pub stale: bool,
}
