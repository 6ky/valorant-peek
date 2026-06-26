// Visual lookups ported from the imported design's renderer. These drive the
// monogram and color fallbacks shown when real art is missing. Real agent and
// rank art renders as an <img> on top of these.

// Role accent colors (design ROLE table).
export const ROLE: Record<string, string> = {
  duelist: "#d06a64",
  initiator: "#c7a456",
  controller: "#6486b5",
  sentinel: "#5a9e80",
};

// Agent -> [role, 2-letter monogram] (design AGENTS table).
const AGENTS: Record<string, [string, string]> = {
  Jett: ["duelist", "JT"],
  Reyna: ["duelist", "RY"],
  Raze: ["duelist", "RZ"],
  Phoenix: ["duelist", "PX"],
  Neon: ["duelist", "NE"],
  Iso: ["duelist", "IS"],
  Yoru: ["duelist", "YR"],
  Sova: ["initiator", "SV"],
  Fade: ["initiator", "FD"],
  Breach: ["initiator", "BR"],
  Skye: ["initiator", "SK"],
  Gekko: ["initiator", "GK"],
  KAYO: ["initiator", "KO"],
  Tejo: ["initiator", "TJ"],
  Omen: ["controller", "OM"],
  Brimstone: ["controller", "BS"],
  Viper: ["controller", "VP"],
  Astra: ["controller", "AS"],
  Harbor: ["controller", "HB"],
  Clove: ["controller", "CL"],
  Killjoy: ["sentinel", "KJ"],
  Cypher: ["sentinel", "CY"],
  Sage: ["sentinel", "SG"],
  Chamber: ["sentinel", "CH"],
  Deadlock: ["sentinel", "DL"],
  Vyse: ["sentinel", "VY"],
};

export interface AgentMeta {
  role: string;
  mono: string;
  color: string;
}

export function agentMeta(name: string): AgentMeta {
  const m = AGENTS[name];
  if (m) return { role: m[0], mono: m[1], color: ROLE[m[0]] };
  // Unknown agent: derive a 2-letter monogram from the name.
  const mono = (name || "").replace(/[^A-Za-z]/g, "").slice(0, 2).toUpperCase() || "??";
  return { role: "duelist", mono, color: ROLE.duelist };
}

// Division color chip, keyed by rankTier (design DIV table). Unranked is faint.
interface Div {
  min: number;
  max: number;
  color: string;
}
const DIVS: Div[] = [
  { min: 3, max: 5, color: "#80868f" }, // iron
  { min: 6, max: 8, color: "#a9763f" }, // bronze
  { min: 9, max: 11, color: "#aab2ba" }, // silver
  { min: 12, max: 14, color: "#d8b057" }, // gold
  { min: 15, max: 17, color: "#5aa6b5" }, // plat
  { min: 18, max: 20, color: "#b07cc9" }, // diamond
  { min: 21, max: 23, color: "#479e74" }, // ascendant
  { min: 24, max: 26, color: "#d05462" }, // immortal
  { min: 27, max: 27, color: "#e9d9a0" }, // radiant
];
const FAINT = "#565b65";

export function divColor(tier: number): string {
  for (const d of DIVS) {
    if (tier >= d.min && tier <= d.max) return d.color;
  }
  return FAINT;
}

// good/bad tone for a stat against the lobby (design tone() thresholds). For an
// enemy, a strong number is bad for us; for an ally it is good.
export function tone(val: number, lo: number, hi: number, isEnemy: boolean): string {
  const strong = val >= hi;
  const weak = val <= lo;
  if (!strong && !weak) return "tone-neutral";
  if (isEnemy) return strong ? "tone-bad" : "tone-good";
  return strong ? "tone-good" : "tone-bad";
}

// kills/deaths with a deaths==0 guard, matching the design's kd handling.
export function kdOf(kills: number, deaths: number): number {
  return deaths > 0 ? kills / deaths : kills;
}
