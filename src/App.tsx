import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { MatchView, MatchState } from "./types";
import { StatusScreen } from "./components/StatusScreen";
import { PlayerTable } from "./components/PlayerTable";
import { ProfileCard } from "./components/ProfileCard";
import { HistoryStrip } from "./components/HistoryStrip";

const INITIAL: MatchView = {
  state: "NoGame",
  mode: "",
  players: [],
  me: null,
  history: [],
  stale: false,
};

const STATE_LABEL: Record<MatchState, string> = {
  NoGame: "Offline",
  Menu: "Menu",
  PreGame: "Agent Select",
  CoreGame: "Live",
};

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
  const live = view.state === "CoreGame" || view.state === "PreGame";
  const win = getCurrentWindow();

  return (
    <div className="app">
      <header className="titlebar" data-tauri-drag-region>
        <span className="wordmark" data-tauri-drag-region>
          PEE<span className="wordmark-accent">K</span>
        </span>
        <span className={`state-pill ${live ? "state-live" : ""}`}>
          <span className="state-dot" />
          {STATE_LABEL[view.state]}
        </span>
        {view.mode && live && <span className="mode-pill">{view.mode}</span>}
        {view.stale && <span className="stale">stale</span>}
        <span className="win-controls">
          <button
            className="win-btn"
            onClick={() => win.minimize()}
            aria-label="Minimize"
            title="Minimize"
          >
            &#x2013;
          </button>
          <button
            className="win-btn"
            onClick={() => win.hide()}
            aria-label="Minimize to tray"
            title="Minimize to tray"
          >
            &#x2304;
          </button>
          <button
            className="win-btn win-close"
            onClick={() => win.close()}
            aria-label="Close"
            title="Close"
          >
            &#x2715;
          </button>
        </span>
      </header>
      <main className="app-body">
        {showTable ? (
          <PlayerTable players={view.players} />
        ) : (
          <div className="idle">
            {view.me && <ProfileCard me={view.me} />}
            <HistoryStrip history={view.history} />
            <StatusScreen state={view.state} />
          </div>
        )}
      </main>
    </div>
  );
}
