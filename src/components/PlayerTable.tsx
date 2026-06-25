import { PlayerRow as Row } from "../types";
import { PlayerRow } from "./PlayerRow";

function sorted(players: Row[]): Row[] {
  return [...players].sort((a, b) => {
    if (a.team !== b.team) return a.team.localeCompare(b.team);
    return b.rankTier - a.rankTier;
  });
}

export function PlayerTable({ players }: { players: Row[] }) {
  return (
    <table className="player-table">
      <thead>
        <tr>
          <th>Player</th>
          <th>Agent</th>
          <th>Rank</th>
          <th>RR</th>
          <th>Peak</th>
          <th>Level</th>
        </tr>
      </thead>
      <tbody>
        {sorted(players).map((p) => (
          <PlayerRow key={p.puuid} row={p} />
        ))}
      </tbody>
    </table>
  );
}
