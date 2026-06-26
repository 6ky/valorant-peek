import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { MatchView, MatchState } from "./types";
import { StatusScreen } from "./components/StatusScreen";
import { PlayerTable } from "./components/PlayerTable";
import { ProfileCard } from "./components/ProfileCard";
import { HistoryStrip } from "./components/HistoryStrip";
import { CloseDialog } from "./components/CloseDialog";

const CLOSE_PREF_KEY = "peek.closeAction";

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
  const [askClose, setAskClose] = useState(false);
  const win = getCurrentWindow();

  function onCloseClick() {
    const remembered = localStorage.getItem(CLOSE_PREF_KEY);
    if (remembered === "tray") return void win.hide();
    if (remembered === "quit") return void win.close();
    setAskClose(true);
  }

  function resolveClose(action: "tray" | "quit", remember: boolean) {
    if (remember) localStorage.setItem(CLOSE_PREF_KEY, action);
    setAskClose(false);
    if (action === "tray") win.hide();
    else win.close();
  }

  useEffect(() => {
    const unlisten = listen<MatchView>("match-view", (e) => setView(e.payload));
    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  const showTable =
    (view.state === "CoreGame" || view.state === "PreGame") && view.players.length > 0;
  const live = view.state === "CoreGame" || view.state === "PreGame";

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
            className="win-btn win-close"
            onClick={onCloseClick}
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
      {askClose && (
        <CloseDialog
          onTray={(remember) => resolveClose("tray", remember)}
          onQuit={(remember) => resolveClose("quit", remember)}
          onCancel={() => setAskClose(false)}
        />
      )}
    </div>
  );
}
