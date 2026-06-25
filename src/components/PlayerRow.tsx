import { PlayerRow as Row } from "../types";

const PARTY_COLORS = ["#f0a", "#0bf", "#fb0", "#5d5", "#a7f"];

function partyColor(partyId: string): string | undefined {
  if (!partyId) return undefined;
  let hash = 0;
  for (let i = 0; i < partyId.length; i++) {
    hash = (hash * 31 + partyId.charCodeAt(i)) | 0;
  }
  return PARTY_COLORS[Math.abs(hash) % PARTY_COLORS.length];
}

export function PlayerRow({ row }: { row: Row }) {
  const color = partyColor(row.partyId);
  return (
    <tr style={color ? { borderLeft: `3px solid ${color}` } : undefined}>
      <td className="name">{row.name || "Unknown"}</td>
      <td>{row.agent}</td>
      <td>{row.rankName}</td>
      <td className="rr">{row.rankTier > 0 ? `${row.rr} RR` : ""}</td>
      <td className="peak">{row.peakRankName}</td>
      <td className="level">{row.accountLevel > 0 ? row.accountLevel : ""}</td>
    </tr>
  );
}
