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
    readonly searchPlaceholder: string;
    readonly searchLabel: string;
    readonly loading: string;
    readonly countSummary: (shown: number, total: number) => string;
    readonly statTotalSize: string;
    readonly emptyTitle: string;
    readonly emptyBody: string;
    readonly noMatchesTitle: string;
    readonly noMatchesBody: string;
    readonly errorTitle: string;
    readonly retry: string;
    readonly columnProgram: string;
    readonly columnVersion: string;
    readonly columnPublisher: string;
    readonly columnSize: string;
    readonly columnInstalled: string;
    readonly badgeMsi: string;
    readonly badgeExecutable: string;
    readonly badgeManualOnly: string;
    readonly badgeNone: string;
    readonly badgeInvalid: string;
    readonly badgeManualOnlyHint: string;
    readonly badgeNoneHint: string;
    readonly badgeInvalidHint: string;
    readonly badgeUser: string;
    readonly badgeUserHint: string;
  };
  readonly uninstall: {
    readonly action: string;
    readonly confirmTitle: (name: string) => string;
    readonly confirmBody: string;
    readonly commandLabel: string;
    readonly elevationNote: string;
    readonly restorePointNote: string;
    readonly confirm: string;
    readonly cancel: string;
    readonly close: string;
    readonly planning: string;
    readonly planFailedTitle: string;
    readonly running: (name: string) => string;
    readonly runningNote: string;
    readonly reportSuccessTitle: string;
    readonly reportFailureTitle: string;
    readonly rebootNote: string;
    readonly restorePointCreated: string;
    readonly restorePointSkipped: (reason: string) => string;
    readonly restorePointFailed: (reason: string) => string;
    readonly exitCodeLabel: string;
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
    searchPlaceholder: "Search by name or publisher...",
    searchLabel: "Search installed programs",
    loading: "Reading installed programs...",
    countSummary: (shown, total) =>
      shown === total
        ? `${String(total)} programs`
        : `${String(shown)} of ${String(total)} programs`,
    statTotalSize: "on disk",
    emptyTitle: "No programs found",
    emptyBody:
      "No uninstallable programs were found in the Windows registry. This is unusual - if you believe it's wrong, please report it.",
    noMatchesTitle: "No matches",
    noMatchesBody: "No installed program matches your search.",
    errorTitle: "Could not read installed programs",
    retry: "Try again",
    columnProgram: "Program",
    columnVersion: "Version",
    columnPublisher: "Publisher",
    columnSize: "Size",
    columnInstalled: "Installed",
    badgeMsi: "MSI",
    badgeExecutable: "EXE",
    badgeManualOnly: "Manual",
    badgeNone: "No uninstaller",
    badgeInvalid: "Broken entry",
    badgeManualOnlyHint:
      "This program's uninstall command runs a script interpreter, so for safety it must be run manually.",
    badgeNoneHint: "This registry entry declares no uninstall command.",
    badgeInvalidHint: "This program's uninstall command could not be understood.",
    badgeUser: "User",
    badgeUserHint: "Installed for this user only, not machine-wide.",
  },
  uninstall: {
    action: "Uninstall",
    confirmTitle: (name) => `Uninstall ${name}?`,
    confirmBody:
      "Exactly the command below will run - nothing else. It was rebuilt from the Windows registry and re-checked; it will be re-checked once more at the moment it runs.",
    commandLabel: "Command",
    elevationNote: "Windows will ask for administrator approval (UAC) first.",
    restorePointNote: "A System Restore point will be attempted before anything runs.",
    confirm: "Uninstall",
    cancel: "Cancel",
    close: "Close",
    planning: "Checking what would run...",
    planFailedTitle: "Cannot uninstall automatically",
    running: (name) => `Uninstalling ${name}...`,
    runningNote:
      "The program's uninstaller is running. This window stays responsive; some uninstallers open their own windows.",
    reportSuccessTitle: "Uninstalled",
    reportFailureTitle: "Uninstall did not complete",
    rebootNote: "A restart is required to finish removing files.",
    restorePointCreated: "System Restore point: created.",
    restorePointSkipped: (reason) => `System Restore point: skipped - ${reason}`,
    restorePointFailed: (reason) => `System Restore point: not created - ${reason}`,
    exitCodeLabel: "Exit code",
  },
  errors: {
    generic: "Something went wrong. Please try again.",
  },
};

export const dictionaries = { en } as const;
export type Locale = keyof typeof dictionaries;

/** Active dictionary. Lifted into state when more locales land. */
export const text: Dictionary = dictionaries.en;
