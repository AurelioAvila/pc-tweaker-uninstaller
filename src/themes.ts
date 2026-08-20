/**
 * Themes: the same tema≠identità contract as PC Tweaker — a theme swaps a
 * small set of raw values (backgrounds, glow, accent pair); every component
 * reads only the CSS variables, so identity survives any palette.
 */

export type ThemeCode =
  | "violet"
  | "teal_depths"
  | "crimson_steel"
  | "ocean_blue"
  | "forest_emerald"
  | "royal_gold"
  | "slate_mono"
  | "coral_sunset";

export interface Theme {
  code: ThemeCode;
  label: string;
  vars: {
    bg: string;
    bgRaised: string;
    surface: string;
    glow: string;
    accent: string;
    accent2: string;
  };
}

export const THEMES: Theme[] = [
  {
    code: "violet",
    label: "Violet",
    vars: {
      bg: "#0a0912",
      bgRaised: "#12101d",
      surface: "#16131f",
      glow: "#1c142e",
      accent: "#8b5cf6",
      accent2: "#d946ef",
    },
  },
  {
    code: "teal_depths",
    label: "Teal Depths",
    vars: {
      bg: "#07110f",
      bgRaised: "#0d1a17",
      surface: "#101e1b",
      glow: "#122a26",
      accent: "#2dd4bf",
      accent2: "#22d3ee",
    },
  },
  {
    code: "crimson_steel",
    label: "Crimson Steel",
    vars: {
      bg: "#120a0d",
      bgRaised: "#1c1014",
      surface: "#201318",
      glow: "#2c1420",
      accent: "#fb7185",
      accent2: "#f43f5e",
    },
  },
  {
    code: "ocean_blue",
    label: "Ocean Blue",
    vars: {
      bg: "#080d16",
      bgRaised: "#0e1522",
      surface: "#111927",
      glow: "#122036",
      accent: "#60a5fa",
      accent2: "#38bdf8",
    },
  },
  {
    code: "forest_emerald",
    label: "Forest Emerald",
    vars: {
      bg: "#08110b",
      bgRaised: "#0e1a12",
      surface: "#111e15",
      glow: "#122a1a",
      accent: "#34d399",
      accent2: "#4ade80",
    },
  },
  {
    code: "royal_gold",
    label: "Royal Gold",
    vars: {
      bg: "#110d06",
      bgRaised: "#1a140a",
      surface: "#1e180c",
      glow: "#2a2010",
      accent: "#fbbf24",
      accent2: "#f59e0b",
    },
  },
  {
    code: "slate_mono",
    label: "Slate Mono",
    vars: {
      bg: "#0b0d10",
      bgRaised: "#121519",
      surface: "#15181d",
      glow: "#1a2028",
      accent: "#9aa5b1",
      accent2: "#cbd5e1",
    },
  },
  {
    code: "coral_sunset",
    label: "Coral Sunset",
    vars: {
      bg: "#120b08",
      bgRaised: "#1c110c",
      surface: "#20140e",
      glow: "#2c1a12",
      accent: "#fb923c",
      accent2: "#f97316",
    },
  },
];

const THEME_KEY = "pcu-theme";

export function initialTheme(): ThemeCode {
  try {
    const stored = localStorage.getItem(THEME_KEY);
    if (stored !== null && THEMES.some((t) => t.code === stored)) return stored as ThemeCode;
  } catch {
    // Default stands.
  }
  return "violet";
}

/** Writes the theme's raw values onto :root and persists the choice. */
export function applyTheme(code: ThemeCode): void {
  const theme = THEMES.find((t) => t.code === code);
  if (theme === undefined) return;
  const root = document.documentElement.style;
  root.setProperty("--bg", theme.vars.bg);
  root.setProperty("--bg-raised", theme.vars.bgRaised);
  root.setProperty("--surface", theme.vars.surface);
  root.setProperty("--glow", theme.vars.glow);
  root.setProperty("--accent", theme.vars.accent);
  root.setProperty("--accent-2", theme.vars.accent2);
  try {
    localStorage.setItem(THEME_KEY, code);
  } catch {
    // Non-fatal.
  }
}
