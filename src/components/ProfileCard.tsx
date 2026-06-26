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

      {ranked && me.rankIcon ? (
        <img
          className="profile-emblem"
          src={me.rankIcon}
          alt={me.rankName}
          style={{ filter: tierGlow(me.rankTier) ? `drop-shadow(0 0 16px ${color}aa)` : undefined }}
        />
      ) : (
        <div className="profile-rank" style={{ color, borderColor: color }}>
          {ranked ? me.rankName : "Unranked"}
        </div>
      )}

      <div className="profile-rankname" style={{ color }}>
        {ranked ? me.rankName : "Unranked"}
        {me.leaderboard > 0 && <span className="lb"> #{me.leaderboard}</span>}
      </div>

      <div className="profile-stats">
        <span>
          <b style={{ fontVariantNumeric: "tabular-nums" }}>{ranked ? me.rr : "--"}</b> RR
        </span>
        {me.games > 0 && (
          <span>
            <b>
              {me.wins}W {me.games - me.wins}L
            </b>{" "}
            <span className="dim">({me.winRate}%)</span>
          </span>
        )}
        {hasPeak && (
          <span>
            peak <b style={{ color: tierColor(me.peakRankTier) }}>{me.peakRankName}</b>
          </span>
        )}
      </div>
    </div>
  );
}
