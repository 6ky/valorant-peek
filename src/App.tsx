import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { MatchView, MatchState } from "./types";
import { StatusScreen } from "./components/StatusScreen";
import { PlayerTable } from "./components/PlayerTable";
import { ProfileCard } from "./components/ProfileCard";
import { RecentMatches } from "./components/RecentMatches";
import { CloseDialog } from "./components/CloseDialog";
import { Settings } from "./components/Settings";

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
  const [exiting, setExiting] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const win = getCurrentWindow();

  // Apply persisted settings on startup.
  useEffect(() => {
    win.setAlwaysOnTop(localStorage.getItem("peek.alwaysOnTop") !== "false");
    invoke("set_rpc_enabled", {
      enabled: localStorage.getItem("peek.rpcEnabled") !== "false",
    });
  }, []);

  function performExit(action: "tray" | "quit") {
    setExiting(true);
    window.setTimeout(() => {
      if (action === "quit") {
        win.close();
      } else {
        win.hide();
        setExiting(false);
      }
    }, 200);
  }

  function onCloseClick() {
    const remembered = localStorage.getItem(CLOSE_PREF_KEY);
    if (remembered === "tray") return performExit("tray");
    if (remembered === "quit") return performExit("quit");
    setAskClose(true);
  }

  function resolveClose(action: "tray" | "quit", remember: boolean) {
    if (remember) localStorage.setItem(CLOSE_PREF_KEY, action);
    setAskClose(false);
    performExit(action);
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
    <div className={`app${exiting ? " exiting" : ""}`}>
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
            onClick={() => setShowSettings(true)}
            aria-label="Settings"
            title="Settings"
          >
            &#x2699;
          </button>
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
            <div className="idle-inner">
              {view.me && <ProfileCard me={view.me} />}
              <RecentMatches history={view.history} />
              <StatusScreen state={view.state} />
            </div>
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
      {showSettings && <Settings onClose={() => setShowSettings(false)} />}
    </div>
  );
}
