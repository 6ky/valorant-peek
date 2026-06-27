import { render, screen } from "@testing-library/react";
import { StatusScreen } from "./StatusScreen";

test("shows the waiting headline for NoGame", () => {
  render(<StatusScreen state="NoGame" />);
  expect(screen.getByText(/waiting for valorant/i)).toBeInTheDocument();
});

test("marks VALORANT not running when offline", () => {
  // Riot Client and VALORANT both read "Not running" when offline, so target the
  // VALORANT row's value specifically rather than matching either one.
  const { container } = render(<StatusScreen state="NoGame" />);
  const valorant = Array.from(container.querySelectorAll(".sb-row")).find(
    (row) => row.querySelector(".l")?.textContent === "VALORANT"
  );
  expect(valorant?.querySelector(".v")?.textContent).toMatch(/not running/i);
});
