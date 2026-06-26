import { MatchState, HistoryEntry } from "../types";
import peekLogo from "../assets/peek.svg";

// The VALORANT logo is trademarked, so it is not bundled in this repo. Point
// this at a hosted copy to show it on standby; it falls back to the Peek mark
// when unset or unreachable.
const VALORANT_LOGO_URL = "";

// The standby screen: shown when VALORANT is not running (NoGame). state still
// drives the live-status rows. history (when present) powers the last-session
// summary in the footer.
export function StatusScreen({
  state,
  history = [],
}: {
  state: MatchState;
  history?: HistoryEntry[];
}) {
  // This screen only renders on NoGame, which means the Riot Client lockfile is
  // absent, so the client is not running. Reflect that honestly.
  const riotRunning = state !== "NoGame";
  const valorantRunning = state === "PreGame" || state === "CoreGame";

  let wins = 0;
  let losses = 0;
  let net = 0;
  for (const h of history) {
    const won = h.hasStats ? h.won : h.rrChange >= 0;
    won ? wins++ : losses++;
    net += h.rrChange;
  }

  return (
    <div className="view on">
      <div className="standby">
        <div className="sb-hero">
          <div className="sb-mark">
            <img
              className="sb-logo"
              src={VALORANT_LOGO_URL || peekLogo}
              alt=""
              onError={(e) => {
                const img = e.currentTarget;
                if (img.dataset.fallback) return;
                img.dataset.fallback = "1";
                img.src = peekLogo;
              }}
            />
          </div>
          <div className="sb-title">Waiting for VALORANT</div>
          <div className="sb-sub">
            Launch the game and Peek connects automatically. Your lobby appears here the moment
            agent select loads.
          </div>
          <div className="sb-status">
            <div className={`sb-row ${riotRunning ? "ok" : "off"}`}>
              <span className="d" />
              <span className="l">Riot Client</span>
              <span className="v">{riotRunning ? "Connected" : "Not running"}</span>
            </div>
            <div className={`sb-row ${valorantRunning ? "ok" : "off"}`}>
              <span className="d" />
              <span className="l">VALORANT</span>
              <span className="v">{valorantRunning ? "Running" : "Not running"}</span>
            </div>
            <div className={`sb-row ${valorantRunning ? "ok" : "wait"}`}>
              <span className="d" />
              <span className="l">Match listener</span>
              <span className="v">{valorantRunning ? "Live" : "Standby"}</span>
            </div>
          </div>
        </div>
        <div className="sb-foot">
          <div className="sb-fcol">
            <span className="k">Last session</span>
            <b>
              {history.length > 0 ? (
                <>
                  {wins}W {losses}L &middot; {net > 0 ? "+" : ""}
                  {net} RR
                </>
              ) : (
                "No recent games"
              )}
            </b>
          </div>
          <div className="sb-tip">
            Peek stays pinned and out of the way, then wakes on its own when a match is found.
          </div>
          {/* TODO: app version is not plumbed to the frontend yet. */}
          <div className="sb-ver mono">Peek</div>
        </div>
      </div>
    </div>
  );
}
