import { MatchState } from "../types";

const MESSAGES: Record<MatchState, string> = {
  NoGame: "Waiting for VALORANT...",
  Menu: "In menu - queue a game",
  PreGame: "Loading match...",
  CoreGame: "Loading match...",
};

export function StatusScreen({ state }: { state: MatchState }) {
  return <div className="status">{MESSAGES[state]}</div>;
}
