import { render, screen } from "@testing-library/react";
import { PlayerTable } from "./PlayerTable";
import { PlayerRow } from "../types";

const rows: PlayerRow[] = [
  {
    puuid: "p1",
    name: "Ace#NA1",
    playerCard: "",
    agent: "Jett",
    agentIcon: "",
    team: "Blue",
    partyId: "x",
    hiddenName: false,
    rankTier: 21,
    rankName: "Immortal 3",
    rankIcon: "",
    rr: 42,
    peakRankName: "Radiant",
    peakRankTier: 27,
    peakRankIcon: "",
    peakAct: "E7A3",
    winRate: 60,
    wins: 18,
    games: 30,
    leaderboard: 0,
    accountLevel: 120,
    lastKills: 20,
    lastDeaths: 14,
    lastAssists: 5,
    lastHs: 25,
    lastAcs: 240,
    lastAdr: 155,
    lastKast: 72,
    hasCombat: true,
    streak: 0,
    rrTrend: 0,
    recentWins: 0,
    recentLosses: 0,
    smurfScore: 0,
    partySize: 1,
    encounters: 0,
    encounterWins: 0,
    encounterLosses: 0,
    locked: true,
    premiumSkins: false,
    vandalSkin: "",
    vandalImage: "",
    vandalTierColor: "",
  },
];

test("renders a player's name and rank", () => {
  // The design splits "Name#Tag" into a name span and a dim tag span.
  const { container } = render(<PlayerTable players={rows} state="CoreGame" combatLoading={false} />);
  expect(container.querySelector(".pid .name")?.textContent).toBe("Ace#NA1");
  expect(screen.getByText("Immortal 3")).toBeInTheDocument();
});

test("sorts higher rank first within a team", () => {
  const two: PlayerRow[] = [
    { ...rows[0], puuid: "low", name: "Low#1", rankTier: 10, rankName: "Gold 1" },
    { ...rows[0], puuid: "high", name: "High#1", rankTier: 24, rankName: "Radiant" },
  ];
  const { container } = render(<PlayerTable players={two} state="CoreGame" combatLoading={false} />);
  const names = Array.from(container.querySelectorAll(".pid .name")).map((el) => el.textContent);
  expect(names[0]).toBe("High#1");
});
