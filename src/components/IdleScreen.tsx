import { PlayerRow, HistoryEntry } from "../types";
import { ProfileCard } from "./ProfileCard";
import { RecentMatches } from "./RecentMatches";

export function IdleScreen({
  me,
  history,
  historyQueue,
}: {
  me: PlayerRow | null;
  history: HistoryEntry[];
  historyQueue: number;
}) {
  return (
    <div className="view on">
      <div className="idle">
        {me ? <ProfileCard me={me} history={history} /> : <div className="profile" />}
        <RecentMatches history={history} historyQueue={historyQueue} />
      </div>
    </div>
  );
}
