export type MatchState = "NoGame" | "Menu" | "PreGame" | "CoreGame";

export interface PlayerRow {
  puuid: string;
  name: string;
  playerCard: string;
  agent: string;
  agentIcon: string;
  team: string;
  partyId: string;
  hiddenName: boolean;
  rankTier: number;
  rankName: string;
  rankIcon: string;
  rr: number;
  peakRankName: string;
  peakRankTier: number;
  peakAct: string;
  winRate: number;
  wins: number;
  games: number;
  leaderboard: number;
  accountLevel: number;
}

export interface HistoryEntry {
  map: string;
  rrChange: number;
  tier: number;
  rankName: string;
  agentIcon: string;
  kills: number;
  deaths: number;
  assists: number;
  acs: number;
  selfRounds: number;
  enemyRounds: number;
  won: boolean;
  hasStats: boolean;
}

export interface MatchView {
  state: MatchState;
  mode: string;
  players: PlayerRow[];
  me: PlayerRow | null;
  history: HistoryEntry[];
  stale: boolean;
}
