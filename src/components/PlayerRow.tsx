import { CSSProperties } from "react";
import { PlayerRow as Row } from "../types";
import { agentMeta, divColor, kdOf, tone } from "../design";

// Per-party rail colors. partySize >= 2 marks a premade; hashing partyId keeps
// distinct parties visually distinct.
const PARTY_COLORS = ["#caa05c", "#5fb392", "#6486b5", "#c08bff", "#d06a64"];
const SMURF_THRESHOLD = 50;
// An enemy with a very high recent K/D also trips the danger styling.
const DANGER_KD = 1.7;

function partyColor(partyId: string): string {
  let hash = 0;
  for (let i = 0; i < partyId.length; i++) {
    hash = (hash * 31 + partyId.charCodeAt(i)) | 0;
  }
  return PARTY_COLORS[Math.abs(hash) % PARTY_COLORS.length];
}

// "GameName#TagLine" -> name + tag (tag keeps its leading #). Hidden players
// show "Hidden" with no tag.
function splitName(row: Row): { name: string; tag: string } {
  if (row.hiddenName) return { name: "Hidden", tag: "" };
  const i = row.name.lastIndexOf("#");
  if (i < 0) return { name: row.name || "Unknown", tag: "" };
  return { name: row.name.slice(0, i), tag: row.name.slice(i) };
}

// During agent select an ally is either hovering an agent (dimmed) or locked in
// (full, with a brief lock-in pulse). Keying the img on the icon url remounts it
// when the pick changes, so switching hover Reyna -> Clove cross-fades. Outside
// agent select (selecting=false) the tile renders plain.
function AgentTile({
  agent,
  icon,
  selecting,
  locked,
}: {
  agent: string;
  icon: string;
  selecting: boolean;
  locked: boolean;
}) {
  const meta = agentMeta(agent);
  const cls = ["agent", selecting && !locked ? "hovering" : "", selecting && locked ? "locked" : ""]
    .filter(Boolean)
    .join(" ");
  return (
    <span className={cls}>
      <span className="mg" style={{ color: meta.color }}>
        {meta.mono}
      </span>
      {icon && <img key={icon} src={icon} alt="" />}
    </span>
  );
}

// Show the real emblem when we have it; fall back to the colored chip only when
// there is no icon, so the chip never sits behind a transparent emblem png.
function Emblem({ tier, icon, className }: { tier: number; icon: string; className?: string }) {
  return (
    <span className={`emb${className ? ` ${className}` : ""}`}>
      {icon ? (
        <img src={icon} alt="" />
      ) : (
        <span className="chip" style={{ background: divColor(tier) }} />
      )}
    </span>
  );
}

export function PlayerRow({
  row,
  isEnemy,
  avgTier,
  dm,
  picking,
  selecting,
  combatLoading,
}: {
  row: Row;
  isEnemy: boolean;
  avgTier: number;
  dm: boolean;
  picking: boolean;
  selecting: boolean;
  combatLoading: boolean;
}) {
  const { name, tag } = splitName(row);
  const ranked = row.rankTier > 0;
  // Combat stats arrive in a second pass; show one spinner per player while this
  // row is still pending, rather than a dot in each stat field.
  const loadingCombat = combatLoading && !row.hasCombat;
  const kd = kdOf(row.lastKills, row.lastDeaths);
  const isSmurf = row.smurfScore >= SMURF_THRESHOLD;
  const danger = isSmurf || (isEnemy && row.hasCombat && kd >= DANGER_KD);
  const stack = row.partySize >= 2;
  const party = stack ? partyColor(row.partyId) : undefined;
  const hasPeak = Boolean(row.peakRankName) && row.peakRankName !== "Unranked";

  // Rank disparity vs the lobby average steers the rank-name tone.
  const diff = row.rankTier - avgTier;
  let rankTone = "tone-neutral";
  if (diff >= 2.2) rankTone = isEnemy ? "tone-bad" : "tone-good";
  else if (diff <= -2.2) rankTone = isEnemy ? "tone-good" : "tone-bad";

  const side = dm ? "dm" : isEnemy ? "enemy" : "ally";
  const cls = ["prow", side, stack ? "party" : "", danger ? "danger" : "", picking ? "picking" : ""]
    .filter(Boolean)
    .join(" ");
  const style = party ? ({ "--party-rail": party } as CSSProperties) : undefined;
  const seenTitle =
    row.encounters > 0
      ? `Seen ${row.encounters} times, ${row.encounterWins}-${row.encounterLosses}`
      : undefined;

  return (
    <div className={cls} style={style}>
      {row.playerCard && (
        <span className="cardbg">
          <img src={row.playerCard} alt="" />
        </span>
      )}
      <div />
      <div className="cell">
        <AgentTile agent={row.agent} icon={row.agentIcon} selecting={selecting} locked={row.locked} />
      </div>

      <div className="cell pid">
        <div className="top">
          <span className="name">
            {name}
            {tag && <span className="ttag">{tag}</span>}
          </span>
          {stack && <span className="partylink">PARTY {row.partySize}</span>}
        </div>
        <div className="bot">
          {row.accountLevel > 0 && (
            <span className="lvl">
              <b>LVL</b> {row.accountLevel}
            </span>
          )}
          {isSmurf && <span className="badge smurf">Smurf?</span>}
          {row.recentWins + row.recentLosses > 0 && (
            <span
              className={`badge ${row.recentWins >= row.recentLosses ? "streak-good" : "streak-bad"}`}
              title={
                row.streak !== 0
                  ? `${Math.abs(row.streak)} ${row.streak < 0 ? "loss" : "win"} streak, recent record ${row.recentWins}-${row.recentLosses}`
                  : `recent record ${row.recentWins}-${row.recentLosses}`
              }
            >
              {row.recentWins}-{row.recentLosses}
            </span>
          )}
          {row.encounters > 0 && (
            <span className="badge seen" title={seenTitle}>
              seen x{row.encounters}
            </span>
          )}
        </div>
      </div>

      {/* Equipped Vandal skin: tier color drives the left accent stripe. */}
      <div className="cell vandal">
        {row.vandalSkin ? (
          <>
            <span
              className="skinthumb"
              style={row.vandalTierColor ? ({ "--ed": row.vandalTierColor } as CSSProperties) : undefined}
            >
              {row.vandalImage && <img src={row.vandalImage} alt="" />}
            </span>
            <span className="skname">{row.vandalSkin}</span>
          </>
        ) : (
          <span className="skname faint">Default</span>
        )}
      </div>

      <div className="cell rank">
        <Emblem tier={row.rankTier} icon={row.rankIcon} />
        <div className="nm">
          <b className={rankTone}>{ranked ? row.rankName : "Unranked"}</b>
          {row.leaderboard > 0 && <span className="lb"> #{row.leaderboard}</span>}
        </div>
      </div>

      <div className="cell num tone-neutral">
        {ranked ? (
          <>
            {row.rr}
            <span className="u">RR</span>
          </>
        ) : (
          <span className="faint">&middot;</span>
        )}
      </div>

      {/* Performance capsule: recent ACS/ADR/KAST over K/D/HS/WR in a 2x3 mono
          grid rather than six columns. ACS leads as the headline number and
          carries the single per-row loading spinner while combat streams in. */}
      <div className="cell form">
        <div className="caps">
          <span className={`s ${row.hasCombat ? tone(row.lastAcs, 170, 220, isEnemy) : "tone-neutral"}`}>
            <i>ACS</i>
            {row.hasCombat ? (
              row.lastAcs
            ) : loadingCombat ? (
              <span className="stat-spin" />
            ) : (
              <em>&middot;</em>
            )}
          </span>
          <span className={`s ${row.hasCombat ? tone(row.lastAdr, 120, 150, isEnemy) : "tone-neutral"}`}>
            <i>ADR</i>
            {row.hasCombat ? row.lastAdr : <em>&middot;</em>}
          </span>
          <span className={`s ${row.hasCombat ? tone(row.lastKast, 62, 72, isEnemy) : "tone-neutral"}`}>
            <i>KAST</i>
            {row.hasCombat ? `${row.lastKast}%` : <em>&middot;</em>}
          </span>
          <span className={`s ${row.hasCombat ? tone(kd, 0.85, 1.1, isEnemy) : "tone-neutral"}`}>
            <i>K/D</i>
            {row.hasCombat ? kd.toFixed(2) : <em>&middot;</em>}
          </span>
          <span className="s">
            <i>HS</i>
            {row.hasCombat ? `${row.lastHs}%` : <em>&middot;</em>}
          </span>
          <span className={`s ${row.games > 0 ? tone(row.winRate, 45, 55, isEnemy) : "tone-neutral"}`}>
            <i>WR</i>
            {row.games > 0 ? `${row.winRate}%` : <em>&middot;</em>}
          </span>
        </div>
      </div>

      <div className="cell peak">
        {hasPeak ? (
          <>
            <Emblem tier={row.peakRankTier} icon={row.peakRankIcon} />
            <div className="pk">
              <b>{row.peakRankName}</b>
              {row.peakAct && <span className="act">peaked {row.peakAct}</span>}
            </div>
          </>
        ) : (
          <span className="faint">&middot;</span>
        )}
      </div>
    </div>
  );
}
