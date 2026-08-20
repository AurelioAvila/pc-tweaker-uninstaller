/**
 * UI strings. English is the launch language; the shape mirrors PC Tweaker's
 * dictionary so the other four languages (IT/FR/ES/DE) can be added without
 * touching component code. Every user-visible string lives here — components
 * never embed literals, which is what keeps a translation pass mechanical.
 */

export type Dictionary = {
  readonly app: {
    readonly title: string;
    readonly tagline: string;
  };
  readonly programs: {
    readonly emptyTitle: string;
    readonly emptyBody: string;
  };
  readonly errors: {
    readonly generic: string;
  };
};

const en: Dictionary = {
  app: {
    title: "PC Tweaker Uninstaller",
    tagline: "Uninstall programs cleanly - with a safety net.",
  },
  programs: {
    emptyTitle: "No programs loaded yet",
    emptyBody:
      "Program scanning arrives in the next update. This build establishes the app shell, license verification, and the rollback engine.",
  },
  errors: {
    generic: "Something went wrong. Please try again.",
  },
};

export const dictionaries = { en } as const;
export type Locale = keyof typeof dictionaries;

/** Active dictionary. Lifted into state when more locales land. */
export const text: Dictionary = dictionaries.en;
