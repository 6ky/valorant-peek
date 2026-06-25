interface Division {
  min: number;
  max: number;
  color: string;
}

// Authentic-ish competitive tier colors, keyed by tier number range.
const DIVISIONS: Division[] = [
  { min: 3, max: 5, color: "#7c7e82" }, // Iron
  { min: 6, max: 8, color: "#a97142" }, // Bronze
  { min: 9, max: 11, color: "#c6cfd8" }, // Silver
  { min: 12, max: 14, color: "#e7c45a" }, // Gold
  { min: 15, max: 17, color: "#3fc7c9" }, // Platinum
  { min: 18, max: 20, color: "#c079e0" }, // Diamond
  { min: 21, max: 23, color: "#2fcb6e" }, // Ascendant
  { min: 24, max: 26, color: "#e0436f" }, // Immortal
  { min: 27, max: 27, color: "#fff4b8" }, // Radiant
];

const UNRANKED = "#565b64";

export function tierColor(tier: number): string {
  for (const d of DIVISIONS) {
    if (tier >= d.min && tier <= d.max) return d.color;
  }
  return UNRANKED;
}

// Radiant gets a glow; high tiers feel "hot" at a glance.
export function tierGlow(tier: number): boolean {
  return tier >= 27;
}
