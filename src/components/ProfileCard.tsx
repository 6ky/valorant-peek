import { PlayerRow } from "../types";
import { tierColor, tierGlow } from "../rank";

export function ProfileCard({ me }: { me: PlayerRow }) {
  const color = tierColor(me.rankTier);
  const ranked = me.rankTier > 0;
  const hasPeak = me.peakRankName && me.peakRankName !== "Unranked";

  return (
    <div className="profile">
      <div className="profile-head">
        <span className="profile-name">{me.name || "You"}</span>
        {me.accountLevel > 0 && <span className="profile-level">Level {me.accountLevel}</span>}
      </div>
      <div
        className="profile-rank"
        style={{
          color,
          borderColor: color,
          boxShadow: tierGlow(me.rankTier) ? `0 0 18px ${color}55` : undefined,
        }}
      >
        {ranked ? me.rankName : "Unranked"}
      </div>
      <div className="profile-stats">
        <span>
          <b style={{ fontVariantNumeric: "tabular-nums" }}>{ranked ? me.rr : "--"}</b> RR
        </span>
        {hasPeak && (
          <span>
            peak <b style={{ color: tierColor(me.peakRankTier) }}>{me.peakRankName}</b>
          </span>
        )}
      </div>
    </div>
  );
}
