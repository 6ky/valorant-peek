import { PlayerRow as Row } from "../types";
import { PlayerRow } from "./PlayerRow";

function byRank(players: Row[]): Row[] {
  return [...players].sort((a, b) => b.rankTier - a.rankTier);
}

function Section({ title, color, players }: { title: string; color: string; players: Row[] }) {
  if (players.length === 0) return null;
  return (
    <section className="team">
      <div className="team-head" style={{ color }}>
        <span className="team-rail" style={{ background: color }} />
        {title}
        <span className="team-count">{players.length}</span>
      </div>
      <div className="team-rows">
        {byRank(players).map((p) => (
          <PlayerRow key={p.puuid} row={p} />
        ))}
      </div>
    </section>
  );
}

export function PlayerTable({ players }: { players: Row[] }) {
  const ally = players.filter((p) => p.team === "Ally");
  const enemy = players.filter((p) => p.team === "Enemy");
  const neutral = players.filter((p) => p.team !== "Ally" && p.team !== "Enemy");

  return (
    <div className="roster">
      <Section title="Allies" color="#28c8a0" players={ally} />
      <Section title="Enemies" color="#ff4655" players={enemy} />
      <Section title="Players" color="#c6cfd8" players={neutral} />
    </div>
  );
}
