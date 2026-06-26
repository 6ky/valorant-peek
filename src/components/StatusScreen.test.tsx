import { render, screen } from "@testing-library/react";
import { StatusScreen } from "./StatusScreen";

test("shows the waiting headline for NoGame", () => {
  render(<StatusScreen state="NoGame" />);
  expect(screen.getByText(/waiting for valorant/i)).toBeInTheDocument();
});

test("marks VALORANT not running when offline", () => {
  render(<StatusScreen state="NoGame" />);
  expect(screen.getByText(/not running/i)).toBeInTheDocument();
});
