import { CSSProperties } from "react";
import { PlayerRow as Row, MatchState } from "../types";
import { PlayerRow } from "./PlayerRow";
import { kdOf } from "../design";

function byRank(players: Row[]): Row[] {
  return [...players].sort((a, b) => b.rankTier - a.rankTier);
}

function avgWinRate(players: Row[]): number | null {
  const withGames = players.filter((p) => p.games > 0);
  if (withGames.length === 0) return null;
  return withGames.reduce((s, p) => s + p.winRate, 0) / withGames.length;
}

function avgKd(players: Row[]): number {
  const withCombat = players.filter((p) => p.hasCombat);
  if (withCombat.length === 0) return 0;
  return withCombat.reduce((s, p) => s + kdOf(p.lastKills, p.lastDeaths), 0) / withCombat.length;
}

function ColumnHeader() {
  return (
    <div className="colhdr">
      <span />
      <span />
      <span>Player</span>
      <span>Rank</span>
      <span>Vandal</span>
      <span className="r">RR</span>
      <span className="r">K/D</span>
      <span className="r">HS%</span>
      <span className="r">Win%</span>
      <span>Peak</span>
    </div>
  );
}

function TeamHeader({
  kind,
  label,
  meta,
  metaRight,
}: {
  kind: "ally" | "enemy" | "lobby";
  label: string;
  meta: string;
  metaRight: string;
}) {
  return (
    <div className={`teamhdr ${kind}`}>
      <span className="lab">{label}</span>
      <span className="meta">{meta}</span>
      <span className="line" />
      <span className="meta">{metaRight}</span>
    </div>
  );
}

function Compare({ ally, enemy }: { ally: Row[]; enemy: Row[] }) {
  const aw = avgWinRate(ally);
  const ew = avgWinRate(enemy);
  if (aw === null && ew === null) return null;
  const av = aw ?? 0;
  const ev = ew ?? 0;
  const total = av + ev || 1;
  return (
    <div className="compare">
      <div className="top">
        <div className="side">
          <span>Allies</span>
          <span className="k">
            win <b className="aw">{Math.round(av)}%</b>
          </span>
        </div>
        <span className="mid">Team comparison</span>
        <div className="side">
          <span className="k">
            win <b className="ew">{Math.round(ev)}%</b>
          </span>
          <span>Enemies</span>
        </div>
      </div>
      <div className="bar">
        <i className="a" style={{ width: `${(av / total) * 100}%` }} />
        <i className="e" style={{ width: `${(ev / total) * 100}%` }} />
        <span className="vs" />
      </div>
    </div>
  );
}

export function PlayerTable({ players, state }: { players: Row[]; state: MatchState }) {
  // Agent select dims allies who have not locked in yet; in CoreGame the lock
  // flag is meaningless, so the picking treatment is suppressed.
  const pregame = state === "PreGame";
  const ally = players.filter((p) => p.team === "Ally");
  const enemy = players.filter((p) => p.team === "Enemy");
  const split = ally.length > 0 || enemy.length > 0;
  const avgTier = players.length
    ? players.reduce((s, p) => s + p.rankTier, 0) / players.length
    : 0;

  // No Ally/Enemy split -> free-for-all single lobby (deathmatch style).
  if (!split) {
    const dmRows: CSSProperties = { "--rh": "40px", "--gap": "3px" } as CSSProperties;
    return (
      <div className="roster">
        <TeamHeader
          kind="lobby"
          label="Lobby"
          meta={`${players.length} players · free for all`}
          metaRight={`${avgKd(players).toFixed(2)} avg kd`}
        />
        <ColumnHeader />
        <div className="rows" style={dmRows}>
          {byRank(players).map((p) => (
            <PlayerRow key={p.puuid} row={p} isEnemy={false} avgTier={avgTier} dm picking={false} />
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="roster">
      <ColumnHeader />
      <TeamHeader
        kind="ally"
        label="Allies"
        meta={`${ally.length} players`}
        metaRight={`${avgKd(ally).toFixed(2)} avg kd`}
      />
      <div className="rows">
        {byRank(ally).map((p) => (
          <PlayerRow
            key={p.puuid}
            row={p}
            isEnemy={false}
            avgTier={avgTier}
            dm={false}
            picking={pregame && !p.locked}
          />
        ))}
      </div>
      <Compare ally={ally} enemy={enemy} />
      <TeamHeader
        kind="enemy"
        label="Enemies"
        meta={`${enemy.length} players`}
        metaRight={`${avgKd(enemy).toFixed(2)} avg kd`}
      />
      <div className="rows">
        {byRank(enemy).map((p) => (
          <PlayerRow key={p.puuid} row={p} isEnemy avgTier={avgTier} dm={false} picking={false} />
        ))}
      </div>
    </div>
  );
}
