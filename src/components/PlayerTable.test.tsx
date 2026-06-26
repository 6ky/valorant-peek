import { render, screen } from "@testing-library/react";
import { PlayerTable } from "./PlayerTable";
import { PlayerRow } from "../types";

const rows: PlayerRow[] = [
  {
    puuid: "p1",
    name: "Ace#NA1",
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
    winRate: 60,
    games: 30,
    accountLevel: 120,
  },
];

test("renders a player's name and rank", () => {
  render(<PlayerTable players={rows} />);
  expect(screen.getByText("Ace#NA1")).toBeInTheDocument();
  expect(screen.getByText(/Immortal 3/)).toBeInTheDocument();
});

test("sorts higher rank first within a team", () => {
  const two: PlayerRow[] = [
    { ...rows[0], puuid: "low", name: "Low#1", rankTier: 10, rankName: "Gold 1" },
    { ...rows[0], puuid: "high", name: "High#1", rankTier: 24, rankName: "Radiant" },
  ];
  render(<PlayerTable players={two} />);
  const names = screen.getAllByText(/#1/).map((el) => el.textContent);
  expect(names[0]).toBe("High#1");
});
