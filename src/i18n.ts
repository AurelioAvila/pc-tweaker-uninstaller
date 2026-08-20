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
    readonly badgeHidden: string;
    readonly badgeHiddenHint: string;
    readonly badgeSuite: string;
    readonly badgeSuiteHint: string;
    readonly filterAll: string;
    readonly filterLarge: string;
    readonly filterRecent: string;
    readonly showHidden: string;
    readonly detailSource: string;
    readonly detailKey: string;
    readonly detailLocation: string;
    readonly detailNoLocation: string;
    readonly openFolder: string;
    readonly sourceMachine64: string;
    readonly sourceMachine32: string;
    readonly sourceUser: string;
  };
  readonly footer: {
    readonly family: string;
    readonly pcTweaker: string;
    readonly promptShield: string;
    readonly privacy: string;
    readonly restoreInfo: string;
    readonly openRestore: string;
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
    readonly familyNote: string;
    readonly hiddenNote: string;
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
    badgeHidden: "Hidden",
    badgeHiddenHint:
      "Windows normally hides this entry (system component or child update). Removing it can affect other software - be sure you know what it is.",
    badgeSuite: "Suite",
    badgeSuiteHint: "Part of your PC Tweaker suite.",
    filterAll: "All",
    filterLarge: "Large",
    filterRecent: "Recent",
    showHidden: "Show hidden",
    detailSource: "Scope",
    detailKey: "Registry entry",
    detailLocation: "Install folder",
    detailNoLocation: "Not recorded",
    openFolder: "Open folder",
    sourceMachine64: "This PC (64-bit)",
    sourceMachine32: "This PC (32-bit)",
    sourceUser: "This user only",
  },
  footer: {
    family: "Part of the PC Tweaker family",
    pcTweaker: "PC Tweaker",
    promptShield: "PromptShield",
    privacy: "Privacy",
    restoreInfo:
      "Restore points are created and stored by Windows on your system drive (System Protection).",
    openRestore: "Manage restore points",
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
    familyNote:
      "This app is part of your PC Tweaker suite. You can remove it, but suite features that depend on it will stop working.",
    hiddenNote:
      "Windows normally hides this entry. Removing system components or child updates can affect other software.",
  },
  errors: {
    generic: "Something went wrong. Please try again.",
  },
};

export const dictionaries = { en } as const;
export type Locale = keyof typeof dictionaries;

/** Active dictionary. Lifted into state when more locales land. */
export const text: Dictionary = dictionaries.en;
