import { PlayerRow, HistoryEntry } from "../types";
import { divColor, kdOf } from "../design";

function splitName(name: string): { name: string; tag: string } {
  const i = name.lastIndexOf("#");
  if (i < 0) return { name: name || "You", tag: "" };
  return { name: name.slice(0, i), tag: name.slice(i) };
}

// Cumulative RR path from history rrChange values. Real data, oldest -> newest.
// history arrives newest-first, so reverse it before accumulating.
function Sparkline({ history }: { history: HistoryEntry[] }) {
  const ordered = [...history].reverse();
  const v: number[] = [];
  let cur = 0;
  for (const h of ordered) {
    cur += h.rrChange;
    v.push(cur);
  }
  if (v.length < 2) return null;

  const w = 332;
  const hgt = 50;
  const pad = 4;
  const min = Math.min(...v);
  const max = Math.max(...v);
  const rng = max - min || 1;
  const pts = v.map((y, i): [number, number] => [
    pad + (w - pad * 2) * (i / (v.length - 1)),
    pad + (hgt - pad * 2) * (1 - (y - min) / rng),
  ]);
  const line = pts.map((p) => `${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(" ");
  const area =
    `M${pts[0][0].toFixed(1)},${hgt} ` +
    pts.map((p) => `L${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(" ") +
    ` L${pts[pts.length - 1][0].toFixed(1)},${hgt} Z`;
  const last = pts[pts.length - 1];
  const up = v[v.length - 1] >= v[0];
  const col = up ? "#5fb392" : "#ff4655";

  return (
    <svg viewBox={`0 0 ${w} ${hgt}`} preserveAspectRatio="none">
      <defs>
        <linearGradient id="sg" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor={col} stopOpacity=".16" />
          <stop offset="1" stopColor={col} stopOpacity="0" />
        </linearGradient>
      </defs>
      <path d={area} fill="url(#sg)" />
      <polyline
        points={line}
        fill="none"
        stroke={col}
        strokeWidth="1.6"
        strokeLinejoin="round"
        strokeLinecap="round"
      />
      <circle cx={last[0].toFixed(1)} cy={last[1].toFixed(1)} r="2.6" fill={col} />
    </svg>
  );
}

interface AgentTally {
  name: string;
  icon: string;
  count: number;
  kd: number;
}

// Most-played agents from history, grouped by agent name. Top 3 by appearance
// with their average K/D and play share.
function topAgents(history: HistoryEntry[]): { agents: AgentTally[]; total: number } {
  const map = new Map<string, { icon: string; count: number; kdSum: number }>();
  let total = 0;
  for (const h of history) {
    if (!h.agentName) continue;
    total++;
    const e = map.get(h.agentName) || { icon: h.agentIcon, count: 0, kdSum: 0 };
    if (!e.icon) e.icon = h.agentIcon;
    e.count++;
    e.kdSum += kdOf(h.kills, h.deaths);
    map.set(h.agentName, e);
  }
  const agents = [...map.entries()]
    .map(([name, e]): AgentTally => ({ name, icon: e.icon, count: e.count, kd: e.kdSum / e.count }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 3);
  return { agents, total };
}

export function ProfileCard({ me, history }: { me: PlayerRow; history: HistoryEntry[] }) {
  const { name, tag } = splitName(me.name);
  const ranked = me.rankTier > 0;
  const color = divColor(me.rankTier);
  const hasPeak = Boolean(me.peakRankName) && me.peakRankName !== "Unranked";

  // Recent form K/D and headshot for the signed in user, the same window and
  // source the in-match roster uses per player, so your own value reads the
  // same on both screens. Act-wide self stats are not available from the API.
  const recentKd = me.hasCombat ? kdOf(me.lastKills, me.lastDeaths) : null;
  const recentHs = me.hasCombat ? me.lastHs : null;

  const net = history.reduce((s, h) => s + h.rrChange, 0);
  const { agents, total } = topAgents(history);

  // RR progress within the division. We do not have the next-rank threshold, so
  // approximate against 100 RR per division.
  // TODO: real next-rank threshold needs backend support.
  const rrClamped = Math.max(0, Math.min(100, me.rr));

  return (
    <div className="profile">
      <div className="banner">
        <div className="slot">
          {me.playerCard && <img src={me.playerCard} alt="" onError={(e) => e.currentTarget.remove()} />}
        </div>
        <span className="emb bigemb">
          {ranked && me.rankIcon ? (
            <img src={me.rankIcon} alt="" onError={(e) => e.currentTarget.remove()} />
          ) : (
            <span className="chip" style={{ background: color }} />
          )}
        </span>
        <div className="who">
          <div className="nm">
            {name}
            {tag && <i>{tag}</i>}
          </div>
          {/* TODO: current act label is not in the data; show level only. */}
          {me.accountLevel > 0 && <div className="sub">LEVEL {me.accountLevel}</div>}
        </div>
      </div>

      <div className="prank">
        <div className="l1">
          <span className="rkname">
            <span className="emb">
              {ranked && me.rankIcon ? (
                <img src={me.rankIcon} alt="" onError={(e) => e.currentTarget.remove()} />
              ) : (
                <span className="chip" style={{ background: color }} />
              )}
            </span>
            {ranked ? me.rankName : "Unranked"}
            {me.leaderboard > 0 && <span className="lb"> #{me.leaderboard}</span>}
          </span>
          <span className="rr">
            <b className="mono">{ranked ? me.rr : "--"}</b> <span>RR</span>
          </span>
        </div>
        <div className="track">
          <i style={{ width: `${rrClamped}%` }} />
        </div>
        <div className="l2">
          <span>Rating progress</span>
          <span className="mono">{Math.max(0, 100 - me.rr)} RR to go</span>
        </div>
      </div>

      <div className="psum">
        <div className="stat">
          <div className="k">Win rate</div>
          <div className="v">
            {me.games > 0 ? me.winRate.toFixed(1) : "--"}
            <small>%</small>
          </div>
        </div>
        <div className="stat">
          <div className="k">K / D &middot; recent</div>
          <div className="v">{recentKd !== null ? recentKd.toFixed(2) : "--"}</div>
        </div>
        <div className="stat">
          <div className="k">Headshot &middot; recent</div>
          <div className="v">
            {recentHs !== null ? recentHs.toFixed(0) : "--"}
            <small>%</small>
          </div>
        </div>
        <div className="stat">
          <div className="k">Games (act)</div>
          <div className="v">{me.games}</div>
        </div>
      </div>

      {history.length >= 2 && (
        <div className="ptrend">
          <div className="hd">
            <span className="k">Rating trend &middot; last {history.length}</span>
            <span className="net" style={{ color: net >= 0 ? "var(--green)" : "var(--red)" }}>
              {net > 0 ? "+" : ""}
              {net} RR
            </span>
          </div>
          <Sparkline history={history} />
          <div className="axis">
            <span>{history.length} games ago</span>
            <span>now</span>
          </div>
        </div>
      )}

      {agents.length > 0 && (
        <div className="pagents">
          <div className="k">Most played &middot; recent</div>
          {agents.map((a) => {
            const pct = total > 0 ? Math.round((a.count / total) * 100) : 0;
            return (
              <div className="agrow" key={a.name}>
                <span className="agent">
                  {a.icon && <img src={a.icon} alt="" onError={(e) => e.currentTarget.remove()} />}
                </span>
                <div className="mid">
                  <div className="nm">
                    {a.name}
                    <span className="pct">{pct}%</span>
                  </div>
                  <div className="bar">
                    <i style={{ width: `${pct}%` }} />
                  </div>
                </div>
                <div className="kd">
                  {a.kd.toFixed(2)}
                  <small>K/D</small>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {hasPeak && (
        <div className="ppeak">
          <span className="emb">
            {me.peakRankIcon ? (
              <img src={me.peakRankIcon} alt="" onError={(e) => e.currentTarget.remove()} />
            ) : (
              <span className="chip" style={{ background: divColor(me.peakRankTier) }} />
            )}
          </span>
          <div className="col">
            <span className="k">Peak rating</span>
            <b>{me.peakRankName}</b>
          </div>
          {me.peakAct && (
            <div className="act">
              reached
              <br />
              {me.peakAct}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
