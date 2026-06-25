import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { MatchView } from "./types";
import { StatusScreen } from "./components/StatusScreen";
import { PlayerTable } from "./components/PlayerTable";

const INITIAL: MatchView = { state: "NoGame", players: [], stale: false };

export default function App() {
  const [view, setView] = useState<MatchView>(INITIAL);

  useEffect(() => {
    const unlisten = listen<MatchView>("match-view", (e) => setView(e.payload));
    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  const showTable =
    (view.state === "CoreGame" || view.state === "PreGame") && view.players.length > 0;

  return (
    <div className="app">
      <header className="app-header">
        <span className="title">val-companion</span>
        {view.stale && <span className="stale">stale</span>}
      </header>
      {showTable ? (
        <PlayerTable players={view.players} />
      ) : (
        <StatusScreen state={view.state} />
      )}
    </div>
  );
}
