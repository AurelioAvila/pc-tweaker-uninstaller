export type IconName =
  | "trash"
  | "inspect"
  | "check"
  | "lock"
  | "camera"
  | "keyboard"
  | "music"
  | "apps"
  | "drive"
  | "clock"
  | "filters"
  | "refresh"
  | "history"
  | "user"
  | "shield"
  | "chevron"
  | "download"
  | "palette"
  | "globe"
  | "close"
  | "package";
const paths: Record<IconName, string> = {
  trash: "M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7M14 10v7",
  inspect: "M10 3a7 7 0 1 0 0 14 7 7 0 0 0 0-14Zm5 12 6 6M7 10h6M10 7v6",
  check: "m5 12 4 4L19 6",
  lock: "M5 10h14v11H5zM8 10V7a4 4 0 0 1 8 0v3M12 14v3",
  camera: "M3 7h5l2-3h4l2 3h5v13H3zM12 10a4 4 0 1 0 0 8 4 4 0 0 0 0-8",
  keyboard: "M3 5h18v14H3zM7 9h.01M11 9h.01M15 9h.01M18 9h.01M7 12h.01M11 12h.01M15 12h.01M7 16h10",
  music: "M9 18V5l11-2v13M9 18a3 3 0 1 1-3-3h3M20 16a3 3 0 1 1-3-3h3",
  apps: "M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z",
  drive: "M5 4h14l2 11v5H3v-5L5 4ZM3 15h18M7 18h.01M11 18h.01",
  clock: "M12 4a8 8 0 1 0 0 16 8 8 0 0 0 0-16ZM12 8v5l3 2",
  filters: "M4 7h9m4 0h3M4 17h3m4 0h9M13 4v6M7 14v6",
  refresh: "M20 8a8 8 0 0 0-14-2L3 9m0-5v5h5M4 16a8 8 0 0 0 14 2l3-3m0 5v-5h-5",
  history: "M4 9a8 8 0 1 1 0 6M3 4v5h5M12 8v5l3 2",
  user: "M12 3a4 4 0 1 0 0 8 4 4 0 0 0 0-8ZM4 21v-2a8 8 0 0 1 16 0v2",
  shield: "m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6l8-3ZM8 12l3 3 5-6",
  chevron: "m9 5 7 7-7 7",
  download: "M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5",
  palette:
    "M12 3a9 9 0 1 0 0 18c2 0 3-1 2-3s0-3 2-3h2c4 0 4-12-6-12ZM7 9h.01M11 6h.01M16 8h.01M6 14h.01",
  globe: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18ZM3 12h18M12 3c-5 5-5 13 0 18 5-5 5-13 0-18Z",
  close: "m6 6 12 12M18 6 6 18",
  package: "m12 3 9 5v9l-9 5-9-5V8l9-5ZM3 8l9 5 9-5M12 13v9M7 5.8l10 5.5",
};
export function UiIcon({ name, className = "" }: { name: IconName; className?: string }) {
  return (
    <svg className={`ui-icon ${className}`} viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path
        d={paths[name]}
        stroke="currentColor"
        strokeWidth="1.7"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
