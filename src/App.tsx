import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { MatchView, MatchState } from "./types";
import { MatchScreen } from "./components/MatchScreen";
import { IdleScreen } from "./components/IdleScreen";
import { StatusScreen } from "./components/StatusScreen";
import { CloseDialog } from "./components/CloseDialog";
import { Settings } from "./components/Settings";

const CLOSE_PREF_KEY = "peek.closeAction";

const INITIAL: MatchView = {
  state: "NoGame",
  mode: "",
  activity: "",
  players: [],
  me: null,
  history: [],
  stale: false,
  phaseTime: 0,
  map: "",
  mapImage: "",
  allyScore: 0,
  enemyScore: 0,
  combatLoading: false,
  historyQueue: 0,
};

const STATE_LABEL: Record<MatchState, string> = {
  NoGame: "Offline",
  Menu: "Idle",
  PreGame: "Agent Select",
  CoreGame: "In match",
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
    invoke("set_combat_enabled", {
      enabled: localStorage.getItem("peek.combat") !== "false",
    });
    const hq = Number(localStorage.getItem("peek.historyQueue"));
    invoke("set_history_queue", { queue: hq === 1 || hq === 2 ? hq : 0 });
    // The window starts hidden so the dark UI is painted before it appears,
    // avoiding a white flash. Reveal it on the next frame.
    requestAnimationFrame(() => win.show());
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

  const showMatch =
    (view.state === "CoreGame" || view.state === "PreGame") && view.players.length > 0;
  const showStandby = !showMatch && view.state === "NoGame";

  // Status dot/label. Live and idle read green; standby reads faint. Prefixed
  // so the state class cannot collide with layout classes like .idle.
  const statusKind = showStandby ? "s-off" : showMatch ? "s-live" : "s-idle";
  const statusLabel = view.activity || STATE_LABEL[view.state];

  return (
    <div className={`app${exiting ? " exiting" : ""}`}>
      <div className="titlebar" data-tauri-drag-region>
        <div className="wordmark" data-tauri-drag-region>
          <span className="peek-name">PEEK</span>
        </div>
        <div className="status">
          <span className={`dot ${statusKind}`} />
          <span className="lbl">{statusLabel}</span>
          {view.stale && <span className="stale">stale</span>}
        </div>
        <div className="wins">
          <button
            className="wbtn"
            title="Settings"
            aria-label="Settings"
            onClick={() => setShowSettings(true)}
          >
            <svg width="13" height="13" viewBox="0 0 13 13">
              <line x1="2" y1="3.5" x2="11" y2="3.5" stroke="currentColor" strokeWidth="1.3" />
              <line x1="2" y1="9.5" x2="11" y2="9.5" stroke="currentColor" strokeWidth="1.3" />
              <circle cx="8" cy="3.5" r="1.7" fill="var(--bg)" stroke="currentColor" strokeWidth="1.3" />
              <circle cx="5" cy="9.5" r="1.7" fill="var(--bg)" stroke="currentColor" strokeWidth="1.3" />
            </svg>
          </button>
          <button className="wbtn" title="Minimize" aria-label="Minimize" onClick={() => win.minimize()}>
            <svg width="12" height="12" viewBox="0 0 12 12">
              <line x1="2.5" y1="6" x2="9.5" y2="6" stroke="currentColor" strokeWidth="1.4" />
            </svg>
          </button>
          <button
            className="wbtn close"
            title="Close"
            aria-label="Close"
            onClick={onCloseClick}
          >
            <svg width="12" height="12" viewBox="0 0 12 12">
              <line x1="3" y1="3" x2="9" y2="9" stroke="currentColor" strokeWidth="1.4" />
              <line x1="9" y1="3" x2="3" y2="9" stroke="currentColor" strokeWidth="1.4" />
            </svg>
          </button>
        </div>
      </div>

      {showMatch ? (
        <MatchScreen view={view} />
      ) : showStandby ? (
        <StatusScreen state={view.state} history={view.history} />
      ) : (
        <IdleScreen me={view.me} history={view.history} historyQueue={view.historyQueue} />
      )}

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
