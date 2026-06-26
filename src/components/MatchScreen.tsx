import { useEffect, useState } from "react";
import { MatchView } from "../types";
import { PlayerTable } from "./PlayerTable";

function dropImg(e: React.SyntheticEvent<HTMLImageElement>) {
  e.currentTarget.remove();
}

// m:ss countdown text.
function fmtTime(s: number): string {
  const m = Math.floor(s / 60);
  const sec = s % 60;
  return `${m}:${sec.toString().padStart(2, "0")}`;
}

// The match screen: a context strip over the roster. Comp vs FFA is inferred
// from the team split inside PlayerTable.
export function MatchScreen({ view }: { view: MatchView }) {
  const pregame = view.state === "PreGame";
  const [remaining, setRemaining] = useState(view.phaseTime);

  // Reset on every backend update, then tick down locally between updates. Only
  // agent select carries a phase countdown; in CoreGame there is nothing to show.
  useEffect(() => {
    setRemaining(view.phaseTime);
    if (!pregame || view.phaseTime <= 0) return;
    const id = setInterval(() => {
      setRemaining((r) => (r <= 1 ? 0 : r - 1));
    }, 1000);
    return () => clearInterval(id);
  }, [pregame, view.phaseTime]);

  const showScore = view.allyScore > 0 || view.enemyScore > 0;
  const showClock = pregame && remaining > 0;

  return (
    <div className="view on">
      <div className="ctx">
        <div className="mode">
          <span className="tick" />
          <b>{view.mode || "Match"}</b>
        </div>
        {view.mapImage && (
          <img className="ctxmap" src={view.mapImage} alt="" onError={dropImg} />
        )}
        {view.map && <span className="map">{view.map}</span>}
        {showScore && (
          <div className="score mono">
            <span className="aw">{view.allyScore}</span>
            <span className="sep">:</span>
            <span className="ew">{view.enemyScore}</span>
          </div>
        )}
        {showClock && <span className="phase mono">{fmtTime(remaining)}</span>}
      </div>
      <PlayerTable players={view.players} state={view.state} />
    </div>
  );
}
