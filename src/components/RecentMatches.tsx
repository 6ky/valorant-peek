import { useState } from "react";
import { HistoryEntry } from "../types";
import { tierColor } from "../rank";

const MAX = 6;

export function RecentMatches({ history }: { history: HistoryEntry[] }) {
  const [open, setOpen] = useState<number | null>(null);
  if (history.length === 0) return null;

  const shown = history.slice(0, MAX);
  const net = shown.reduce((sum, h) => sum + h.rrChange, 0);

  return (
    <div className="matches">
      <div className="matches-head">
        <span>Recent Competitive</span>
        <span className="matches-net">
          net{" "}
          <span className={net >= 0 ? "pos" : "neg"}>
            {net >= 0 ? "+" : ""}
            {net} RR
          </span>
        </span>
      </div>
      <div className="match-list">
        {shown.map((h, i) => {
          const won = h.hasStats ? h.won : h.rrChange >= 0;
          const expanded = open === i;
          const kd = h.deaths > 0 ? (h.kills / h.deaths).toFixed(2) : h.kills.toFixed(2);
          return (
            <div key={i} className={`match-row ${won ? "win" : "loss"} ${expanded ? "open" : ""}`}>
              <button className="match-main" onClick={() => setOpen(expanded ? null : i)}>
                <span className="match-result">{won ? "W" : "L"}</span>
                {h.agentIcon ? (
                  <img className="match-agent" src={h.agentIcon} alt="" />
                ) : (
                  <span className="match-agent empty" />
                )}
                <span className="match-map">{h.map || "Match"}</span>
                {h.hasStats && (
                  <span className="match-kda">
                    {h.kills}
                    <span className="slash">/</span>
                    {h.deaths}
                    <span className="slash">/</span>
                    {h.assists}
                  </span>
                )}
                <span className={`match-rr ${h.rrChange >= 0 ? "pos" : "neg"}`}>
                  {h.rrChange >= 0 ? "+" : ""}
                  {h.rrChange}
                </span>
              </button>
              {expanded && (
                <div className="match-detail">
                  {h.hasStats && (
                    <div className="md-summary">
                      <span>
                        <b>
                          {h.selfRounds}-{h.enemyRounds}
                        </b>{" "}
                        score
                      </span>
                      <span>
                        <b>{kd}</b> K/D
                      </span>
                      <span>
                        <b>{h.acs}</b> ACS
                      </span>
                      <span style={{ color: tierColor(h.tier) }}>{h.rankName}</span>
                    </div>
                  )}
                  {h.scoreboard.length > 0 && (
                    <div className="scoreboard">
                      {h.scoreboard.map((p, j) => {
                        const divider = j > 0 && h.scoreboard[j - 1].ally !== p.ally;
                        return (
                          <div key={j}>
                            {divider && <div className="sb-divider" />}
                            <div
                              className={`sb-row ${p.ally ? "ally" : "enemy"} ${
                                p.isSelf ? "self" : ""
                              }`}
                            >
                              {p.agentIcon ? (
                                <img className="sb-agent" src={p.agentIcon} alt="" />
                              ) : (
                                <span className="sb-agent empty" />
                              )}
                              <span className="sb-name">{p.name || "Hidden"}</span>
                              <span className="sb-kda">
                                {p.kills}/{p.deaths}/{p.assists}
                              </span>
                              <span className="sb-acs">{p.acs}</span>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
