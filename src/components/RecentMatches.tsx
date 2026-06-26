import { HistoryEntry } from "../types";
import { divColor, kdOf } from "../design";

const MAX = 9;

function dropImg(e: React.SyntheticEvent<HTMLImageElement>) {
  e.currentTarget.remove();
}

function MapThumb({ map, image }: { map: string; image: string }) {
  const ab = (map || "MAP").slice(0, 3).toUpperCase();
  return (
    <span className="mthumb">
      <span className="ab">{ab}</span>
      {image && <img src={image} alt="" onError={dropImg} />}
    </span>
  );
}

export function RecentMatches({ history }: { history: HistoryEntry[] }) {
  if (history.length === 0) return null;

  const shown = history.slice(0, MAX);
  let wins = 0;
  let losses = 0;
  let net = 0;
  for (const h of shown) {
    const won = h.hasStats ? h.won : h.rrChange >= 0;
    won ? wins++ : losses++;
    net += h.rrChange;
  }
  // Form dots read oldest -> newest, left to right.
  const form = [...shown].reverse();

  return (
    <div className="matches">
      <div className="mhdr">
        <h3>Recent Competitive</h3>
        <span className="sm">
          <span className="gp">{wins}W</span> <span className="lp">{losses}L</span> &middot;{" "}
          {net > 0 ? "+" : ""}
          {net} RR
        </span>
        <div className="form">
          {form.map((h, i) => {
            const won = h.hasStats ? h.won : h.rrChange >= 0;
            return <i key={i} className={won ? "w" : "l"} />;
          })}
        </div>
      </div>

      <div className="mlist">
        {shown.map((h, i) => {
          const won = h.hasStats ? h.won : h.rrChange >= 0;
          const kd = kdOf(h.kills, h.deaths);
          return (
            <div key={i} className={`mrow ${won ? "win" : "loss"}`} style={{ animationDelay: `${i * 42}ms` }}>
              <div className="acc" />
              <div className="cell">
                <MapThumb map={h.map} image={h.mapImage} />
              </div>
              <div className="cell">
                <div className="mp">{h.map || "Match"}</div>
                {/* TODO: per-match mode and timestamp are not in history; show
                    the rank at the time instead. */}
                <div className="mt" style={{ color: divColor(h.tier) }}>
                  {h.rankName || "Competitive"}
                </div>
              </div>
              <div className="cell">
                <div className="res">{won ? "WIN" : "LOSS"}</div>
                {h.hasStats && (
                  <div className="sc">
                    {h.selfRounds}-{h.enemyRounds}
                  </div>
                )}
              </div>
              <div className="cell">
                <span className="agent">
                  {h.agentIcon && <img src={h.agentIcon} alt="" onError={(e) => e.currentTarget.remove()} />}
                </span>
              </div>
              <div className="cell">
                {h.hasStats ? (
                  <div className="kda">
                    {h.kills}
                    <span className="s">/</span>
                    {h.deaths}
                    <span className="s">/</span>
                    {h.assists}
                    <span className="small">{h.hs}% HS &middot; {kd.toFixed(2)} KD</span>
                  </div>
                ) : (
                  <div className="kda faint">&middot;</div>
                )}
              </div>
              <div className="cell rrc">
                {h.rrChange > 0 ? "+" : ""}
                {h.rrChange}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
