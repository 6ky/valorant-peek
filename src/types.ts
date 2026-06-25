export type MatchState = "NoGame" | "Menu" | "PreGame" | "CoreGame";

export interface PlayerRow {
  puuid: string;
  name: string;
  agent: string;
  team: string;
  partyId: string;
  hiddenName: boolean;
  rankTier: number;
  rankName: string;
  rr: number;
  peakRankName: string;
  peakRankTier: number;
  accountLevel: number;
}

export interface MatchView {
  state: MatchState;
  mode: string;
  players: PlayerRow[];
  me: PlayerRow | null;
  stale: boolean;
}
