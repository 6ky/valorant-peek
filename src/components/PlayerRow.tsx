import { PlayerRow as Row } from "../types";
import { tierColor, tierGlow } from "../rank";

const PARTY_COLORS = ["#ff8a3d", "#3dd6ff", "#ffd23d", "#5dff9b", "#c08bff"];

function partyColor(partyId: string): string | undefined {
  if (!partyId) return undefined;
  let hash = 0;
  for (let i = 0; i < partyId.length; i++) {
    hash = (hash * 31 + partyId.charCodeAt(i)) | 0;
  }
  return PARTY_COLORS[Math.abs(hash) % PARTY_COLORS.length];
}

export function PlayerRow({ row }: { row: Row }) {
  const color = tierColor(row.rankTier);
  const ranked = row.rankTier > 0;
  const party = partyColor(row.partyId);
  const hasPeak = row.peakRankName && row.peakRankName !== "Unranked";

  return (
    <div className="prow">
      <span
        className="party"
        style={{ background: party ?? "transparent" }}
        title={party ? "In a party" : undefined}
      />
      {row.agentIcon ? (
        <img className="agent-ico" src={row.agentIcon} alt={row.agent} title={row.agent} />
      ) : (
        <span className="agent-ico empty" />
      )}
      {ranked && row.rankIcon ? (
        <img
          className="rank-ico"
          src={row.rankIcon}
          alt={row.rankName}
          title={row.rankName}
          style={{ filter: tierGlow(row.rankTier) ? `drop-shadow(0 0 6px ${color}99)` : undefined }}
        />
      ) : (
        <span className="rank-ico-fallback" style={{ background: color }} title={row.rankName} />
      )}
      <span className="prow-id">
        <span className={`prow-name${row.hiddenName ? " hidden" : ""}`}>
          {row.hiddenName ? "Hidden" : row.name || "Unknown"}
        </span>
        <span className="prow-meta">
          <span style={{ color }}>{ranked ? row.rankName : "Unranked"}</span>
          {row.leaderboard > 0 && (
            <>
              <span className="dot-sep">&middot;</span>
              <span className="lb">#{row.leaderboard}</span>
            </>
          )}
          {row.games > 0 && (
            <>
              <span className="dot-sep">&middot;</span>
              <span>
                {row.wins}W {row.games - row.wins}L ({row.winRate}%)
              </span>
            </>
          )}
          {hasPeak && (
            <>
              <span className="dot-sep">&middot;</span>
              <span>
                peak <span style={{ color: tierColor(row.peakRankTier) }}>{row.peakRankName}</span>
              </span>
            </>
          )}
        </span>
      </span>
      <span className="prow-stats">
        <span className="prow-rr">
          {ranked ? row.rr : "--"}
          <span className="unit">rr</span>
        </span>
        <span className="prow-level">{row.accountLevel > 0 ? `lvl ${row.accountLevel}` : ""}</span>
      </span>
    </div>
  );
}
