/**
 * UI strings for all five suite languages. English is the default; the
 * selected locale persists in localStorage. Every user-visible string lives
 * here — components never embed literals.
 *
 * `text` is a module-level binding deliberately: the app is a single
 * component tree that re-renders when the language state changes, so
 * reassigning before that render is race-free and keeps 800 lines of
 * call sites untouched.
 */

export type ConfidenceReason =
  | "hiddenSystem"
  | "sharedRuntime"
  | "driverComponent"
  | "sharedLauncher"
  | "noPublisher"
  | "brokenUninstaller"
  | "manualUninstaller"
  | "noUninstaller"
  | "namedPublisher"
  | "standardUninstaller";

export type Dictionary = {
  readonly app: {
    readonly title: string;
    readonly tagline: string;
    readonly suiteDetected: string;
    readonly suiteDetectedHint: string;
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
  readonly confidence: {
    readonly labelSafe: string;
    readonly labelReview: string;
    readonly labelKeep: string;
    readonly disclaimer: string;
    readonly reasons: Record<ConfidenceReason, string>;
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
    readonly methodLabel: string;
    readonly methodMsi: string;
    readonly methodExe: string;
    readonly privilegesLabel: string;
    readonly privilegesAdmin: string;
    readonly privilegesUser: string;
    readonly sizeLabel: string;
    readonly sizeUnknown: string;
    readonly confidenceLabel: string;
    readonly notRemovedNote: string;
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
    readonly residueScan: string;
    readonly residueScanning: string;
    readonly residueTitle: string;
    readonly residueNone: string;
    readonly residueIntro: (count: number, mb: string) => string;
    readonly residueClean: string;
    readonly residueRegistryNote: string;
    readonly residueDone: (count: number, mb: string) => string;
    readonly residueFailed: (count: number) => string;
    readonly residueKinds: Readonly<Record<string, string>>;
    readonly relDependentsWarning: (names: string) => string;
    readonly relInstalledVia: (name: string) => string;
    readonly relSiblings: (count: number) => string;
    readonly batchBar: (count: number, size: string) => string;
    readonly batchClear: string;
    readonly batchConfirmTitle: (count: number) => string;
    readonly batchConfirmBody: string;
    readonly batchNotBatchable: string;
    readonly batchRunningStep: (name: string, index: number, total: number) => string;
    readonly batchDoneTitle: string;
    readonly batchFailedNote: (count: number) => string;
    readonly familyNote: string;
    readonly hiddenNote: string;
  };
  readonly ledger: {
    readonly open: string;
    readonly title: string;
    readonly subtitle: string;
    readonly empty: string;
    readonly exportButton: string;
    readonly exportedTo: (path: string) => string;
    readonly verifiedFreed: (size: string) => string;
    readonly estimatedOnly: (size: string) => string;
    readonly rebootFlag: string;
    readonly failedFlag: string;
    readonly restorePointLabel: string;
  };
  readonly menu: {
    readonly open: string;
    readonly account: string;
    readonly signIn: string;
    readonly signInHint: string;
    readonly plans: string;
    readonly planMonthly: string;
    readonly planAnnual: string;
    readonly loyaltyTitle: string;
    readonly loyaltyPrice: string;
    readonly loyaltyHint: string;
    readonly choosePlans: string;
    readonly language: string;
    readonly theme: string;
    readonly openPcTweaker: string;
    readonly emailLabel: string;
    readonly passwordLabel: string;
    readonly signInButton: string;
    readonly signingIn: string;
    readonly signOut: string;
    readonly proActive: string;
    readonly proInactive: string;
    readonly loyaltyLocked: string;
    readonly checkFailed: string;
    readonly registerHint: string;
  };
  readonly updater: {
    readonly title: (version: string) => string;
    readonly body: string;
    readonly install: string;
    readonly later: string;
    readonly downloading: (percent: number) => string;
    readonly installing: string;
    readonly error: (message: string) => string;
  };
  readonly errors: {
    readonly generic: string;
  };
};

const en: Dictionary = {
  app: {
    title: "PC Tweaker Uninstaller",
    tagline: "Removal Intelligence for Windows - remove software with clarity, not guesswork.",
    suiteDetected: "PC Tweaker detected - suite member",
    suiteDetectedHint:
      "PC Tweaker is installed on this PC. Sign in with your PC Tweaker Pro account to unlock the loyalty price.",
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
  confidence: {
    labelSafe: "Safe to remove",
    labelReview: "Review before removing",
    labelKeep: "Keep - system-related",
    disclaimer: "Based on visible evidence - a guide, not a certainty.",
    reasons: {
      hiddenSystem: "Windows itself hides this entry (system component or child update).",
      sharedRuntime: "Shared runtime: other programs likely depend on it.",
      driverComponent: "Driver or chipset package: removing it can affect hardware.",
      sharedLauncher: "Launcher/store: software installed through it would stop working.",
      noPublisher: "No publisher recorded - origin cannot be verified.",
      brokenUninstaller: "Its uninstall command is broken and cannot run automatically.",
      manualUninstaller: "Its uninstaller must be run manually (script-based).",
      noUninstaller: "It declares no uninstall command at all.",
      namedPublisher: "A named publisher is recorded.",
      standardUninstaller: "It has a standard MSI/EXE uninstaller.",
    },
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
      "Exactly the command below will run - nothing else. It was rebuilt from the Windows registry and will be re-checked at the moment it runs.",
    commandLabel: "Command",
    methodLabel: "Method",
    methodMsi: "Windows Installer (silent)",
    methodExe: "The program's own uninstaller",
    privilegesLabel: "Privileges",
    privilegesAdmin: "Administrator (one UAC prompt)",
    privilegesUser: "Current user",
    sizeLabel: "Estimated space to reclaim",
    sizeUnknown: "Not recorded",
    confidenceLabel: "Confidence",
    notRemovedNote:
      "Not removed automatically: leftover files, folders and registry entries. Residue scanning arrives in a later update and will always ask first.",
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
    residueScan: "Scan for leftovers",
    residueScanning: "Scanning for leftovers...",
    residueTitle: "Leftovers found",
    residueNone: "No leftovers found. Clean removal.",
    residueIntro: (count, mb) => `${String(count)} item(s) left behind, about ${mb} MB. Everything you select goes to the Recycle Bin, so it stays recoverable.`,
    residueClean: "Move selected to Recycle Bin",
    residueRegistryNote: "User registry keys are deleted directly (not recoverable); machine-wide keys are listed for reference only.",
    residueDone: (count, mb) => `Cleaned ${String(count)} item(s), freed about ${mb} MB.`,
    residueFailed: (count) => `${String(count)} item(s) could not be removed.`,
    residueKinds: { "install-dir": "Install folder", "app-data": "App data", shortcut: "Shortcut", "registry-user": "Registry (user)", "registry-machine": "Registry (machine)" },
    relDependentsWarning: (names) => `Removing this also takes down programs installed inside its folder: ${names}.`,
    relInstalledVia: (name) => `Installed via ${name} — consider removing it from there instead.`,
    relSiblings: (count) => `${String(count)} other program(s) from this publisher are installed.`,
    batchBar: (count, size) => `Uninstall ${String(count)} selected · ${size}`,
    batchClear: "Clear selection",
    batchConfirmTitle: (count) => `Remove ${String(count)} programs?`,
    batchConfirmBody: "They run one at a time, contained programs before their containers, with the same checks as a single uninstall. Protected entries (system components, shared runtimes, your PC Tweaker suite) cannot be selected at all.",
    batchNotBatchable: "Protected — not available for batch removal.",
    batchRunningStep: (name, index, total) => `Uninstalling ${name} (${String(index)} of ${String(total)})...`,
    batchDoneTitle: "Batch complete",
    batchFailedNote: (count) => `${String(count)} program(s) did not complete — see the ledger for details.`,
    familyNote:
      "This app is part of your PC Tweaker suite. You can remove it, but suite features that depend on it will stop working.",
    hiddenNote:
      "Windows normally hides this entry. Removing system components or child updates can affect other software.",
  },
  ledger: {
    open: "History",
    title: "Removal Ledger",
    subtitle:
      "A local receipt for every removal this app ran - successes and failures alike. Stored on this PC, never uploaded.",
    empty: "No removals recorded yet. Your first uninstall will leave its receipt here.",
    exportButton: "Export JSON",
    exportedTo: (path) => `Exported to ${path}`,
    verifiedFreed: (size) => `${size} freed (verified)`,
    estimatedOnly: (size) => `~${size} (registry estimate)`,
    rebootFlag: "restart required",
    failedFlag: "failed",
    restorePointLabel: "Restore point",
  },
  menu: {
    open: "Account & settings",
    account: "Account",
    signIn: "Sign in / Register on pctweaker.app",
    signInHint:
      "One account for the whole PC Tweaker suite. Registration happens on pctweaker.app and is valid here too.",
    plans: "Uninstaller Pro",
    planMonthly: "€3.99 / month",
    planAnnual: "€13.99 / year",
    loyaltyTitle: "Suite loyalty price",
    loyaltyPrice: "€4.99 / year",
    loyaltyHint:
      "Already subscribed to any PC Tweaker plan? Uninstaller Pro costs €4.99/year on the same account.",
    choosePlans: "See plans on pctweaker.app",
    language: "Language",
    theme: "Theme",
    openPcTweaker: "Open PC Tweaker",
    emailLabel: "Email",
    passwordLabel: "Password",
    signInButton: "Sign in",
    signingIn: "Signing in...",
    signOut: "Sign out",
    proActive: "PC Tweaker Pro - active on this account",
    proInactive: "This account is not on PC Tweaker Pro yet.",
    loyaltyLocked: "Sign in with your PC Tweaker Pro account to unlock the loyalty price.",
    checkFailed: "Could not verify your account right now. Try again shortly.",
    registerHint: "No account yet? Create one on pctweaker.app.",
  },
  updater: {
    title: (version) => `Update ${version} is ready`,
    body: "A new signed build is available. It installs in seconds and the app restarts by itself.",
    install: "Install now",
    later: "Later",
    downloading: (percent) => `Downloading… ${String(percent)}%`,
    installing: "Installing…",
    error: (message) => `The update could not be installed: ${message}`,
  },
  errors: {
    generic: "Something went wrong. Please try again.",
  },
};

const it: Dictionary = {
  app: {
    title: "PC Tweaker Uninstaller",
    tagline: "Removal Intelligence per Windows: rimuovi il software con chiarezza, non a intuito.",
    suiteDetected: "PC Tweaker rilevato - membro della suite",
    suiteDetectedHint:
      "PC Tweaker è installato su questo PC. Accedi con il tuo account PC Tweaker Pro per sbloccare il prezzo fedeltà.",
  },
  programs: {
    searchPlaceholder: "Cerca per nome o produttore...",
    searchLabel: "Cerca tra i programmi installati",
    loading: "Lettura dei programmi installati...",
    countSummary: (shown, total) =>
      shown === total
        ? `${String(total)} programmi`
        : `${String(shown)} di ${String(total)} programmi`,
    statTotalSize: "su disco",
    emptyTitle: "Nessun programma trovato",
    emptyBody:
      "Nessun programma disinstallabile trovato nel registro di Windows. È insolito: se pensi sia un errore, segnalacelo.",
    noMatchesTitle: "Nessun risultato",
    noMatchesBody: "Nessun programma installato corrisponde alla ricerca.",
    errorTitle: "Impossibile leggere i programmi installati",
    retry: "Riprova",
    columnProgram: "Programma",
    columnVersion: "Versione",
    columnPublisher: "Produttore",
    columnSize: "Dimensione",
    columnInstalled: "Installato",
    badgeMsi: "MSI",
    badgeExecutable: "EXE",
    badgeManualOnly: "Manuale",
    badgeNone: "Senza uninstaller",
    badgeInvalid: "Voce danneggiata",
    badgeManualOnlyHint:
      "Il comando di disinstallazione usa un interprete di script: per sicurezza va eseguito manualmente.",
    badgeNoneHint: "Questa voce di registro non dichiara alcun comando di disinstallazione.",
    badgeInvalidHint: "Il comando di disinstallazione di questo programma non è comprensibile.",
    badgeUser: "Utente",
    badgeUserHint: "Installato solo per questo utente, non per tutto il PC.",
    badgeHidden: "Nascosto",
    badgeHiddenHint:
      "Windows normalmente nasconde questa voce (componente di sistema o aggiornamento). Rimuoverla può influire su altro software: assicurati di sapere cos'è.",
    badgeSuite: "Suite",
    badgeSuiteHint: "Parte della tua suite PC Tweaker.",
    filterAll: "Tutti",
    filterLarge: "Grandi",
    filterRecent: "Recenti",
    showHidden: "Mostra nascosti",
    detailSource: "Ambito",
    detailKey: "Voce di registro",
    detailLocation: "Cartella di installazione",
    detailNoLocation: "Non registrata",
    openFolder: "Apri cartella",
    sourceMachine64: "Questo PC (64 bit)",
    sourceMachine32: "Questo PC (32 bit)",
    sourceUser: "Solo questo utente",
  },
  confidence: {
    labelSafe: "Rimozione sicura",
    labelReview: "Rivedi prima di rimuovere",
    labelKeep: "Da tenere - componente di sistema",
    disclaimer: "Basato su evidenze visibili: una guida, non una certezza.",
    reasons: {
      hiddenSystem: "Windows stesso nasconde questa voce (componente di sistema o aggiornamento).",
      sharedRuntime: "Runtime condiviso: altri programmi probabilmente ne dipendono.",
      driverComponent: "Pacchetto driver o chipset: rimuoverlo può influire sull'hardware.",
      sharedLauncher:
        "Launcher/store: il software installato tramite esso smetterebbe di funzionare.",
      noPublisher: "Nessun produttore registrato: origine non verificabile.",
      brokenUninstaller:
        "Il comando di disinstallazione è danneggiato e non può girare in automatico.",
      manualUninstaller: "Il suo uninstaller va eseguito manualmente (basato su script).",
      noUninstaller: "Non dichiara alcun comando di disinstallazione.",
      namedPublisher: "È registrato un produttore con nome.",
      standardUninstaller: "Ha un uninstaller standard MSI/EXE.",
    },
  },
  footer: {
    family: "Parte della famiglia PC Tweaker",
    pcTweaker: "PC Tweaker",
    promptShield: "PromptShield",
    privacy: "Privacy",
    restoreInfo:
      "I punti di ripristino sono creati e conservati da Windows sull'unità di sistema (Protezione sistema).",
    openRestore: "Gestisci punti di ripristino",
  },
  uninstall: {
    action: "Disinstalla",
    confirmTitle: (name) => `Disinstallare ${name}?`,
    confirmBody:
      "Verrà eseguito esattamente il comando qui sotto, nient'altro. È ricostruito dal registro di Windows e verrà ricontrollato al momento dell'esecuzione.",
    commandLabel: "Comando",
    methodLabel: "Metodo",
    methodMsi: "Windows Installer (silenzioso)",
    methodExe: "L'uninstaller del programma stesso",
    privilegesLabel: "Privilegi",
    privilegesAdmin: "Amministratore (una richiesta UAC)",
    privilegesUser: "Utente corrente",
    sizeLabel: "Spazio stimato da recuperare",
    sizeUnknown: "Non registrato",
    confidenceLabel: "Affidabilità",
    notRemovedNote:
      "Non rimosso automaticamente: file, cartelle e voci di registro residue. La scansione dei residui arriverà in un prossimo aggiornamento e chiederà sempre prima.",
    elevationNote: "Windows chiederà prima l'approvazione da amministratore (UAC).",
    restorePointNote: "Prima dell'esecuzione verrà tentato un punto di ripristino di sistema.",
    confirm: "Disinstalla",
    cancel: "Annulla",
    close: "Chiudi",
    planning: "Verifica di cosa verrebbe eseguito...",
    planFailedTitle: "Disinstallazione automatica non possibile",
    running: (name) => `Disinstallazione di ${name}...`,
    runningNote:
      "L'uninstaller del programma è in esecuzione. Questa finestra resta reattiva; alcuni uninstaller aprono finestre proprie.",
    reportSuccessTitle: "Disinstallato",
    reportFailureTitle: "Disinstallazione non completata",
    rebootNote: "Serve un riavvio per completare la rimozione dei file.",
    restorePointCreated: "Punto di ripristino: creato.",
    restorePointSkipped: (reason) => `Punto di ripristino: saltato - ${reason}`,
    restorePointFailed: (reason) => `Punto di ripristino: non creato - ${reason}`,
    exitCodeLabel: "Codice di uscita",
    residueScan: "Cerca residui",
    residueScanning: "Ricerca dei residui...",
    residueTitle: "Residui trovati",
    residueNone: "Nessun residuo trovato. Rimozione pulita.",
    residueIntro: (count, mb) => `${String(count)} elemento/i rimasti, circa ${mb} MB. Tutto ciò che selezioni va nel Cestino, quindi resta recuperabile.`,
    residueClean: "Sposta selezionati nel Cestino",
    residueRegistryNote: "Le chiavi di registro utente vengono eliminate direttamente (non recuperabili); quelle di sistema sono elencate solo per riferimento.",
    residueDone: (count, mb) => `Puliti ${String(count)} elemento/i, liberati circa ${mb} MB.`,
    residueFailed: (count) => `${String(count)} elemento/i non rimossi.`,
    residueKinds: { "install-dir": "Cartella di installazione", "app-data": "Dati applicazione", shortcut: "Collegamento", "registry-user": "Registro (utente)", "registry-machine": "Registro (sistema)" },
    relDependentsWarning: (names) => `Rimuovendolo elimini anche i programmi installati nella sua cartella: ${names}.`,
    relInstalledVia: (name) => `Installato tramite ${name} — valuta di rimuoverlo da lì.`,
    relSiblings: (count) => `${String(count)} altro/i programma/i di questo publisher sono installati.`,
    batchBar: (count, size) => `Disinstalla ${String(count)} selezionati · ${size}`,
    batchClear: "Svuota selezione",
    batchConfirmTitle: (count) => `Rimuovere ${String(count)} programmi?`,
    batchConfirmBody: "Vengono eseguiti uno alla volta, i programmi contenuti prima dei loro contenitori, con gli stessi controlli di una disinstallazione singola. Le voci protette (componenti di sistema, runtime condivisi, la tua suite PC Tweaker) non sono selezionabili.",
    batchNotBatchable: "Protetto — non disponibile per la rimozione in batch.",
    batchRunningStep: (name, index, total) => `Disinstallazione di ${name} (${String(index)} di ${String(total)})...`,
    batchDoneTitle: "Batch completato",
    batchFailedNote: (count) => `${String(count)} programma/i non completati — vedi il registro per i dettagli.`,
    familyNote:
      "Questa app fa parte della tua suite PC Tweaker. Puoi rimuoverla, ma le funzioni della suite che ne dipendono smetteranno di funzionare.",
    hiddenNote:
      "Windows normalmente nasconde questa voce. Rimuovere componenti di sistema o aggiornamenti può influire su altro software.",
  },
  ledger: {
    open: "Cronologia",
    title: "Registro rimozioni",
    subtitle:
      "Una ricevuta locale per ogni rimozione eseguita da questa app, successi e fallimenti inclusi. Salvata su questo PC, mai caricata online.",
    empty: "Nessuna rimozione registrata. La prima disinstallazione lascerà qui la sua ricevuta.",
    exportButton: "Esporta JSON",
    exportedTo: (path) => `Esportato in ${path}`,
    verifiedFreed: (size) => `${size} liberati (verificato)`,
    estimatedOnly: (size) => `~${size} (stima del registro)`,
    rebootFlag: "riavvio richiesto",
    failedFlag: "non riuscita",
    restorePointLabel: "Punto di ripristino",
  },
  menu: {
    open: "Account e impostazioni",
    account: "Account",
    signIn: "Accedi / Registrati su pctweaker.app",
    signInHint:
      "Un solo account per tutta la suite PC Tweaker. La registrazione avviene su pctweaker.app e vale anche qui.",
    plans: "Uninstaller Pro",
    planMonthly: "3,99 € / mese",
    planAnnual: "13,99 € / anno",
    loyaltyTitle: "Prezzo fedeltà suite",
    loyaltyPrice: "4,99 € / anno",
    loyaltyHint:
      "Sei già abbonato a un piano PC Tweaker? Uninstaller Pro costa 4,99 €/anno sullo stesso account.",
    choosePlans: "Vedi i piani su pctweaker.app",
    language: "Lingua",
    theme: "Tema",
    openPcTweaker: "Apri PC Tweaker",
    emailLabel: "Email",
    passwordLabel: "Password",
    signInButton: "Accedi",
    signingIn: "Accesso in corso...",
    signOut: "Esci",
    proActive: "PC Tweaker Pro - attivo su questo account",
    proInactive: "Questo account non ha ancora PC Tweaker Pro.",
    loyaltyLocked: "Accedi con il tuo account PC Tweaker Pro per sbloccare il prezzo fedeltà.",
    checkFailed: "Impossibile verificare l'account in questo momento. Riprova tra poco.",
    registerHint: "Non hai un account? Creane uno su pctweaker.app.",
  },
  updater: {
    title: (version) => `Aggiornamento ${version} pronto`,
    body: "È disponibile una nuova build firmata. Si installa in pochi secondi e l'app si riavvia da sola.",
    install: "Installa ora",
    later: "Più tardi",
    downloading: (percent) => `Download… ${String(percent)}%`,
    installing: "Installazione…",
    error: (message) => `Impossibile installare l'aggiornamento: ${message}`,
  },
  errors: {
    generic: "Qualcosa è andato storto. Riprova.",
  },
};

const fr: Dictionary = {
  app: {
    title: "PC Tweaker Uninstaller",
    tagline:
      "Removal Intelligence pour Windows : supprimez vos logiciels avec clarté, pas au hasard.",
    suiteDetected: "PC Tweaker détecté - membre de la suite",
    suiteDetectedHint:
      "PC Tweaker est installé sur ce PC. Connectez-vous avec votre compte PC Tweaker Pro pour débloquer le prix fidélité.",
  },
  programs: {
    searchPlaceholder: "Rechercher par nom ou éditeur...",
    searchLabel: "Rechercher parmi les programmes installés",
    loading: "Lecture des programmes installés...",
    countSummary: (shown, total) =>
      shown === total
        ? `${String(total)} programmes`
        : `${String(shown)} sur ${String(total)} programmes`,
    statTotalSize: "sur le disque",
    emptyTitle: "Aucun programme trouvé",
    emptyBody:
      "Aucun programme désinstallable n'a été trouvé dans le registre Windows. C'est inhabituel : si vous pensez que c'est une erreur, signalez-le.",
    noMatchesTitle: "Aucun résultat",
    noMatchesBody: "Aucun programme installé ne correspond à votre recherche.",
    errorTitle: "Impossible de lire les programmes installés",
    retry: "Réessayer",
    columnProgram: "Programme",
    columnVersion: "Version",
    columnPublisher: "Éditeur",
    columnSize: "Taille",
    columnInstalled: "Installé",
    badgeMsi: "MSI",
    badgeExecutable: "EXE",
    badgeManualOnly: "Manuel",
    badgeNone: "Sans désinstalleur",
    badgeInvalid: "Entrée endommagée",
    badgeManualOnlyHint:
      "La commande de désinstallation passe par un interpréteur de scripts : par sécurité, elle doit être lancée manuellement.",
    badgeNoneHint: "Cette entrée de registre ne déclare aucune commande de désinstallation.",
    badgeInvalidHint: "La commande de désinstallation de ce programme est incompréhensible.",
    badgeUser: "Utilisateur",
    badgeUserHint: "Installé pour cet utilisateur uniquement, pas pour tout le PC.",
    badgeHidden: "Masqué",
    badgeHiddenHint:
      "Windows masque normalement cette entrée (composant système ou mise à jour). La supprimer peut affecter d'autres logiciels : soyez sûr de savoir ce que c'est.",
    badgeSuite: "Suite",
    badgeSuiteHint: "Fait partie de votre suite PC Tweaker.",
    filterAll: "Tous",
    filterLarge: "Volumineux",
    filterRecent: "Récents",
    showHidden: "Afficher masqués",
    detailSource: "Portée",
    detailKey: "Entrée de registre",
    detailLocation: "Dossier d'installation",
    detailNoLocation: "Non enregistré",
    openFolder: "Ouvrir le dossier",
    sourceMachine64: "Ce PC (64 bits)",
    sourceMachine32: "Ce PC (32 bits)",
    sourceUser: "Cet utilisateur uniquement",
  },
  confidence: {
    labelSafe: "Suppression sûre",
    labelReview: "À vérifier avant suppression",
    labelKeep: "À conserver - lié au système",
    disclaimer: "Basé sur des preuves visibles : un guide, pas une certitude.",
    reasons: {
      hiddenSystem: "Windows lui-même masque cette entrée (composant système ou mise à jour).",
      sharedRuntime: "Runtime partagé : d'autres programmes en dépendent probablement.",
      driverComponent: "Pilote ou chipset : sa suppression peut affecter le matériel.",
      sharedLauncher:
        "Launcher/boutique : les logiciels installés via celui-ci cesseraient de fonctionner.",
      noPublisher: "Aucun éditeur enregistré : origine invérifiable.",
      brokenUninstaller:
        "Sa commande de désinstallation est endommagée et ne peut pas s'exécuter automatiquement.",
      manualUninstaller: "Son désinstalleur doit être lancé manuellement (script).",
      noUninstaller: "Il ne déclare aucune commande de désinstallation.",
      namedPublisher: "Un éditeur nommé est enregistré.",
      standardUninstaller: "Il possède un désinstalleur standard MSI/EXE.",
    },
  },
  footer: {
    family: "Fait partie de la famille PC Tweaker",
    pcTweaker: "PC Tweaker",
    promptShield: "PromptShield",
    privacy: "Confidentialité",
    restoreInfo:
      "Les points de restauration sont créés et conservés par Windows sur le disque système (Protection du système).",
    openRestore: "Gérer les points de restauration",
  },
  uninstall: {
    action: "Désinstaller",
    confirmTitle: (name) => `Désinstaller ${name} ?`,
    confirmBody:
      "Exactement la commande ci-dessous sera exécutée, rien d'autre. Elle est reconstruite depuis le registre Windows et sera revérifiée au moment de l'exécution.",
    commandLabel: "Commande",
    methodLabel: "Méthode",
    methodMsi: "Windows Installer (silencieux)",
    methodExe: "Le désinstalleur du programme lui-même",
    privilegesLabel: "Privilèges",
    privilegesAdmin: "Administrateur (une demande UAC)",
    privilegesUser: "Utilisateur actuel",
    sizeLabel: "Espace estimé à récupérer",
    sizeUnknown: "Non enregistré",
    confidenceLabel: "Confiance",
    notRemovedNote:
      "Non supprimé automatiquement : fichiers, dossiers et entrées de registre résiduels. L'analyse des résidus arrivera dans une prochaine mise à jour et demandera toujours d'abord.",
    elevationNote: "Windows demandera d'abord l'approbation administrateur (UAC).",
    restorePointNote: "Un point de restauration système sera tenté avant toute exécution.",
    confirm: "Désinstaller",
    cancel: "Annuler",
    close: "Fermer",
    planning: "Vérification de ce qui serait exécuté...",
    planFailedTitle: "Désinstallation automatique impossible",
    running: (name) => `Désinstallation de ${name}...`,
    runningNote:
      "Le désinstalleur du programme est en cours. Cette fenêtre reste réactive ; certains désinstalleurs ouvrent leurs propres fenêtres.",
    reportSuccessTitle: "Désinstallé",
    reportFailureTitle: "Désinstallation non terminée",
    rebootNote: "Un redémarrage est nécessaire pour finir de supprimer les fichiers.",
    restorePointCreated: "Point de restauration : créé.",
    restorePointSkipped: (reason) => `Point de restauration : ignoré - ${reason}`,
    restorePointFailed: (reason) => `Point de restauration : non créé - ${reason}`,
    exitCodeLabel: "Code de sortie",
    residueScan: "Rechercher les restes",
    residueScanning: "Recherche des restes...",
    residueTitle: "Restes trouvés",
    residueNone: "Aucun reste trouvé. Désinstallation propre.",
    residueIntro: (count, mb) => `${String(count)} élément(s) restants, environ ${mb} Mo. Tout ce que vous sélectionnez va à la Corbeille et reste récupérable.`,
    residueClean: "Déplacer la sélection vers la Corbeille",
    residueRegistryNote: "Les clés de registre utilisateur sont supprimées directement (non récupérables) ; les clés machine sont listées à titre indicatif.",
    residueDone: (count, mb) => `${String(count)} élément(s) nettoyés, environ ${mb} Mo libérés.`,
    residueFailed: (count) => `${String(count)} élément(s) n'ont pas pu être supprimés.`,
    residueKinds: { "install-dir": "Dossier d'installation", "app-data": "Données d'application", shortcut: "Raccourci", "registry-user": "Registre (utilisateur)", "registry-machine": "Registre (machine)" },
    relDependentsWarning: (names) => `Sa suppression emporte aussi les programmes installés dans son dossier : ${names}.`,
    relInstalledVia: (name) => `Installé via ${name} — envisagez de le supprimer depuis là.`,
    relSiblings: (count) => `${String(count)} autre(s) programme(s) de cet éditeur sont installés.`,
    batchBar: (count, size) => `Désinstaller ${String(count)} sélectionnés · ${size}`,
    batchClear: "Vider la sélection",
    batchConfirmTitle: (count) => `Supprimer ${String(count)} programmes ?`,
    batchConfirmBody: "Ils s'exécutent un par un, les programmes contenus avant leurs conteneurs, avec les mêmes contrôles qu'une désinstallation simple. Les entrées protégées (composants système, runtimes partagés, votre suite PC Tweaker) ne sont pas sélectionnables.",
    batchNotBatchable: "Protégé — indisponible pour la suppression par lot.",
    batchRunningStep: (name, index, total) => `Désinstallation de ${name} (${String(index)} sur ${String(total)})...`,
    batchDoneTitle: "Lot terminé",
    batchFailedNote: (count) => `${String(count)} programme(s) inachevé(s) — voir le registre pour les détails.`,
    familyNote:
      "Cette application fait partie de votre suite PC Tweaker. Vous pouvez la supprimer, mais les fonctions de la suite qui en dépendent cesseront de fonctionner.",
    hiddenNote:
      "Windows masque normalement cette entrée. Supprimer des composants système ou des mises à jour peut affecter d'autres logiciels.",
  },
  ledger: {
    open: "Historique",
    title: "Registre des suppressions",
    subtitle:
      "Un reçu local pour chaque suppression effectuée par cette application, réussites comme échecs. Conservé sur ce PC, jamais envoyé en ligne.",
    empty: "Aucune suppression enregistrée. Votre première désinstallation laissera son reçu ici.",
    exportButton: "Exporter en JSON",
    exportedTo: (path) => `Exporté vers ${path}`,
    verifiedFreed: (size) => `${size} libérés (vérifié)`,
    estimatedOnly: (size) => `~${size} (estimation du registre)`,
    rebootFlag: "redémarrage requis",
    failedFlag: "échec",
    restorePointLabel: "Point de restauration",
  },
  menu: {
    open: "Compte et réglages",
    account: "Compte",
    signIn: "Se connecter / S'inscrire sur pctweaker.app",
    signInHint:
      "Un seul compte pour toute la suite PC Tweaker. L'inscription se fait sur pctweaker.app et vaut aussi ici.",
    plans: "Uninstaller Pro",
    planMonthly: "3,99 € / mois",
    planAnnual: "13,99 € / an",
    loyaltyTitle: "Prix fidélité de la suite",
    loyaltyPrice: "4,99 € / an",
    loyaltyHint:
      "Déjà abonné à un plan PC Tweaker ? Uninstaller Pro coûte 4,99 €/an sur le même compte.",
    choosePlans: "Voir les offres sur pctweaker.app",
    language: "Langue",
    theme: "Thème",
    openPcTweaker: "Ouvrir PC Tweaker",
    emailLabel: "E-mail",
    passwordLabel: "Mot de passe",
    signInButton: "Se connecter",
    signingIn: "Connexion...",
    signOut: "Se déconnecter",
    proActive: "PC Tweaker Pro - actif sur ce compte",
    proInactive: "Ce compte n'a pas encore PC Tweaker Pro.",
    loyaltyLocked:
      "Connectez-vous avec votre compte PC Tweaker Pro pour débloquer le prix fidélité.",
    checkFailed: "Impossible de vérifier le compte pour le moment. Réessayez bientôt.",
    registerHint: "Pas encore de compte ? Créez-en un sur pctweaker.app.",
  },
  updater: {
    title: (version) => `Mise à jour ${version} prête`,
    body: "Une nouvelle version signée est disponible. Elle s'installe en quelques secondes et l'application redémarre seule.",
    install: "Installer maintenant",
    later: "Plus tard",
    downloading: (percent) => `Téléchargement… ${String(percent)}%`,
    installing: "Installation…",
    error: (message) => `La mise à jour n'a pas pu être installée : ${message}`,
  },
  errors: {
    generic: "Une erreur s'est produite. Veuillez réessayer.",
  },
};

const es: Dictionary = {
  app: {
    title: "PC Tweaker Uninstaller",
    tagline: "Removal Intelligence para Windows: elimina software con claridad, no a ciegas.",
    suiteDetected: "PC Tweaker detectado - miembro de la suite",
    suiteDetectedHint:
      "PC Tweaker está instalado en este PC. Inicia sesión con tu cuenta PC Tweaker Pro para desbloquear el precio de fidelidad.",
  },
  programs: {
    searchPlaceholder: "Buscar por nombre o fabricante...",
    searchLabel: "Buscar entre los programas instalados",
    loading: "Leyendo los programas instalados...",
    countSummary: (shown, total) =>
      shown === total
        ? `${String(total)} programas`
        : `${String(shown)} de ${String(total)} programas`,
    statTotalSize: "en disco",
    emptyTitle: "No se encontraron programas",
    emptyBody:
      "No se encontraron programas desinstalables en el registro de Windows. Es inusual: si crees que es un error, infórmanos.",
    noMatchesTitle: "Sin coincidencias",
    noMatchesBody: "Ningún programa instalado coincide con tu búsqueda.",
    errorTitle: "No se pudieron leer los programas instalados",
    retry: "Reintentar",
    columnProgram: "Programa",
    columnVersion: "Versión",
    columnPublisher: "Fabricante",
    columnSize: "Tamaño",
    columnInstalled: "Instalado",
    badgeMsi: "MSI",
    badgeExecutable: "EXE",
    badgeManualOnly: "Manual",
    badgeNone: "Sin desinstalador",
    badgeInvalid: "Entrada dañada",
    badgeManualOnlyHint:
      "El comando de desinstalación usa un intérprete de scripts: por seguridad debe ejecutarse manualmente.",
    badgeNoneHint: "Esta entrada del registro no declara ningún comando de desinstalación.",
    badgeInvalidHint: "El comando de desinstalación de este programa no se pudo entender.",
    badgeUser: "Usuario",
    badgeUserHint: "Instalado solo para este usuario, no para todo el equipo.",
    badgeHidden: "Oculto",
    badgeHiddenHint:
      "Windows normalmente oculta esta entrada (componente del sistema o actualización). Quitarla puede afectar a otro software: asegúrate de saber qué es.",
    badgeSuite: "Suite",
    badgeSuiteHint: "Parte de tu suite PC Tweaker.",
    filterAll: "Todos",
    filterLarge: "Grandes",
    filterRecent: "Recientes",
    showHidden: "Mostrar ocultos",
    detailSource: "Ámbito",
    detailKey: "Entrada del registro",
    detailLocation: "Carpeta de instalación",
    detailNoLocation: "No registrada",
    openFolder: "Abrir carpeta",
    sourceMachine64: "Este PC (64 bits)",
    sourceMachine32: "Este PC (32 bits)",
    sourceUser: "Solo este usuario",
  },
  confidence: {
    labelSafe: "Eliminación segura",
    labelReview: "Revisar antes de eliminar",
    labelKeep: "Conservar - relacionado con el sistema",
    disclaimer: "Basado en evidencia visible: una guía, no una certeza.",
    reasons: {
      hiddenSystem:
        "El propio Windows oculta esta entrada (componente del sistema o actualización).",
      sharedRuntime: "Runtime compartido: es probable que otros programas dependan de él.",
      driverComponent: "Paquete de controlador o chipset: quitarlo puede afectar al hardware.",
      sharedLauncher: "Launcher/tienda: el software instalado a través de él dejaría de funcionar.",
      noPublisher: "Sin fabricante registrado: origen no verificable.",
      brokenUninstaller:
        "Su comando de desinstalación está dañado y no puede ejecutarse automáticamente.",
      manualUninstaller: "Su desinstalador debe ejecutarse manualmente (basado en scripts).",
      noUninstaller: "No declara ningún comando de desinstalación.",
      namedPublisher: "Hay un fabricante con nombre registrado.",
      standardUninstaller: "Tiene un desinstalador estándar MSI/EXE.",
    },
  },
  footer: {
    family: "Parte de la familia PC Tweaker",
    pcTweaker: "PC Tweaker",
    promptShield: "PromptShield",
    privacy: "Privacidad",
    restoreInfo:
      "Los puntos de restauración los crea y guarda Windows en la unidad del sistema (Protección del sistema).",
    openRestore: "Administrar puntos de restauración",
  },
  uninstall: {
    action: "Desinstalar",
    confirmTitle: (name) => `¿Desinstalar ${name}?`,
    confirmBody:
      "Se ejecutará exactamente el comando de abajo, nada más. Se reconstruyó desde el registro de Windows y se volverá a comprobar en el momento de ejecutarse.",
    commandLabel: "Comando",
    methodLabel: "Método",
    methodMsi: "Windows Installer (silencioso)",
    methodExe: "El desinstalador del propio programa",
    privilegesLabel: "Privilegios",
    privilegesAdmin: "Administrador (una solicitud UAC)",
    privilegesUser: "Usuario actual",
    sizeLabel: "Espacio estimado a recuperar",
    sizeUnknown: "No registrado",
    confidenceLabel: "Confianza",
    notRemovedNote:
      "No se elimina automáticamente: archivos, carpetas y entradas de registro residuales. El análisis de residuos llegará en una próxima actualización y siempre preguntará primero.",
    elevationNote: "Windows pedirá primero la aprobación de administrador (UAC).",
    restorePointNote: "Se intentará crear un punto de restauración antes de ejecutar nada.",
    confirm: "Desinstalar",
    cancel: "Cancelar",
    close: "Cerrar",
    planning: "Comprobando qué se ejecutaría...",
    planFailedTitle: "No se puede desinstalar automáticamente",
    running: (name) => `Desinstalando ${name}...`,
    runningNote:
      "El desinstalador del programa se está ejecutando. Esta ventana sigue respondiendo; algunos desinstaladores abren sus propias ventanas.",
    reportSuccessTitle: "Desinstalado",
    reportFailureTitle: "La desinstalación no se completó",
    rebootNote: "Se requiere un reinicio para terminar de eliminar los archivos.",
    restorePointCreated: "Punto de restauración: creado.",
    restorePointSkipped: (reason) => `Punto de restauración: omitido - ${reason}`,
    restorePointFailed: (reason) => `Punto de restauración: no creado - ${reason}`,
    exitCodeLabel: "Código de salida",
    residueScan: "Buscar restos",
    residueScanning: "Buscando restos...",
    residueTitle: "Restos encontrados",
    residueNone: "No se encontraron restos. Desinstalación limpia.",
    residueIntro: (count, mb) => `${String(count)} elemento(s) restantes, unos ${mb} MB. Todo lo que selecciones va a la Papelera y sigue siendo recuperable.`,
    residueClean: "Mover selección a la Papelera",
    residueRegistryNote: "Las claves de registro del usuario se eliminan directamente (no recuperables); las de sistema se listan solo como referencia.",
    residueDone: (count, mb) => `${String(count)} elemento(s) limpiados, unos ${mb} MB liberados.`,
    residueFailed: (count) => `${String(count)} elemento(s) no se pudieron eliminar.`,
    residueKinds: { "install-dir": "Carpeta de instalación", "app-data": "Datos de la aplicación", shortcut: "Acceso directo", "registry-user": "Registro (usuario)", "registry-machine": "Registro (equipo)" },
    relDependentsWarning: (names) => `Al eliminarlo también se eliminan los programas instalados en su carpeta: ${names}.`,
    relInstalledVia: (name) => `Instalado a través de ${name}; considera eliminarlo desde allí.`,
    relSiblings: (count) => `Hay ${String(count)} programa(s) más de este editor instalados.`,
    batchBar: (count, size) => `Desinstalar ${String(count)} seleccionados · ${size}`,
    batchClear: "Vaciar selección",
    batchConfirmTitle: (count) => `¿Eliminar ${String(count)} programas?`,
    batchConfirmBody: "Se ejecutan de uno en uno, los programas contenidos antes que sus contenedores, con los mismos controles que una desinstalación individual. Las entradas protegidas (componentes del sistema, runtimes compartidos, tu suite PC Tweaker) no se pueden seleccionar.",
    batchNotBatchable: "Protegido — no disponible para eliminación por lotes.",
    batchRunningStep: (name, index, total) => `Desinstalando ${name} (${String(index)} de ${String(total)})...`,
    batchDoneTitle: "Lote completado",
    batchFailedNote: (count) => `${String(count)} programa(s) no completados — consulta el registro para más detalles.`,
    familyNote:
      "Esta aplicación forma parte de tu suite PC Tweaker. Puedes eliminarla, pero las funciones de la suite que dependen de ella dejarán de funcionar.",
    hiddenNote:
      "Windows normalmente oculta esta entrada. Eliminar componentes del sistema o actualizaciones puede afectar a otro software.",
  },
  ledger: {
    open: "Historial",
    title: "Registro de eliminaciones",
    subtitle:
      "Un recibo local por cada eliminación que ejecutó esta aplicación, éxitos y fallos incluidos. Guardado en este PC, nunca se sube.",
    empty: "Aún no hay eliminaciones registradas. Tu primera desinstalación dejará aquí su recibo.",
    exportButton: "Exportar JSON",
    exportedTo: (path) => `Exportado a ${path}`,
    verifiedFreed: (size) => `${size} liberados (verificado)`,
    estimatedOnly: (size) => `~${size} (estimación del registro)`,
    rebootFlag: "reinicio requerido",
    failedFlag: "falló",
    restorePointLabel: "Punto de restauración",
  },
  menu: {
    open: "Cuenta y ajustes",
    account: "Cuenta",
    signIn: "Inicia sesión / Regístrate en pctweaker.app",
    signInHint:
      "Una sola cuenta para toda la suite PC Tweaker. El registro se hace en pctweaker.app y también vale aquí.",
    plans: "Uninstaller Pro",
    planMonthly: "3,99 € / mes",
    planAnnual: "13,99 € / año",
    loyaltyTitle: "Precio de fidelidad de la suite",
    loyaltyPrice: "4,99 € / año",
    loyaltyHint:
      "¿Ya estás suscrito a algún plan de PC Tweaker? Uninstaller Pro cuesta 4,99 €/año en la misma cuenta.",
    choosePlans: "Ver planes en pctweaker.app",
    language: "Idioma",
    theme: "Tema",
    openPcTweaker: "Abrir PC Tweaker",
    emailLabel: "Correo",
    passwordLabel: "Contraseña",
    signInButton: "Iniciar sesión",
    signingIn: "Iniciando sesión...",
    signOut: "Cerrar sesión",
    proActive: "PC Tweaker Pro - activo en esta cuenta",
    proInactive: "Esta cuenta aún no tiene PC Tweaker Pro.",
    loyaltyLocked:
      "Inicia sesión con tu cuenta PC Tweaker Pro para desbloquear el precio de fidelidad.",
    checkFailed: "No se pudo verificar la cuenta en este momento. Inténtalo de nuevo en breve.",
    registerHint: "¿Aún no tienes cuenta? Créala en pctweaker.app.",
  },
  updater: {
    title: (version) => `Actualización ${version} lista`,
    body: "Hay una nueva versión firmada disponible. Se instala en segundos y la aplicación se reinicia sola.",
    install: "Instalar ahora",
    later: "Más tarde",
    downloading: (percent) => `Descargando… ${String(percent)}%`,
    installing: "Instalando…",
    error: (message) => `No se pudo instalar la actualización: ${message}`,
  },
  errors: {
    generic: "Algo salió mal. Inténtalo de nuevo.",
  },
};

const de: Dictionary = {
  app: {
    title: "PC Tweaker Uninstaller",
    tagline:
      "Removal Intelligence für Windows: Software mit Klarheit entfernen, nicht auf gut Glück.",
    suiteDetected: "PC Tweaker erkannt - Suite-Mitglied",
    suiteDetectedHint:
      "PC Tweaker ist auf diesem PC installiert. Melde dich mit deinem PC-Tweaker-Pro-Konto an, um den Treuepreis freizuschalten.",
  },
  programs: {
    searchPlaceholder: "Nach Name oder Hersteller suchen...",
    searchLabel: "Installierte Programme durchsuchen",
    loading: "Installierte Programme werden gelesen...",
    countSummary: (shown, total) =>
      shown === total
        ? `${String(total)} Programme`
        : `${String(shown)} von ${String(total)} Programmen`,
    statTotalSize: "auf der Festplatte",
    emptyTitle: "Keine Programme gefunden",
    emptyBody:
      "In der Windows-Registrierung wurden keine deinstallierbaren Programme gefunden. Das ist ungewöhnlich - wenn du glaubst, dass das ein Fehler ist, melde es bitte.",
    noMatchesTitle: "Keine Treffer",
    noMatchesBody: "Kein installiertes Programm entspricht deiner Suche.",
    errorTitle: "Installierte Programme konnten nicht gelesen werden",
    retry: "Erneut versuchen",
    columnProgram: "Programm",
    columnVersion: "Version",
    columnPublisher: "Hersteller",
    columnSize: "Größe",
    columnInstalled: "Installiert",
    badgeMsi: "MSI",
    badgeExecutable: "EXE",
    badgeManualOnly: "Manuell",
    badgeNone: "Kein Uninstaller",
    badgeInvalid: "Defekter Eintrag",
    badgeManualOnlyHint:
      "Der Deinstallationsbefehl nutzt einen Skript-Interpreter und muss aus Sicherheitsgründen manuell ausgeführt werden.",
    badgeNoneHint: "Dieser Registrierungseintrag deklariert keinen Deinstallationsbefehl.",
    badgeInvalidHint: "Der Deinstallationsbefehl dieses Programms war unverständlich.",
    badgeUser: "Benutzer",
    badgeUserHint: "Nur für diesen Benutzer installiert, nicht PC-weit.",
    badgeHidden: "Verborgen",
    badgeHiddenHint:
      "Windows verbirgt diesen Eintrag normalerweise (Systemkomponente oder Unter-Update). Das Entfernen kann andere Software beeinträchtigen - sei sicher, dass du weißt, was es ist.",
    badgeSuite: "Suite",
    badgeSuiteHint: "Teil deiner PC-Tweaker-Suite.",
    filterAll: "Alle",
    filterLarge: "Groß",
    filterRecent: "Neu",
    showHidden: "Verborgene zeigen",
    detailSource: "Bereich",
    detailKey: "Registrierungseintrag",
    detailLocation: "Installationsordner",
    detailNoLocation: "Nicht erfasst",
    openFolder: "Ordner öffnen",
    sourceMachine64: "Dieser PC (64-Bit)",
    sourceMachine32: "Dieser PC (32-Bit)",
    sourceUser: "Nur dieser Benutzer",
  },
  confidence: {
    labelSafe: "Sicher entfernbar",
    labelReview: "Vor dem Entfernen prüfen",
    labelKeep: "Behalten - systemnah",
    disclaimer: "Basierend auf sichtbaren Belegen: ein Leitfaden, keine Gewissheit.",
    reasons: {
      hiddenSystem: "Windows selbst verbirgt diesen Eintrag (Systemkomponente oder Unter-Update).",
      sharedRuntime:
        "Gemeinsame Laufzeitumgebung: andere Programme hängen wahrscheinlich davon ab.",
      driverComponent:
        "Treiber- oder Chipsatzpaket: das Entfernen kann die Hardware beeinträchtigen.",
      sharedLauncher:
        "Launcher/Store: darüber installierte Software würde nicht mehr funktionieren.",
      noPublisher: "Kein Hersteller erfasst - Herkunft nicht überprüfbar.",
      brokenUninstaller:
        "Der Deinstallationsbefehl ist defekt und kann nicht automatisch ausgeführt werden.",
      manualUninstaller: "Der Uninstaller muss manuell ausgeführt werden (skriptbasiert).",
      noUninstaller: "Es ist überhaupt kein Deinstallationsbefehl deklariert.",
      namedPublisher: "Ein benannter Hersteller ist erfasst.",
      standardUninstaller: "Es gibt einen Standard-Uninstaller (MSI/EXE).",
    },
  },
  footer: {
    family: "Teil der PC-Tweaker-Familie",
    pcTweaker: "PC Tweaker",
    promptShield: "PromptShield",
    privacy: "Datenschutz",
    restoreInfo:
      "Wiederherstellungspunkte werden von Windows auf dem Systemlaufwerk erstellt und gespeichert (Computerschutz).",
    openRestore: "Wiederherstellungspunkte verwalten",
  },
  uninstall: {
    action: "Deinstallieren",
    confirmTitle: (name) => `${name} deinstallieren?`,
    confirmBody:
      "Es wird genau der folgende Befehl ausgeführt - nichts anderes. Er wurde aus der Windows-Registrierung neu aufgebaut und wird im Moment der Ausführung erneut geprüft.",
    commandLabel: "Befehl",
    methodLabel: "Methode",
    methodMsi: "Windows Installer (still)",
    methodExe: "Der Uninstaller des Programms selbst",
    privilegesLabel: "Berechtigungen",
    privilegesAdmin: "Administrator (eine UAC-Abfrage)",
    privilegesUser: "Aktueller Benutzer",
    sizeLabel: "Geschätzter freiwerdender Speicher",
    sizeUnknown: "Nicht erfasst",
    confidenceLabel: "Einschätzung",
    notRemovedNote:
      "Nicht automatisch entfernt: übrig gebliebene Dateien, Ordner und Registrierungseinträge. Die Rückstands-Analyse kommt in einem späteren Update und fragt immer zuerst.",
    elevationNote: "Windows fragt zuerst nach Administrator-Bestätigung (UAC).",
    restorePointNote: "Vor der Ausführung wird ein Systemwiederherstellungspunkt versucht.",
    confirm: "Deinstallieren",
    cancel: "Abbrechen",
    close: "Schließen",
    planning: "Es wird geprüft, was ausgeführt würde...",
    planFailedTitle: "Automatische Deinstallation nicht möglich",
    running: (name) => `${name} wird deinstalliert...`,
    runningNote:
      "Der Uninstaller des Programms läuft. Dieses Fenster bleibt bedienbar; manche Uninstaller öffnen eigene Fenster.",
    reportSuccessTitle: "Deinstalliert",
    reportFailureTitle: "Deinstallation nicht abgeschlossen",
    rebootNote: "Ein Neustart ist nötig, um die Dateien vollständig zu entfernen.",
    restorePointCreated: "Wiederherstellungspunkt: erstellt.",
    restorePointSkipped: (reason) => `Wiederherstellungspunkt: übersprungen - ${reason}`,
    restorePointFailed: (reason) => `Wiederherstellungspunkt: nicht erstellt - ${reason}`,
    exitCodeLabel: "Exit-Code",
    residueScan: "Nach Resten suchen",
    residueScanning: "Suche nach Resten...",
    residueTitle: "Reste gefunden",
    residueNone: "Keine Reste gefunden. Saubere Deinstallation.",
    residueIntro: (count, mb) => `${String(count)} Element(e) übrig, etwa ${mb} MB. Alles Ausgewählte wandert in den Papierkorb und bleibt wiederherstellbar.`,
    residueClean: "Auswahl in den Papierkorb verschieben",
    residueRegistryNote: "Benutzer-Registrierungsschlüssel werden direkt gelöscht (nicht wiederherstellbar); Maschinenschlüssel werden nur aufgelistet.",
    residueDone: (count, mb) => `${String(count)} Element(e) bereinigt, etwa ${mb} MB freigegeben.`,
    residueFailed: (count) => `${String(count)} Element(e) konnten nicht entfernt werden.`,
    residueKinds: { "install-dir": "Installationsordner", "app-data": "Anwendungsdaten", shortcut: "Verknüpfung", "registry-user": "Registrierung (Benutzer)", "registry-machine": "Registrierung (System)" },
    relDependentsWarning: (names) => `Beim Entfernen werden auch Programme in seinem Ordner entfernt: ${names}.`,
    relInstalledVia: (name) => `Installiert über ${name} — besser dort entfernen.`,
    relSiblings: (count) => `${String(count)} weitere(s) Programm(e) dieses Herausgebers sind installiert.`,
    batchBar: (count, size) => `${String(count)} ausgewählte deinstallieren · ${size}`,
    batchClear: "Auswahl leeren",
    batchConfirmTitle: (count) => `${String(count)} Programme entfernen?`,
    batchConfirmBody: "Sie laufen nacheinander, enthaltene Programme vor ihren Containern, mit denselben Prüfungen wie eine einzelne Deinstallation. Geschützte Einträge (Systemkomponenten, gemeinsame Runtimes, Ihre PC-Tweaker-Suite) sind nicht auswählbar.",
    batchNotBatchable: "Geschützt — nicht für Stapelentfernung verfügbar.",
    batchRunningStep: (name, index, total) => `Deinstalliere ${name} (${String(index)} von ${String(total)})...`,
    batchDoneTitle: "Stapel abgeschlossen",
    batchFailedNote: (count) => `${String(count)} Programm(e) nicht abgeschlossen — Details im Protokoll.`,
    familyNote:
      "Diese App gehört zu deiner PC-Tweaker-Suite. Du kannst sie entfernen, aber davon abhängige Suite-Funktionen hören auf zu funktionieren.",
    hiddenNote:
      "Windows verbirgt diesen Eintrag normalerweise. Das Entfernen von Systemkomponenten oder Unter-Updates kann andere Software beeinträchtigen.",
  },
  ledger: {
    open: "Verlauf",
    title: "Entfernungsprotokoll",
    subtitle:
      "Eine lokale Quittung für jede Entfernung, die diese App ausgeführt hat - Erfolge wie Fehlschläge. Auf diesem PC gespeichert, nie hochgeladen.",
    empty:
      "Noch keine Entfernungen aufgezeichnet. Deine erste Deinstallation hinterlässt hier ihre Quittung.",
    exportButton: "Als JSON exportieren",
    exportedTo: (path) => `Exportiert nach ${path}`,
    verifiedFreed: (size) => `${size} freigegeben (verifiziert)`,
    estimatedOnly: (size) => `~${size} (Registrierungs-Schätzung)`,
    rebootFlag: "Neustart erforderlich",
    failedFlag: "fehlgeschlagen",
    restorePointLabel: "Wiederherstellungspunkt",
  },
  menu: {
    open: "Konto & Einstellungen",
    account: "Konto",
    signIn: "Anmelden / Registrieren auf pctweaker.app",
    signInHint:
      "Ein Konto für die ganze PC-Tweaker-Suite. Die Registrierung erfolgt auf pctweaker.app und gilt auch hier.",
    plans: "Uninstaller Pro",
    planMonthly: "3,99 € / Monat",
    planAnnual: "13,99 € / Jahr",
    loyaltyTitle: "Suite-Treuepreis",
    loyaltyPrice: "4,99 € / Jahr",
    loyaltyHint:
      "Bereits ein PC-Tweaker-Abo? Uninstaller Pro kostet 4,99 €/Jahr auf demselben Konto.",
    choosePlans: "Tarife auf pctweaker.app ansehen",
    language: "Sprache",
    theme: "Design",
    openPcTweaker: "PC Tweaker öffnen",
    emailLabel: "E-Mail",
    passwordLabel: "Passwort",
    signInButton: "Anmelden",
    signingIn: "Anmeldung läuft...",
    signOut: "Abmelden",
    proActive: "PC Tweaker Pro - auf diesem Konto aktiv",
    proInactive: "Dieses Konto hat noch kein PC Tweaker Pro.",
    loyaltyLocked:
      "Melde dich mit deinem PC-Tweaker-Pro-Konto an, um den Treuepreis freizuschalten.",
    checkFailed: "Konto konnte gerade nicht überprüft werden. Versuche es gleich noch einmal.",
    registerHint: "Noch kein Konto? Erstelle eines auf pctweaker.app.",
  },
  updater: {
    title: (version) => `Update ${version} ist bereit`,
    body: "Ein neuer signierter Build ist verfügbar. Er installiert sich in Sekunden und die App startet von selbst neu.",
    install: "Jetzt installieren",
    later: "Später",
    downloading: (percent) => `Wird heruntergeladen… ${String(percent)}%`,
    installing: "Wird installiert…",
    error: (message) => `Das Update konnte nicht installiert werden: ${message}`,
  },
  errors: {
    generic: "Etwas ist schiefgelaufen. Bitte versuche es erneut.",
  },
};

export const dictionaries = { en, it, fr, es, de } as const;
export type Locale = keyof typeof dictionaries;

export const LOCALES: { code: Locale; native: string }[] = [
  { code: "en", native: "English" },
  { code: "it", native: "Italiano" },
  { code: "fr", native: "Français" },
  { code: "es", native: "Español" },
  { code: "de", native: "Deutsch" },
];

const LANG_KEY = "pcu-lang";

function initialLocale(): Locale {
  try {
    const stored = localStorage.getItem(LANG_KEY);
    if (stored !== null && stored in dictionaries) return stored as Locale;
  } catch {
    // localStorage unavailable: default stands.
  }
  return "en";
}

let activeLocale: Locale = initialLocale();

/** Active dictionary. Reassigned by `setLocale` BEFORE the app re-renders, so
 *  every render reads a consistent language. */
export let text: Dictionary = dictionaries[activeLocale];

export function currentLocale(): Locale {
  return activeLocale;
}

export function setLocale(locale: Locale): void {
  activeLocale = locale;
  text = dictionaries[locale];
  try {
    localStorage.setItem(LANG_KEY, locale);
  } catch {
    // Non-fatal: the choice just won't persist.
  }
}
