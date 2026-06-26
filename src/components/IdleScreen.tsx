import { PlayerRow, HistoryEntry } from "../types";
import { ProfileCard } from "./ProfileCard";
import { RecentMatches } from "./RecentMatches";

export function IdleScreen({
  me,
  history,
}: {
  me: PlayerRow | null;
  history: HistoryEntry[];
}) {
  return (
    <div className="view on">
      <div className="idle">
        {me ? <ProfileCard me={me} history={history} /> : <div className="profile" />}
        <RecentMatches history={history} />
      </div>
    </div>
  );
}
