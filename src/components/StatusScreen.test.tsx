import { render, screen } from "@testing-library/react";
import { StatusScreen } from "./StatusScreen";

test("shows waiting message for NoGame", () => {
  render(<StatusScreen state="NoGame" />);
  expect(screen.getByText(/waiting for valorant/i)).toBeInTheDocument();
});

test("shows menu message for Menu", () => {
  render(<StatusScreen state="Menu" />);
  expect(screen.getByText(/queue a game/i)).toBeInTheDocument();
});
