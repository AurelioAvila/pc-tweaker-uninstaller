import type { Locale } from "./i18n";
import type { Blocker, GroupId, LeftoverKind, RestoreKind, Tier } from "./leftovers";

/**
 * Text for the removed-programs list and the leftovers dialog. Rust sends
 * only codes (kind, tier, evidence, blocker, step); every sentence lives
 * here. Windows' own words are used in each language: Recycle Bin / Cestino /
 * Corbeille / Papelera de reciclaje / Papierkorb / Reciclagem.
 *
 * French punctuation before ? and : uses U+00A0, written as an escape so it
 * can never be turned into a plain space by an editor.
 */
type Copy = {
  removedButton: string;
  removedTitle: string;
  removedIntro: string;
  removedEmpty: string;
  howApp: string;
  howOutside: string;
  removedOn: (date: string) => string;
  scanThis: string;
  manualTitle: string;
  manualIntro: string;
  manualName: string;
  manualFolder: string;
  manualFolderHint: string;
  manualScan: string;
  sweepButton: string;
  outsidePrompt: (name: string) => string;
  outsideScan: string;
  notNow: string;
  cardTitle: string;
  cardBody: (count: number) => string;
  cardReview: string;
  titleFound: string;
  titleSweep: string;
  noteManual: string;
  noteSweep: string;
  teaser: (count: number, size: string) => string;
  teaserStartup: (count: number) => string;
  groups: Record<GroupId, string>;
  tiers: Record<Tier, string>;
  kinds: Record<LeftoverKind, string>;
  scope: Record<"user" | "machine", string>;
  restore: Record<RestoreKind, string>;
  proofPath: (name: string) => string;
  proofName: string;
  proofDangling: (missing: string) => string;
  blockers: Record<Blocker, string>;
  keptHint: string;
  selectReview: string;
  proNote: string;
  preview: string;
  previewRecycle: string;
  previewRegFile: string;
  previewLocked: string;
  previewRefused: string;
  previewNone: string;
  previewUnchanged: string;
  previewBack: string;
  clean: string;
  selected: (count: number) => string;
  lockedLine: (count: number) => string;
  reportOnly: string;
};

const en: Copy = {
  removedButton: "Removed programs",
  removedTitle: "Programs removed earlier",
  removedIntro:
    "Programs that disappeared from the installed list, whether this app removed them or something else did. Scan one to see what it left behind.",
  removedEmpty: "Nothing here yet. When a program is removed, it shows up in this list.",
  howApp: "Removed with this app",
  howOutside: "Removed outside this app",
  removedOn: (date) => `Removed ${date}`,
  scanThis: "Scan for leftovers",
  manualTitle: "Scan a removed program",
  manualIntro:
    "Removed a program some other way? Type its name, its old folder, or both. Nothing found this way is selected for you.",
  manualName: "Program name",
  manualFolder: "Folder it was installed in",
  manualFolderHint: "A full path on a local drive, for example C:\\Games\\Name",
  manualScan: "Scan",
  sweepButton: "Check startup entries",
  outsidePrompt: (name) => `${name} was removed outside this app. Scan for leftovers?`,
  outsideScan: "Scan for leftovers",
  notNow: "Not now",
  cardTitle: "Leftovers from programs already gone",
  cardBody: (count) =>
    count === 1
      ? "1 startup entry points to a program that no longer exists."
      : `${String(count)} startup entries point to programs that no longer exist.`,
  cardReview: "Review",
  titleFound: "Leftovers found",
  titleSweep: "Startup entries of programs already gone",
  noteManual:
    "You named this program yourself, so nothing is selected for you. Check each item before you clean.",
  noteSweep: "These startup entries run a file that no longer exists. Nothing is selected for you.",
  teaser: (count, size) => `${String(count)} ${count === 1 ? "leftover" : "leftovers"} · ${size}`,
  teaserStartup: (count) =>
    `${String(count)} still ${count === 1 ? "starts" : "start"} with Windows`,
  groups: {
    files: "Files and folders",
    startup: "Still starts with Windows",
    registry: "Registry keys",
    kept: "Kept for safety",
  },
  tiers: { safe: "Safe", review: "Review", keep: "Kept" },
  kinds: {
    "install-dir": "Install folder",
    "app-data": "App data",
    shortcut: "Shortcut",
    "registry-user": "Registry key (user)",
    "registry-machine": "Registry key (machine)",
    "startup-run": "Startup entry",
    "startup-shortcut": "Startup shortcut",
  },
  scope: { user: "This user", machine: "All users" },
  restore: { recycle: "Recycle Bin", regFile: "Saved as a .reg file first", none: "Listed only" },
  proofPath: (name) => `Inside the install folder recorded for ${name}`,
  proofName: "Matched by name only",
  proofDangling: (missing) => `Runs ${missing}, which no longer exists`,
  blockers: {
    windowsFolder: "Inside the Windows folder",
    protectedFolder: "A protected or shared folder",
    otherProgram: "Another installed program lives here",
    linked: "Reached through a link",
    notRestorable: "Cannot go to the Recycle Bin, so it could not be undone",
    wellKnownKey: "A shared registry location",
  },
  keptHint: "Not offered for cleaning. If you are sure it is a leftover, remove it yourself.",
  selectReview: "Select all to review",
  proNote: "Files and folders are free to clean. Registry keys and startup entries need Pro.",
  preview: "Preview changes",
  previewRecycle: "Move to the Recycle Bin",
  previewRegFile: "Save a .reg copy, then delete",
  previewLocked: "Needs Pro. Skipped.",
  previewRefused: "Not allowed. Skipped.",
  previewNone: "Select something to preview.",
  previewUnchanged: "Nothing has been changed yet.",
  previewBack: "Back",
  clean: "Clean selected",
  selected: (count) => `${String(count)} selected`,
  lockedLine: (count) =>
    `${String(count)} ${count === 1 ? "item needs" : "items need"} Pro and ${count === 1 ? "was" : "were"} not touched.`,
  reportOnly: "Listed only. This app never changes machine-wide keys.",
};

const it: Copy = {
  removedButton: "Programmi rimossi",
  removedTitle: "Programmi rimossi in precedenza",
  removedIntro:
    "Programmi scomparsi dall'elenco di quelli installati, rimossi da questa app o in altro modo. Cercane i residui per vedere cosa hanno lasciato.",
  removedEmpty: "Per ora è vuoto. Quando un programma viene rimosso, compare qui.",
  howApp: "Rimosso con questa app",
  howOutside: "Rimosso fuori da questa app",
  removedOn: (date) => `Rimosso il ${date}`,
  scanThis: "Cerca residui",
  manualTitle: "Cerca i residui di un programma rimosso",
  manualIntro:
    "Hai rimosso un programma in un altro modo? Scrivi il nome, la vecchia cartella o entrambi. Nulla di ciò che trovi così viene selezionato al posto tuo.",
  manualName: "Nome del programma",
  manualFolder: "Cartella di installazione",
  manualFolderHint: "Un percorso completo su un'unità locale, ad esempio C:\\Giochi\\Nome",
  manualScan: "Cerca",
  sweepButton: "Controlla le voci di avvio",
  outsidePrompt: (name) => `${name} è stato rimosso fuori da questa app. Cercare i residui?`,
  outsideScan: "Cerca residui",
  notNow: "Non ora",
  cardTitle: "Residui di programmi già rimossi",
  cardBody: (count) =>
    count === 1
      ? "1 voce di avvio punta a un programma che non esiste più."
      : `${String(count)} voci di avvio puntano a programmi che non esistono più.`,
  cardReview: "Controlla",
  titleFound: "Residui trovati",
  titleSweep: "Voci di avvio di programmi già rimossi",
  noteManual:
    "Hai indicato tu questo programma, quindi nulla è selezionato in anticipo. Controlla ogni voce prima di pulire.",
  noteSweep:
    "Queste voci di avvio eseguono un file che non esiste più. Nulla è selezionato in anticipo.",
  teaser: (count, size) => `${String(count)} ${count === 1 ? "residuo" : "residui"} · ${size}`,
  teaserStartup: (count) => `${String(count)} ${count === 1 ? "parte" : "partono"} con Windows`,
  groups: {
    files: "File e cartelle",
    startup: "Partono con Windows",
    registry: "Chiavi di registro",
    kept: "Conservati per sicurezza",
  },
  tiers: { safe: "Sicuro", review: "Da controllare", keep: "Conservato" },
  kinds: {
    "install-dir": "Cartella di installazione",
    "app-data": "Dati dell'app",
    shortcut: "Collegamento",
    "registry-user": "Chiave di registro (utente)",
    "registry-machine": "Chiave di registro (computer)",
    "startup-run": "Voce di avvio",
    "startup-shortcut": "Collegamento di avvio",
  },
  scope: { user: "Questo utente", machine: "Tutti gli utenti" },
  restore: { recycle: "Cestino", regFile: "Prima salvato in un file .reg", none: "Solo elenco" },
  proofPath: (name) => `Nella cartella di installazione registrata per ${name}`,
  proofName: "Trovato solo per nome",
  proofDangling: (missing) => `Avvia ${missing}, che non esiste più`,
  blockers: {
    windowsFolder: "Dentro la cartella di Windows",
    protectedFolder: "Cartella protetta o condivisa",
    otherProgram: "Qui c'è un altro programma installato",
    linked: "Raggiunta tramite un collegamento",
    notRestorable: "Non può andare nel Cestino, quindi non si potrebbe annullare",
    wellKnownKey: "Posizione condivisa del registro",
  },
  keptHint:
    "Non viene proposta per la pulizia. Se sei sicuro che sia un residuo, rimuovila a mano.",
  selectReview: "Seleziona tutto da controllare",
  proNote:
    "File e cartelle si puliscono gratis. Chiavi di registro e voci di avvio richiedono Pro.",
  preview: "Anteprima delle modifiche",
  previewRecycle: "Sposta nel Cestino",
  previewRegFile: "Salva una copia .reg, poi elimina",
  previewLocked: "Richiede Pro. Saltato.",
  previewRefused: "Non consentito. Saltato.",
  previewNone: "Seleziona qualcosa per vedere l'anteprima.",
  previewUnchanged: "Finora non è stato modificato nulla.",
  previewBack: "Indietro",
  clean: "Pulisci la selezione",
  selected: (count) => `${String(count)} ${count === 1 ? "selezionato" : "selezionati"}`,
  lockedLine: (count) =>
    `${String(count)} ${count === 1 ? "elemento richiede" : "elementi richiedono"} Pro e ${count === 1 ? "non è stato toccato" : "non sono stati toccati"}.`,
  reportOnly: "Solo elenco. Questa app non modifica le chiavi valide per tutto il computer.",
};

const fr: Copy = {
  removedButton: "Programmes supprimés",
  removedTitle: "Programmes supprimés précédemment",
  removedIntro:
    "Programmes disparus de la liste des programmes installés, supprimés par cette application ou autrement. Recherchez leurs restes pour voir ce qu'ils ont laissé.",
  removedEmpty: "Rien pour l'instant. Quand un programme est supprimé, il apparaît ici.",
  howApp: "Supprimé avec cette application",
  howOutside: "Supprimé en dehors de cette application",
  removedOn: (date) => `Supprimé le ${date}`,
  scanThis: "Rechercher les restes",
  manualTitle: "Rechercher les restes d'un programme supprimé",
  manualIntro:
    "Vous avez supprimé un programme autrement\u00a0? Saisissez son nom, son ancien dossier, ou les deux. Rien de ce qui est trouvé ainsi n'est sélectionné à votre place.",
  manualName: "Nom du programme",
  manualFolder: "Dossier d'installation",
  manualFolderHint: "Un chemin complet sur un lecteur local, par exemple C:\\Jeux\\Nom",
  manualScan: "Rechercher",
  sweepButton: "Vérifier les entrées de démarrage",
  outsidePrompt: (name) =>
    `${name} a été supprimé en dehors de cette application. Rechercher les restes\u00a0?`,
  outsideScan: "Rechercher les restes",
  notNow: "Plus tard",
  cardTitle: "Restes de programmes déjà supprimés",
  cardBody: (count) =>
    count === 1
      ? "1 entrée de démarrage pointe vers un programme qui n'existe plus."
      : `${String(count)} entrées de démarrage pointent vers des programmes qui n'existent plus.`,
  cardReview: "Examiner",
  titleFound: "Restes trouvés",
  titleSweep: "Entrées de démarrage de programmes déjà supprimés",
  noteManual:
    "Vous avez désigné ce programme vous-même, donc rien n'est sélectionné d'avance. Vérifiez chaque élément avant de nettoyer.",
  noteSweep:
    "Ces entrées de démarrage lancent un fichier qui n'existe plus. Rien n'est sélectionné d'avance.",
  teaser: (count, size) => `${String(count)} ${count <= 1 ? "reste" : "restes"} · ${size}`,
  teaserStartup: (count) =>
    `${String(count)} ${count === 1 ? "démarre" : "démarrent"} avec Windows`,
  groups: {
    files: "Fichiers et dossiers",
    startup: "Démarrent avec Windows",
    registry: "Clés de registre",
    kept: "Conservés par sécurité",
  },
  tiers: { safe: "Sûr", review: "À examiner", keep: "Conservé" },
  kinds: {
    "install-dir": "Dossier d'installation",
    "app-data": "Données de l'application",
    shortcut: "Raccourci",
    "registry-user": "Clé de registre (utilisateur)",
    "registry-machine": "Clé de registre (ordinateur)",
    "startup-run": "Entrée de démarrage",
    "startup-shortcut": "Raccourci de démarrage",
  },
  scope: { user: "Cet utilisateur", machine: "Tous les utilisateurs" },
  restore: {
    recycle: "Corbeille",
    regFile: "D'abord enregistré dans un fichier .reg",
    none: "Liste seulement",
  },
  proofPath: (name) => `Dans le dossier d'installation enregistré pour ${name}`,
  proofName: "Trouvé par le nom seulement",
  proofDangling: (missing) => `Lance ${missing}, qui n'existe plus`,
  blockers: {
    windowsFolder: "Dans le dossier Windows",
    protectedFolder: "Dossier protégé ou partagé",
    otherProgram: "Un autre programme installé se trouve ici",
    linked: "Atteint par un lien",
    notRestorable: "Ne peut pas aller dans la Corbeille, donc ne pourrait pas être annulé",
    wellKnownKey: "Emplacement de registre partagé",
  },
  keptHint:
    "Non proposé au nettoyage. Si vous êtes sûr qu'il s'agit d'un reste, supprimez-le vous-même.",
  selectReview: "Sélectionner tout ce qui est à examiner",
  proNote:
    "Les fichiers et dossiers se nettoient gratuitement. Les clés de registre et les entrées de démarrage nécessitent Pro.",
  preview: "Aperçu des modifications",
  previewRecycle: "Déplacer vers la Corbeille",
  previewRegFile: "Enregistrer une copie .reg, puis supprimer",
  previewLocked: "Nécessite Pro. Ignoré.",
  previewRefused: "Non autorisé. Ignoré.",
  previewNone: "Sélectionnez quelque chose pour voir l'aperçu.",
  previewUnchanged: "Rien n'a encore été modifié.",
  previewBack: "Retour",
  clean: "Nettoyer la sélection",
  selected: (count) => `${String(count)} ${count > 1 ? "sélectionnés" : "sélectionné"}`,
  lockedLine: (count) =>
    `${String(count)} ${count > 1 ? "éléments nécessitent" : "élément nécessite"} Pro et ${count > 1 ? "n'ont pas été touchés" : "n'a pas été touché"}.`,
  reportOnly:
    "Liste seulement. Cette application ne modifie jamais les clés valables pour tout l'ordinateur.",
};

const es: Copy = {
  removedButton: "Programas eliminados",
  removedTitle: "Programas eliminados antes",
  removedIntro:
    "Programas que desaparecieron de la lista de instalados, eliminados con esta aplicación o de otra forma. Busca sus restos para ver qué dejaron.",
  removedEmpty: "Aún no hay nada. Cuando se elimine un programa, aparecerá aquí.",
  howApp: "Eliminado con esta aplicación",
  howOutside: "Eliminado fuera de esta aplicación",
  removedOn: (date) => `Eliminado el ${date}`,
  scanThis: "Buscar restos",
  manualTitle: "Buscar los restos de un programa eliminado",
  manualIntro:
    "¿Eliminaste un programa de otra forma? Escribe su nombre, su antigua carpeta o ambos. Nada de lo que se encuentre así se selecciona por ti.",
  manualName: "Nombre del programa",
  manualFolder: "Carpeta de instalación",
  manualFolderHint: "Una ruta completa en una unidad local, por ejemplo C:\\Juegos\\Nombre",
  manualScan: "Buscar",
  sweepButton: "Revisar las entradas de inicio",
  outsidePrompt: (name) => `${name} se eliminó fuera de esta aplicación. ¿Buscar restos?`,
  outsideScan: "Buscar restos",
  notNow: "Ahora no",
  cardTitle: "Restos de programas ya eliminados",
  cardBody: (count) =>
    count === 1
      ? "1 entrada de inicio apunta a un programa que ya no existe."
      : `${String(count)} entradas de inicio apuntan a programas que ya no existen.`,
  cardReview: "Revisar",
  titleFound: "Restos encontrados",
  titleSweep: "Entradas de inicio de programas ya eliminados",
  noteManual:
    "Tú indicaste este programa, así que no hay nada seleccionado de antemano. Revisa cada elemento antes de limpiar.",
  noteSweep:
    "Estas entradas de inicio ejecutan un archivo que ya no existe. No hay nada seleccionado de antemano.",
  teaser: (count, size) => `${String(count)} ${count === 1 ? "resto" : "restos"} · ${size}`,
  teaserStartup: (count) =>
    `${String(count)} ${count === 1 ? "se inicia" : "se inician"} con Windows`,
  groups: {
    files: "Archivos y carpetas",
    startup: "Se inician con Windows",
    registry: "Claves del registro",
    kept: "Conservados por seguridad",
  },
  tiers: { safe: "Seguro", review: "Revisar", keep: "Conservado" },
  kinds: {
    "install-dir": "Carpeta de instalación",
    "app-data": "Datos de la aplicación",
    shortcut: "Acceso directo",
    "registry-user": "Clave del registro (usuario)",
    "registry-machine": "Clave del registro (equipo)",
    "startup-run": "Entrada de inicio",
    "startup-shortcut": "Acceso directo de inicio",
  },
  scope: { user: "Este usuario", machine: "Todos los usuarios" },
  restore: {
    recycle: "Papelera de reciclaje",
    regFile: "Antes se guarda en un archivo .reg",
    none: "Solo en la lista",
  },
  proofPath: (name) => `Dentro de la carpeta de instalación registrada para ${name}`,
  proofName: "Encontrado solo por el nombre",
  proofDangling: (missing) => `Ejecuta ${missing}, que ya no existe`,
  blockers: {
    windowsFolder: "Dentro de la carpeta de Windows",
    protectedFolder: "Carpeta protegida o compartida",
    otherProgram: "Aquí hay otro programa instalado",
    linked: "Se llega a ella mediante un vínculo",
    notRestorable: "No puede ir a la Papelera de reciclaje, así que no se podría deshacer",
    wellKnownKey: "Ubicación compartida del registro",
  },
  keptHint: "No se ofrece para limpiar. Si estás seguro de que es un resto, elimínalo tú mismo.",
  selectReview: "Seleccionar todo lo que hay que revisar",
  proNote:
    "Limpiar archivos y carpetas es gratis. Las claves del registro y las entradas de inicio requieren Pro.",
  preview: "Vista previa de los cambios",
  previewRecycle: "Mover a la Papelera de reciclaje",
  previewRegFile: "Guardar una copia .reg y después eliminar",
  previewLocked: "Requiere Pro. Se omite.",
  previewRefused: "No permitido. Se omite.",
  previewNone: "Selecciona algo para ver la vista previa.",
  previewUnchanged: "Todavía no se ha cambiado nada.",
  previewBack: "Volver",
  clean: "Limpiar la selección",
  selected: (count) => `${String(count)} ${count === 1 ? "seleccionado" : "seleccionados"}`,
  lockedLine: (count) =>
    `${String(count)} ${count === 1 ? "elemento requiere" : "elementos requieren"} Pro y no se ${count === 1 ? "tocó" : "tocaron"}.`,
  reportOnly: "Solo en la lista. Esta aplicación nunca modifica las claves de todo el equipo.",
};

const de: Copy = {
  removedButton: "Entfernte Programme",
  removedTitle: "Früher entfernte Programme",
  removedIntro:
    "Programme, die aus der Liste der installierten Programme verschwunden sind – von dieser App entfernt oder auf anderem Weg. Such nach Resten, um zu sehen, was sie hinterlassen haben.",
  removedEmpty: "Noch nichts. Sobald ein Programm entfernt wird, erscheint es hier.",
  howApp: "Mit dieser App entfernt",
  howOutside: "Außerhalb dieser App entfernt",
  removedOn: (date) => `Entfernt am ${date}`,
  scanThis: "Nach Resten suchen",
  manualTitle: "Reste eines entfernten Programms suchen",
  manualIntro:
    "Du hast ein Programm auf anderem Weg entfernt? Gib den Namen, den alten Ordner oder beides ein. Nichts, was so gefunden wird, wird für dich vorausgewählt.",
  manualName: "Programmname",
  manualFolder: "Installationsordner",
  manualFolderHint:
    "Ein vollständiger Pfad auf einem lokalen Laufwerk, zum Beispiel C:\\Spiele\\Name",
  manualScan: "Suchen",
  sweepButton: "Autostart-Einträge prüfen",
  outsidePrompt: (name) => `${name} wurde außerhalb dieser App entfernt. Nach Resten suchen?`,
  outsideScan: "Nach Resten suchen",
  notNow: "Jetzt nicht",
  cardTitle: "Reste bereits entfernter Programme",
  cardBody: (count) =>
    count === 1
      ? "1 Autostart-Eintrag zeigt auf ein Programm, das es nicht mehr gibt."
      : `${String(count)} Autostart-Einträge zeigen auf Programme, die es nicht mehr gibt.`,
  cardReview: "Prüfen",
  titleFound: "Reste gefunden",
  titleSweep: "Autostart-Einträge bereits entfernter Programme",
  noteManual:
    "Du hast dieses Programm selbst angegeben, daher ist nichts vorausgewählt. Prüfe jeden Eintrag, bevor du bereinigst.",
  noteSweep:
    "Diese Autostart-Einträge starten eine Datei, die es nicht mehr gibt. Nichts ist vorausgewählt.",
  teaser: (count, size) => `${String(count)} ${count === 1 ? "Rest" : "Reste"} · ${size}`,
  teaserStartup: (count) => `${String(count)} ${count === 1 ? "startet" : "starten"} mit Windows`,
  groups: {
    files: "Dateien und Ordner",
    startup: "Starten mit Windows",
    registry: "Registrierungsschlüssel",
    kept: "Zur Sicherheit behalten",
  },
  tiers: { safe: "Sicher", review: "Prüfen", keep: "Behalten" },
  kinds: {
    "install-dir": "Installationsordner",
    "app-data": "App-Daten",
    shortcut: "Verknüpfung",
    "registry-user": "Registrierungsschlüssel (Benutzer)",
    "registry-machine": "Registrierungsschlüssel (Computer)",
    "startup-run": "Autostart-Eintrag",
    "startup-shortcut": "Autostart-Verknüpfung",
  },
  scope: { user: "Dieser Benutzer", machine: "Alle Benutzer" },
  restore: {
    recycle: "Papierkorb",
    regFile: "Vorher als .reg-Datei gesichert",
    none: "Nur aufgelistet",
  },
  proofPath: (name) => `Im Installationsordner, der für ${name} eingetragen war`,
  proofName: "Nur über den Namen gefunden",
  proofDangling: (missing) => `Startet ${missing}, die es nicht mehr gibt`,
  blockers: {
    windowsFolder: "Im Windows-Ordner",
    protectedFolder: "Geschützter oder gemeinsam genutzter Ordner",
    otherProgram: "Hier liegt ein anderes installiertes Programm",
    linked: "Über einen Link erreicht",
    notRestorable: "Kann nicht in den Papierkorb, wäre also nicht rückgängig zu machen",
    wellKnownKey: "Gemeinsam genutzter Registrierungsort",
  },
  keptHint:
    "Wird nicht zum Bereinigen angeboten. Wenn du sicher bist, dass es ein Rest ist, entferne ihn selbst.",
  selectReview: "Alles zum Prüfen auswählen",
  proNote:
    "Dateien und Ordner bereinigst du kostenlos. Registrierungsschlüssel und Autostart-Einträge brauchen Pro.",
  preview: "Änderungen ansehen",
  previewRecycle: "In den Papierkorb verschieben",
  previewRegFile: ".reg-Kopie speichern, dann löschen",
  previewLocked: "Braucht Pro. Übersprungen.",
  previewRefused: "Nicht erlaubt. Übersprungen.",
  previewNone: "Wähle etwas aus, um die Vorschau zu sehen.",
  previewUnchanged: "Bisher wurde nichts geändert.",
  previewBack: "Zurück",
  clean: "Auswahl bereinigen",
  selected: (count) => `${String(count)} ausgewählt`,
  lockedLine: (count) =>
    `${String(count)} ${count === 1 ? "Eintrag braucht" : "Einträge brauchen"} Pro und ${count === 1 ? "wurde" : "wurden"} nicht angefasst.`,
  reportOnly:
    "Nur aufgelistet. Diese App ändert nie Schlüssel, die für den ganzen Computer gelten.",
};

const pt: Copy = {
  removedButton: "Programas removidos",
  removedTitle: "Programas removidos anteriormente",
  removedIntro:
    "Programas que desapareceram da lista de instalados, removidos por esta aplicação ou de outra forma. Procure os resíduos para ver o que deixaram.",
  removedEmpty: "Ainda não há nada. Quando um programa for removido, aparece aqui.",
  howApp: "Removido com esta aplicação",
  howOutside: "Removido fora desta aplicação",
  removedOn: (date) => `Removido em ${date}`,
  scanThis: "Procurar resíduos",
  manualTitle: "Procurar os resíduos de um programa removido",
  manualIntro:
    "Removeu um programa de outra forma? Escreva o nome, a pasta antiga ou ambos. Nada do que for encontrado assim é selecionado por si.",
  manualName: "Nome do programa",
  manualFolder: "Pasta de instalação",
  manualFolderHint: "Um caminho completo numa unidade local, por exemplo C:\\Jogos\\Nome",
  manualScan: "Procurar",
  sweepButton: "Verificar as entradas de arranque",
  outsidePrompt: (name) => `${name} foi removido fora desta aplicação. Procurar resíduos?`,
  outsideScan: "Procurar resíduos",
  notNow: "Agora não",
  cardTitle: "Resíduos de programas já removidos",
  cardBody: (count) =>
    count === 1
      ? "1 entrada de arranque aponta para um programa que já não existe."
      : `${String(count)} entradas de arranque apontam para programas que já não existem.`,
  cardReview: "Rever",
  titleFound: "Resíduos encontrados",
  titleSweep: "Entradas de arranque de programas já removidos",
  noteManual:
    "Indicou este programa, por isso nada está selecionado à partida. Reveja cada item antes de limpar.",
  noteSweep:
    "Estas entradas de arranque executam um ficheiro que já não existe. Nada está selecionado à partida.",
  teaser: (count, size) => `${String(count)} ${count === 1 ? "resíduo" : "resíduos"} · ${size}`,
  teaserStartup: (count) =>
    `${String(count)} ${count === 1 ? "arranca" : "arrancam"} com o Windows`,
  groups: {
    files: "Ficheiros e pastas",
    startup: "Arrancam com o Windows",
    registry: "Chaves do registo",
    kept: "Mantidos por segurança",
  },
  tiers: { safe: "Seguro", review: "Rever", keep: "Mantido" },
  kinds: {
    "install-dir": "Pasta de instalação",
    "app-data": "Dados da aplicação",
    shortcut: "Atalho",
    "registry-user": "Chave do registo (utilizador)",
    "registry-machine": "Chave do registo (computador)",
    "startup-run": "Entrada de arranque",
    "startup-shortcut": "Atalho de arranque",
  },
  scope: { user: "Este utilizador", machine: "Todos os utilizadores" },
  restore: {
    recycle: "Reciclagem",
    regFile: "Antes é guardado num ficheiro .reg",
    none: "Apenas listado",
  },
  proofPath: (name) => `Dentro da pasta de instalação registada para ${name}`,
  proofName: "Encontrado apenas pelo nome",
  proofDangling: (missing) => `Executa ${missing}, que já não existe`,
  blockers: {
    windowsFolder: "Dentro da pasta do Windows",
    protectedFolder: "Pasta protegida ou partilhada",
    otherProgram: "Aqui está outro programa instalado",
    linked: "Alcançada através de uma ligação",
    notRestorable: "Não pode ir para a Reciclagem, por isso não poderia ser anulado",
    wellKnownKey: "Localização partilhada do registo",
  },
  keptHint:
    "Não é proposto para limpeza. Se tiver a certeza de que é um resíduo, remova-o você mesmo.",
  selectReview: "Selecionar tudo o que há a rever",
  proNote:
    "Limpar ficheiros e pastas é grátis. As chaves do registo e as entradas de arranque requerem Pro.",
  preview: "Pré-visualizar as alterações",
  previewRecycle: "Mover para a Reciclagem",
  previewRegFile: "Guardar uma cópia .reg e depois eliminar",
  previewLocked: "Requer Pro. Ignorado.",
  previewRefused: "Não permitido. Ignorado.",
  previewNone: "Selecione algo para ver a pré-visualização.",
  previewUnchanged: "Ainda não foi alterado nada.",
  previewBack: "Voltar",
  clean: "Limpar a seleção",
  selected: (count) => `${String(count)} ${count === 1 ? "selecionado" : "selecionados"}`,
  lockedLine: (count) =>
    `${String(count)} ${count === 1 ? "item requer" : "itens requerem"} Pro e não ${count === 1 ? "foi tocado" : "foram tocados"}.`,
  reportOnly: "Apenas listado. Esta aplicação nunca altera chaves de todo o computador.",
};

export const leftoversCopy: Record<Locale, Copy> = { en, it, fr, es, de, pt };
