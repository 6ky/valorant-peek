import { HistoryEntry } from "../types";

export function HistoryStrip({ history }: { history: HistoryEntry[] }) {
  if (history.length === 0) return null;
  const net = history.reduce((sum, h) => sum + h.rrChange, 0);

  return (
    <div className="history">
      <div className="history-head">
        <span>Recent competitive</span>
        <span className={net >= 0 ? "pos" : "neg"}>
          {net >= 0 ? "+" : ""}
          {net} RR
        </span>
      </div>
      <div className="history-row">
        {history.map((h, i) => {
          const win = h.rrChange >= 0;
          return (
            <span
              key={i}
              className={`hpill ${win ? "pos" : "neg"}`}
              title={h.map ? `${h.map} (${h.rankName})` : h.rankName}
            >
              {win ? "+" : ""}
              {h.rrChange}
            </span>
          );
        })}
      </div>
    </div>
  );
}
