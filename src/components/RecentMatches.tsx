import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { HistoryEntry } from "../types";
import { divColor } from "../design";

const MAX = 15;

// Recent-matches queue: index matches the backend's history_queue values.
const QUEUES = [
  { id: 0, label: "Competitive" },
  { id: 1, label: "Unrated" },
  { id: 2, label: "All" },
];

function MapThumb({ map, image }: { map: string; image: string }) {
  const ab = (map || "MAP").slice(0, 3).toUpperCase();
  return (
    <span className="mthumb">
      <span className="ab">{ab}</span>
      {image && <img src={image} alt="" />}
    </span>
  );
}

export function RecentMatches({
  history,
  historyQueue,
}: {
  history: HistoryEntry[];
  historyQueue: number;
}) {
  const [queue, setQueue] = useState<number>(() => {
    const v = Number(localStorage.getItem("peek.historyQueue"));
    return v === 1 || v === 2 ? v : 0;
  });

  function pick(q: number) {
    setQueue(q);
    localStorage.setItem("peek.historyQueue", String(q));
    invoke("set_history_queue", { queue: q });
  }

  // The list still belongs to the previous queue until the backend refetches,
  // so show a loading state rather than the stale or empty list in between.
  const loading = queue !== historyQueue;
  const shown = history.slice(0, MAX);
  // RR only exists for competitive; the other queues come from match history.
  const showRr = queue === 0;
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
        <div className="mseg">
          {QUEUES.map((q) => (
            <button
              key={q.id}
              className={queue === q.id ? "on" : ""}
              onClick={() => pick(q.id)}
            >
              {q.label}
            </button>
          ))}
        </div>
        {!loading && shown.length > 0 && (
          <span className="sm">
            <span className="gp">{wins}W</span> <span className="lp">{losses}L</span>
            {showRr && (
              <>
                {" "}
                &middot; {net > 0 ? "+" : ""}
                {net} RR
              </>
            )}
          </span>
        )}
        {!loading && (
          <div className="form">
            {form.map((h, i) => {
              const won = h.hasStats ? h.won : h.rrChange >= 0;
              return <i key={i} className={won ? "w" : "l"} />;
            })}
          </div>
        )}
      </div>

      <div className="mlist">
        {loading ? (
          <div className="mempty">Loading {QUEUES[queue].label}...</div>
        ) : shown.length === 0 ? (
          <div className="mempty">No recent games in this mode</div>
        ) : (
          shown.map((h, i) => {
            const won = h.hasStats ? h.won : h.rrChange >= 0;
            return (
              <div key={i} className={`mrow ${won ? "win" : "loss"}`} style={{ animationDelay: `${i * 42}ms` }}>
                <div className="acc" />
                <div className="cell">
                  <MapThumb map={h.map} image={h.mapImage} />
                </div>
                <div className="cell">
                  <div className="mp">{h.map || "Match"}</div>
                  <div className="mt" style={{ color: divColor(h.tier) }}>
                    {h.rankName || "Match"}
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
                      {h.acs > 0 && (
                        <span className="small">
                          {h.acs} ACS &middot; {h.adr} ADR &middot; {h.kast}% KAST &middot; {h.hs}% HS
                        </span>
                      )}
                    </div>
                  ) : (
                    <div className="kda faint">&middot;</div>
                  )}
                </div>
                <div className="cell rrc">
                  {h.ranked ? `${h.rrChange > 0 ? "+" : ""}${h.rrChange}` : ""}
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
