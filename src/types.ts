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
}

export interface MatchView {
  state: MatchState;
  mode: string;
  players: PlayerRow[];
  me: PlayerRow | null;
  history: HistoryEntry[];
  stale: boolean;
}
