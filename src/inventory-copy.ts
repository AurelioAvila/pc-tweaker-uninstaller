import type { Locale } from "./i18n";

type Copy = {
  sort: string;
  ascending: string;
  descending: string;
  source: string;
  confidence: string;
  all: string;
  compact: string;
  reset: string;
  estimated: string;
  unknown: string;
  hint: string;
};
export const inventoryCopy: Record<Locale, Copy> = {
  en: {
    sort: "Sort by",
    ascending: "Ascending",
    descending: "Descending",
    source: "Source",
    confidence: "Removal guidance",
    all: "All",
    compact: "Compact rows",
    reset: "Reset view",
    estimated: "Reported size in this view",
    unknown: "Apps with unknown size",
    hint: "Sizes and dates are reported by installers and may be incomplete. Dates may reflect an update or repair. Reported size is not guaranteed recoverable space. Unknown values stay last.",
  },
  it: {
    sort: "Ordina per",
    ascending: "Crescente",
    descending: "Decrescente",
    source: "Origine",
    confidence: "Indicazioni di rimozione",
    all: "Tutti",
    compact: "Righe compatte",
    reset: "Ripristina vista",
    estimated: "Dimensione dichiarata nella vista",
    unknown: "App con dimensione sconosciuta",
    hint: "Dimensioni e date sono dichiarate dagli installer e possono essere incomplete. Le date possono indicare un aggiornamento o una riparazione. Lo spazio recuperabile non è garantito. I valori sconosciuti restano in fondo.",
  },
  fr: {
    sort: "Trier par",
    ascending: "Croissant",
    descending: "Décroissant",
    source: "Origine",
    confidence: "Conseils de suppression",
    all: "Tous",
    compact: "Lignes compactes",
    reset: "Réinitialiser la vue",
    estimated: "Taille déclarée dans cette vue",
    unknown: "Applications de taille inconnue",
    hint: "Les tailles et dates proviennent des programmes d’installation et peuvent être incomplètes. Les dates peuvent correspondre à une mise à jour ou réparation. L’espace récupérable n’est pas garanti. Les valeurs inconnues restent en dernier.",
  },
  es: {
    sort: "Ordenar por",
    ascending: "Ascendente",
    descending: "Descendente",
    source: "Origen",
    confidence: "Orientación de eliminación",
    all: "Todos",
    compact: "Filas compactas",
    reset: "Restablecer vista",
    estimated: "Tamaño declarado en esta vista",
    unknown: "Apps con tamaño desconocido",
    hint: "Los instaladores proporcionan los tamaños y fechas, que pueden estar incompletos. Las fechas pueden indicar una actualización o reparación. El espacio recuperable no está garantizado. Los valores desconocidos quedan al final.",
  },
  de: {
    sort: "Sortieren nach",
    ascending: "Aufsteigend",
    descending: "Absteigend",
    source: "Quelle",
    confidence: "Hinweise zur Entfernung",
    all: "Alle",
    compact: "Kompakte Zeilen",
    reset: "Ansicht zurücksetzen",
    estimated: "Gemeldete Größe in dieser Ansicht",
    unknown: "Apps mit unbekannter Größe",
    hint: "Größen und Datumsangaben stammen von Installationsprogrammen und können unvollständig sein. Datumsangaben können ein Update oder eine Reparatur betreffen. Freigebbarer Speicherplatz ist nicht garantiert. Unbekannte Werte stehen immer am Ende.",
  },
};
