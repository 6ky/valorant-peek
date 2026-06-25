export type MatchState = "NoGame" | "Menu" | "PreGame" | "CoreGame";

export interface PlayerRow {
  puuid: string;
  name: string;
  agent: string;
  team: string;
  partyId: string;
  rankTier: number;
  rankName: string;
  rr: number;
  peakRankName: string;
  accountLevel: number;
}

export interface MatchView {
  state: MatchState;
  players: PlayerRow[];
  stale: boolean;
}
