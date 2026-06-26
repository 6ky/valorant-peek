import { useState } from "react";

export function CloseDialog({
  onTray,
  onQuit,
  onCancel,
}: {
  onTray: (remember: boolean) => void;
  onQuit: (remember: boolean) => void;
  onCancel: () => void;
}) {
  const [remember, setRemember] = useState(false);

  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-title">Close Peek?</div>
        <div className="modal-body">Keep it running in the tray, or quit completely.</div>
        <label className="modal-remember">
          <input
            type="checkbox"
            checked={remember}
            onChange={(e) => setRemember(e.target.checked)}
          />
          Remember my choice
        </label>
        <div className="modal-actions">
          <button className="btn" onClick={() => onTray(remember)}>
            Minimize to tray
          </button>
          <button className="btn btn-danger" onClick={() => onQuit(remember)}>
            Quit
          </button>
        </div>
      </div>
    </div>
  );
}
