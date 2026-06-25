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
      <span
        className="rank-tag"
        style={{
          color,
          borderColor: color,
          boxShadow: tierGlow(row.rankTier) ? `0 0 12px ${color}55` : undefined,
        }}
      >
        {ranked ? row.rankName : "Unranked"}
      </span>
      <span className="prow-id">
        <span className="prow-name">{row.name || "Unknown"}</span>
        <span className="prow-meta">
          {row.agent && <span className="prow-agent">{row.agent}</span>}
          {row.agent && hasPeak && <span className="dot-sep">&middot;</span>}
          {hasPeak && (
            <span>
              peak <span style={{ color: tierColor(row.peakRankTier) }}>{row.peakRankName}</span>
            </span>
          )}
        </span>
      </span>
      <span className="prow-stats">
        <span className="prow-rr">
          {ranked ? row.rr : "--"}
          <span className="unit">rr</span>
        </span>
        <span className="prow-level">
          {row.accountLevel > 0 ? `lvl ${row.accountLevel}` : ""}
        </span>
      </span>
    </div>
  );
}
