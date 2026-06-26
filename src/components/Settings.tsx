import { useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";

type CloseAction = "ask" | "tray" | "quit";

function Toggle({ on, onChange }: { on: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      className={`toggle ${on ? "on" : ""}`}
      role="switch"
      aria-checked={on}
      onClick={() => onChange(!on)}
    >
      <span className="toggle-knob" />
    </button>
  );
}

export function Settings({ onClose }: { onClose: () => void }) {
  const [closeAction, setCloseAction] = useState<CloseAction>(
    (localStorage.getItem("peek.closeAction") as CloseAction) || "ask"
  );
  const [onTop, setOnTop] = useState(localStorage.getItem("peek.alwaysOnTop") !== "false");
  const [rpc, setRpc] = useState(localStorage.getItem("peek.rpcEnabled") !== "false");

  function changeClose(v: CloseAction) {
    setCloseAction(v);
    if (v === "ask") localStorage.removeItem("peek.closeAction");
    else localStorage.setItem("peek.closeAction", v);
  }

  function changeOnTop(v: boolean) {
    setOnTop(v);
    localStorage.setItem("peek.alwaysOnTop", String(v));
    getCurrentWindow().setAlwaysOnTop(v);
  }

  function changeRpc(v: boolean) {
    setRpc(v);
    localStorage.setItem("peek.rpcEnabled", String(v));
    invoke("set_rpc_enabled", { enabled: v });
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal settings" onClick={(e) => e.stopPropagation()}>
        <div className="settings-head">
          <span className="modal-title">Settings</span>
          <button className="win-btn" onClick={onClose} aria-label="Close settings">
            &#x2715;
          </button>
        </div>

        <div className="setting">
          <div className="setting-label">Close button</div>
          <div className="setting-desc">What the X button does</div>
          <div className="seg">
            {(["ask", "tray", "quit"] as CloseAction[]).map((v) => (
              <button
                key={v}
                className={`seg-btn ${closeAction === v ? "active" : ""}`}
                onClick={() => changeClose(v)}
              >
                {v === "ask" ? "Ask" : v === "tray" ? "Tray" : "Quit"}
              </button>
            ))}
          </div>
        </div>

        <div className="setting">
          <div className="setting-row">
            <div>
              <div className="setting-label">Always on top</div>
              <div className="setting-desc">Keep Peek above other windows</div>
            </div>
            <Toggle on={onTop} onChange={changeOnTop} />
          </div>
        </div>

        <div className="setting">
          <div className="setting-row">
            <div>
              <div className="setting-label">Discord Rich Presence</div>
              <div className="setting-desc">Show your Peek status on Discord</div>
            </div>
            <Toggle on={rpc} onChange={changeRpc} />
          </div>
        </div>
      </div>
    </div>
  );
}
