export const SORT_KEYS = ["name", "size", "date", "publisher", "source"] as const;
export type SortKey = (typeof SORT_KEYS)[number];
export interface InventoryRow {
  id: string;
  name: string;
  source: string;
  publisher: string | null;
  installDate: string | null;
  estimatedSizeKb: number | null;
}

export function dateValue(value: string | null): number | null {
  if (!value || !/^\d{4}-\d{2}-\d{2}$/.test(value)) return null;
  const time = Date.parse(`${value}T00:00:00Z`);
  return Number.isFinite(time) && new Date(time).toISOString().slice(0, 10) === value ? time : null;
}

export function isRecent(value: string | null, now: Date): boolean {
  const time = dateValue(value);
  const today = Date.UTC(now.getFullYear(), now.getMonth(), now.getDate());
  return time !== null && time <= today && today - time <= 30 * 86400000;
}

const collator = new Intl.Collator(undefined, { sensitivity: "base", numeric: true });

export function comparePrograms(
  a: InventoryRow,
  b: InventoryRow,
  key: SortKey,
  asc: boolean,
): number {
  const value = (row: InventoryRow): string | number | null => {
    if (key === "size")
      return row.estimatedSizeKb !== null &&
        Number.isFinite(row.estimatedSizeKb) &&
        row.estimatedSizeKb > 0
        ? row.estimatedSizeKb
        : null;
    if (key === "date") return dateValue(row.installDate);
    if (key === "publisher") return row.publisher?.trim() || null;
    return row[key];
  };
  const av = value(a),
    bv = value(b);
  // Unknown data stays last regardless of direction; it is not zero space.
  if (av === null && bv !== null) return 1;
  if (av !== null && bv === null) return -1;
  const primary =
    av === null || bv === null
      ? 0
      : typeof av === "number" && typeof bv === "number"
        ? av - bv
        : collator.compare(String(av), String(bv));
  return (
    (asc ? primary : -primary) ||
    collator.compare(a.name, b.name) ||
    a.source.localeCompare(b.source) ||
    a.id.localeCompare(b.id)
  );
}

export function readView(): { key: SortKey; asc: boolean; compact: boolean } {
  const fallback = { key: "name" as const, asc: true, compact: false };
  try {
    const raw: unknown = JSON.parse(localStorage.getItem("pcu-inventory-view") ?? "null");
    if (!raw || typeof raw !== "object") return fallback;
    const v = raw as Record<string, unknown>;
    return {
      key: SORT_KEYS.includes(v.key as SortKey) ? (v.key as SortKey) : "name",
      asc: typeof v.asc === "boolean" ? v.asc : true,
      compact: v.compact === true,
    };
  } catch {
    return fallback;
  }
}
