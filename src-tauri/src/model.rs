use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub enum MatchState {
    NoGame,
    Menu,
    PreGame,
    CoreGame,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerRow {
    pub puuid: String,
    pub name: String,
    pub agent: String,
    pub team: String,
    pub party_id: String,
    pub hidden_name: bool,
    pub rank_tier: u32,
    pub rank_name: String,
    pub rr: u32,
    pub peak_rank_name: String,
    pub peak_rank_tier: u32,
    pub account_level: u32,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MatchView {
    pub state: MatchState,
    pub mode: String,
    pub players: Vec<PlayerRow>,
    pub me: Option<PlayerRow>,
    pub stale: bool,
}
