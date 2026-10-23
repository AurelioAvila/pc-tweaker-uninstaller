import type { Locale } from "./i18n";

/**
 * Plans & pricing page. Feature claims mirror what the code enforces
 * (residue.rs clean_residue and forget.rs require a verified Pro license;
 * batch removal is gated in App.tsx); everything else in the app is free.
 * The Halloween lines reuse PC Tweaker's approved wording (pricing-copy.ts),
 * narrowed to the one plan the Uninstaller sells: annual.
 */
type Copy = {
  nav: string;
  back: string;
  title: string;
  intro: string;
  accountTitle: string;
  signedOut: string;
  signedOutBody: string;
  signIn: string;
  createAccount: string;
  checking: string;
  offline: string;
  retry: string;
  refresh: string;
  freePlan: string;
  proPlan: string;
  active: string;
  activeUntil: (date: string) => string;
  bonusUntil: (date: string) => string;
  loyaltyNote: (price: string) => string;
  unverified: string;
  promoTitle: string;
  ends: string;
  endsIn: string;
  units: [string, string, string, string];
  promoReference: string;
  preview: string;
  freeName: string;
  freeTagline: string;
  freeNote: string;
  onFree: string;
  includedInPro: string;
  current: string;
  proName: string;
  proTagline: string;
  recommended: string;
  perYear: string;
  was: string;
  now: string;
  firstYear: (price: string) => string;
  loyaltyPrice: string;
  regularNote: string;
  goPro: string;
  manage: string;
  proActiveStatus: string;
  bonusStatus: string;
  signInFirst: string;
  checkoutOpened: string;
  checkoutDone: string;
  checkoutError: string;
  portalError: string;
  freeHighlights: string[];
  proHighlights: string[];
  compareTitle: string;
  capability: string;
  included: string;
  notIncluded: string;
  /** Six free capabilities, then three Pro-only ones (order matters). */
  rows: string[];
  faqTitle: string;
  renewQ: string;
  renewA: string;
  promoTerms: string;
  cancelQ: string;
  cancelA: string;
  lifetimeQ: string;
  lifetimeA: string;
  loyaltyQ: string;
  loyaltyA: (price: string) => string;
  dataTitle: string;
  data: string[];
  privacy: string;
  checkoutNote: string;
};

const en: Copy = {
  nav: "Plans & pricing",
  back: "Installed apps",
  title: "Uninstall for free. Clean up with Pro.",
  intro:
    "Single uninstalls are free, and they stay free. Pro takes care of what comes after: leftovers, several programs at once, and entries whose uninstaller is gone.",
  accountTitle: "Your account",
  signedOut: "You're not signed in",
  signedOutBody:
    "You only need an account for Pro. One free account works in PC Tweaker, Uninstaller and Tweaky Driver.",
  signIn: "Sign in",
  createAccount: "Create account",
  checking: "Checking your account…",
  offline: "We couldn't verify your account right now. Try again in a moment.",
  retry: "Try again",
  refresh: "Refresh status",
  freePlan: "Free plan",
  proPlan: "Uninstaller Pro",
  active: "Active",
  activeUntil: (date) => `Active until ${date}`,
  bonusUntil: (date) => `Included with PC Tweaker Lifetime until ${date}`,
  loyaltyNote: (price) =>
    `PC Tweaker Pro is active on this account, so Uninstaller Pro costs ${price}/year.`,
  unverified: "Verify your email address before upgrading: open the link we sent to your inbox.",
  promoTitle: "Halloween offer",
  ends: "Offer ends",
  endsIn: "Ends in",
  units: ["days", "hours", "minutes", "seconds"],
  promoReference:
    "Struck-through prices are our lowest in the 30 days before the offer. Prices exclude VAT.",
  preview: "Campaign preview · not live",
  freeName: "Free",
  freeTagline: "Everything you need to remove programs safely.",
  freeNote: "Free for good. No account needed.",
  onFree: "You're on the Free plan",
  includedInPro: "Included in your plan",
  current: "Current plan",
  proName: "Pro",
  proTagline: "Clean up what uninstallers leave behind, and remove several programs in one go.",
  recommended: "Recommended",
  perYear: "/year",
  was: "Was",
  now: "now",
  firstYear: (price) => `First year, then ${price}/year`,
  loyaltyPrice: "PC Tweaker Pro loyalty price. Billed yearly.",
  regularNote: "Billed yearly. Renews until you cancel.",
  goPro: "Go Pro",
  manage: "Manage subscription",
  proActiveStatus: "Pro is active on this account",
  bonusStatus: "Included with PC Tweaker Lifetime",
  signInFirst: "Sign in or create a free account to continue.",
  checkoutOpened: "Checkout is open in your browser. When you're done, come back and refresh.",
  checkoutDone: "I've completed checkout — refresh",
  checkoutError: "Checkout could not be started. Please try again.",
  portalError: "The billing portal could not be opened. Please try again in a moment.",
  freeHighlights: [
    "Every program on this PC, Microsoft Store apps included",
    "A confidence score and removal brief before every uninstall",
    "Single uninstalls, free for good",
    "Leftover scan and Removal Ledger",
  ],
  proHighlights: [
    "Everything in Free",
    "Leftover cleanup, with files sent to the Recycle Bin",
    "Batch removal of several programs, in a safe order",
    "Removal of broken entries",
  ],
  compareTitle: "Free and Pro at a glance",
  capability: "What's included",
  included: "Included",
  notIncluded: "Not included",
  rows: [
    "Full inventory: desktop programs and Microsoft Store apps",
    "Confidence score, relationship map and removal brief",
    "Single uninstalls, with a System Restore point attempted for machine-wide programs",
    "Leftover scan after each uninstall",
    "Removal Ledger with verified freed space",
    "Export of your inventory and ledger",
    "Leftover cleanup: files go to the Recycle Bin",
    "Batch removal: several programs, in a safe order",
    "Removal of broken entries whose uninstaller is gone",
  ],
  faqTitle: "Before you choose",
  renewQ: "How does renewal work?",
  renewA: "Uninstaller Pro is billed once a year and renews automatically until you cancel.",
  promoTerms:
    "Halloween offer: the discount covers the first year only; renewals are at the regular price. If you open checkout in the last 30 minutes of the offer, you still have 30 minutes to complete it at the offer price.",
  cancelQ: "How do I cancel?",
  cancelA:
    "Choose Manage subscription on this page. It opens Stripe's billing portal in your browser, where you can cancel, update your card or download invoices. When Pro ends, everything in Free keeps working.",
  lifetimeQ: "I own PC Tweaker Lifetime. Is Pro included?",
  lifetimeA:
    "Yes: 12 months of Uninstaller Pro from your first sign-in here, with no automatic renewal. Sign in with the same account and it's applied automatically.",
  loyaltyQ: "I already pay for PC Tweaker Pro.",
  loyaltyA: (price) =>
    `Any active PC Tweaker plan unlocks the loyalty price: Uninstaller Pro costs ${price}/year on the same account. Checkout applies it for you.`,
  dataTitle: "What the app does with your data",
  data: [
    "Your program list, leftover scans, Removal Ledger and diagnostic log stay on this PC. None of it is uploaded.",
    "No analytics, no ads, no tracking SDKs and no crash-reporting service.",
    "An account is only needed for Pro. It stores your email, a hashed password, your name, date of birth and plan.",
    "Payment runs on Stripe Checkout in your browser. Card details never pass through our servers.",
    "The signed license saved on this PC holds your account ID, plan status, product name and a timestamp. Signing out deletes it.",
  ],
  privacy: "Read the privacy policy",
  checkoutNote:
    "Payment is handled by Stripe in your browser. Final price, applicable taxes and payment terms are shown at checkout.",
};

const it: Copy = {
  nav: "Piani e prezzi",
  back: "App installate",
  title: "Disinstalla gratis. Fai pulizia con Pro.",
  intro:
    "Le disinstallazioni singole sono gratuite e lo resteranno. Pro si occupa di ciò che viene dopo: residui, più programmi insieme e voci il cui programma di disinstallazione non esiste più.",
  accountTitle: "Il tuo account",
  signedOut: "Non hai effettuato l'accesso",
  signedOutBody:
    "L'account serve solo per Pro. Un unico account gratuito vale per PC Tweaker, Uninstaller e Tweaky Driver.",
  signIn: "Accedi",
  createAccount: "Crea account",
  checking: "Verifica dell'account…",
  offline: "Impossibile verificare l'account in questo momento. Riprova tra poco.",
  retry: "Riprova",
  refresh: "Aggiorna stato",
  freePlan: "Piano Free",
  proPlan: "Uninstaller Pro",
  active: "Attivo",
  activeUntil: (date) => `Attivo fino al ${date}`,
  bonusUntil: (date) => `Incluso con PC Tweaker Lifetime fino al ${date}`,
  loyaltyNote: (price) =>
    `PC Tweaker Pro è attivo su questo account: Uninstaller Pro costa ${price}/anno.`,
  unverified:
    "Verifica il tuo indirizzo email prima di passare a Pro: apri il link che ti abbiamo inviato.",
  promoTitle: "Offerta di Halloween",
  ends: "Fine dell'offerta",
  endsIn: "Termina tra",
  units: ["giorni", "ore", "minuti", "secondi"],
  promoReference:
    "I prezzi barrati sono i più bassi dei 30 giorni prima dell'offerta. Prezzi IVA esclusa.",
  preview: "Anteprima della campagna · non attiva",
  freeName: "Free",
  freeTagline: "Tutto il necessario per rimuovere i programmi in sicurezza.",
  freeNote: "Gratis per sempre. Nessun account richiesto.",
  onFree: "Stai usando il piano Free",
  includedInPro: "Incluso nel tuo piano",
  current: "Piano attuale",
  proName: "Pro",
  proTagline:
    "Ripulisci ciò che i programmi di disinstallazione lasciano indietro e rimuovi più programmi in una volta.",
  recommended: "Consigliato",
  perYear: "/anno",
  was: "Prima",
  now: "ora",
  firstYear: (price) => `Primo anno, poi ${price}/anno`,
  loyaltyPrice: "Prezzo fedeltà PC Tweaker Pro. Fatturazione annuale.",
  regularNote: "Fatturazione annuale. Si rinnova fino alla disdetta.",
  goPro: "Passa a Pro",
  manage: "Gestisci abbonamento",
  proActiveStatus: "Pro è attivo su questo account",
  bonusStatus: "Incluso con PC Tweaker Lifetime",
  signInFirst: "Accedi o crea un account gratuito per continuare.",
  checkoutOpened: "Il pagamento è aperto nel browser. Al termine torna qui e aggiorna.",
  checkoutDone: "Ho completato il pagamento — aggiorna",
  checkoutError: "Impossibile avviare il pagamento. Riprova.",
  portalError: "Impossibile aprire il portale di fatturazione. Riprova tra poco.",
  freeHighlights: [
    "Tutti i programmi del PC, comprese le app del Microsoft Store",
    "Affidabilità e riepilogo della rimozione prima di ogni disinstallazione",
    "Disinstallazioni singole, gratis per sempre",
    "Scansione dei residui e Registro rimozioni",
  ],
  proHighlights: [
    "Tutto ciò che include Free",
    "Pulizia dei residui, con i file spostati nel Cestino",
    "Rimozione in batch di più programmi, nell'ordine sicuro",
    "Rimozione delle voci non valide",
  ],
  compareTitle: "Free e Pro a confronto",
  capability: "Cosa è incluso",
  included: "Incluso",
  notIncluded: "Non incluso",
  rows: [
    "Inventario completo: programmi desktop e app del Microsoft Store",
    "Affidabilità, mappa delle relazioni e riepilogo della rimozione",
    "Disinstallazioni singole, con un punto di ripristino tentato per i programmi installati per tutti gli utenti",
    "Scansione dei residui dopo ogni disinstallazione",
    "Registro rimozioni con spazio liberato verificato",
    "Esportazione di inventario e registro",
    "Pulizia dei residui: i file vanno nel Cestino",
    "Rimozione in batch: più programmi, nell'ordine sicuro",
    "Rimozione delle voci non valide il cui programma di disinstallazione non esiste più",
  ],
  faqTitle: "Prima di scegliere",
  renewQ: "Come funziona il rinnovo?",
  renewA:
    "Uninstaller Pro si paga una volta l'anno e si rinnova automaticamente fino alla disdetta.",
  promoTerms:
    "Offerta di Halloween: lo sconto vale solo per il primo anno; i rinnovi sono al prezzo normale. Se avvii il pagamento negli ultimi 30 minuti dell'offerta, hai ancora 30 minuti per completarlo al prezzo scontato.",
  cancelQ: "Come si disdice?",
  cancelA:
    "Scegli Gestisci abbonamento in questa pagina: si apre nel browser il portale di fatturazione di Stripe, dove puoi disdire, aggiornare la carta o scaricare le fatture. Quando Pro termina, tutto ciò che include Free continua a funzionare.",
  lifetimeQ: "Ho PC Tweaker Lifetime. Pro è incluso?",
  lifetimeA:
    "Sì: 12 mesi di Uninstaller Pro dal primo accesso qui, senza rinnovo automatico. Accedi con lo stesso account e viene applicato in automatico.",
  loyaltyQ: "Pago già PC Tweaker Pro.",
  loyaltyA: (price) =>
    `Qualsiasi piano PC Tweaker attivo sblocca il prezzo fedeltà: Uninstaller Pro costa ${price}/anno sullo stesso account. Il pagamento lo applica da solo.`,
  dataTitle: "Cosa fa l'app con i tuoi dati",
  data: [
    "L'elenco dei programmi, le scansioni dei residui, il Registro rimozioni e il log diagnostico restano su questo PC. Non vengono mai caricati.",
    "Nessuna analisi statistica, nessuna pubblicità, nessun SDK di tracciamento e nessun servizio di segnalazione dei crash.",
    "L'account serve solo per Pro. Conserva email, password in forma di hash, nome, data di nascita e piano.",
    "Il pagamento avviene su Stripe Checkout nel browser. I dati della carta non passano mai dai nostri server.",
    "La licenza firmata salvata su questo PC contiene ID account, stato del piano, nome del prodotto e data. Uscendo dall'account viene eliminata.",
  ],
  privacy: "Leggi l'informativa sulla privacy",
  checkoutNote:
    "Il pagamento è gestito da Stripe nel browser. Prezzo finale, imposte applicabili e condizioni di pagamento sono mostrati al checkout.",
};

const fr: Copy = {
  nav: "Offres et tarifs",
  back: "Applications installées",
  title: "Désinstallez gratuitement. Faites place nette avec Pro.",
  intro:
    "Les désinstallations individuelles sont gratuites et le resteront. Pro s'occupe de la suite : les restes, plusieurs programmes à la fois et les entrées dont le programme de désinstallation a disparu.",
  accountTitle: "Votre compte",
  signedOut: "Vous n'êtes pas connecté",
  signedOutBody:
    "Un compte n'est nécessaire que pour Pro. Un seul compte gratuit fonctionne dans PC Tweaker, Uninstaller et Tweaky Driver.",
  signIn: "Se connecter",
  createAccount: "Créer un compte",
  checking: "Vérification du compte…",
  offline: "Impossible de vérifier votre compte pour le moment. Réessayez dans un instant.",
  retry: "Réessayer",
  refresh: "Actualiser le statut",
  freePlan: "Offre Free",
  proPlan: "Uninstaller Pro",
  active: "Actif",
  activeUntil: (date) => `Actif jusqu'au ${date}`,
  bonusUntil: (date) => `Inclus avec PC Tweaker Lifetime jusqu'au ${date}`,
  loyaltyNote: (price) =>
    `PC Tweaker Pro est actif sur ce compte : Uninstaller Pro coûte ${price}/an.`,
  unverified:
    "Confirmez votre adresse e-mail avant de passer à Pro : ouvrez le lien que nous vous avons envoyé.",
  promoTitle: "Offre d'Halloween",
  ends: "Fin de l'offre",
  endsIn: "Se termine dans",
  units: ["jours", "heures", "minutes", "secondes"],
  promoReference:
    "Les prix barrés sont nos prix les plus bas des 30 jours précédant l'offre. Prix hors TVA.",
  preview: "Aperçu de campagne · non actif",
  freeName: "Free",
  freeTagline: "Tout le nécessaire pour supprimer des programmes en toute sécurité.",
  freeNote: "Gratuit pour toujours. Aucun compte requis.",
  onFree: "Vous utilisez l'offre Free",
  includedInPro: "Inclus dans votre offre",
  current: "Offre actuelle",
  proName: "Pro",
  proTagline:
    "Nettoyez ce que les programmes de désinstallation laissent derrière eux et supprimez plusieurs programmes d'un coup.",
  recommended: "Recommandé",
  perYear: "/an",
  was: "Avant",
  now: "maintenant",
  firstYear: (price) => `Première année, puis ${price}/an`,
  loyaltyPrice: "Prix fidélité PC Tweaker Pro. Facturation annuelle.",
  regularNote: "Facturation annuelle. Renouvellement jusqu'à résiliation.",
  goPro: "Passer à Pro",
  manage: "Gérer l'abonnement",
  proActiveStatus: "Pro est actif sur ce compte",
  bonusStatus: "Inclus avec PC Tweaker Lifetime",
  signInFirst: "Connectez-vous ou créez un compte gratuit pour continuer.",
  checkoutOpened:
    "Le paiement est ouvert dans votre navigateur. Une fois terminé, revenez ici et actualisez.",
  checkoutDone: "J'ai terminé le paiement — actualiser",
  checkoutError: "Le paiement n'a pas pu être lancé. Veuillez réessayer.",
  portalError: "Le portail de facturation n'a pas pu s'ouvrir. Veuillez réessayer dans un instant.",
  freeHighlights: [
    "Tous les programmes du PC, applications du Microsoft Store comprises",
    "Indice de confiance et résumé avant chaque désinstallation",
    "Désinstallations individuelles, gratuites pour toujours",
    "Analyse des restes et Registre des suppressions",
  ],
  proHighlights: [
    "Tout ce qu'inclut Free",
    "Nettoyage des restes, fichiers envoyés à la Corbeille",
    "Suppression par lot de plusieurs programmes, dans un ordre sûr",
    "Retrait des entrées cassées",
  ],
  compareTitle: "Free et Pro en bref",
  capability: "Ce qui est inclus",
  included: "Inclus",
  notIncluded: "Non inclus",
  rows: [
    "Inventaire complet : programmes de bureau et applications du Microsoft Store",
    "Indice de confiance, carte des relations et résumé de suppression",
    "Désinstallations individuelles, avec tentative de point de restauration pour les programmes installés pour tous les utilisateurs",
    "Analyse des restes après chaque désinstallation",
    "Registre des suppressions avec espace libéré vérifié",
    "Export de l'inventaire et du registre",
    "Nettoyage des restes : les fichiers vont dans la Corbeille",
    "Suppression par lot : plusieurs programmes, dans un ordre sûr",
    "Retrait des entrées cassées dont le programme de désinstallation a disparu",
  ],
  faqTitle: "Avant de choisir",
  renewQ: "Comment fonctionne le renouvellement ?",
  renewA:
    "Uninstaller Pro est facturé une fois par an et se renouvelle automatiquement jusqu'à résiliation.",
  promoTerms:
    "Offre d'Halloween : la réduction ne porte que sur la première année ; les renouvellements se font au prix normal. Si vous ouvrez le paiement dans les 30 dernières minutes de l'offre, vous disposez encore de 30 minutes pour le finaliser au prix de l'offre.",
  cancelQ: "Comment résilier ?",
  cancelA:
    "Choisissez Gérer l'abonnement sur cette page : le portail de facturation de Stripe s'ouvre dans votre navigateur pour résilier, changer de carte ou télécharger vos factures. Quand Pro prend fin, tout ce qu'inclut Free continue de fonctionner.",
  lifetimeQ: "J'ai PC Tweaker Lifetime. Pro est-il inclus ?",
  lifetimeA:
    "Oui : 12 mois d'Uninstaller Pro dès votre première connexion ici, sans renouvellement automatique. Connectez-vous avec le même compte, c'est appliqué automatiquement.",
  loyaltyQ: "Je paie déjà PC Tweaker Pro.",
  loyaltyA: (price) =>
    `Toute offre PC Tweaker active débloque le prix fidélité : Uninstaller Pro coûte ${price}/an sur le même compte. Le paiement l'applique pour vous.`,
  dataTitle: "Ce que l'application fait de vos données",
  data: [
    "La liste de vos programmes, les analyses des restes, le Registre des suppressions et le journal de diagnostic restent sur ce PC. Rien n'est envoyé.",
    "Aucune mesure d'audience, aucune publicité, aucun SDK de suivi ni service de rapport de plantage.",
    "Un compte n'est nécessaire que pour Pro. Il conserve votre e-mail, un hachage du mot de passe, votre nom, votre date de naissance et votre offre.",
    "Le paiement passe par Stripe Checkout dans votre navigateur. Les données de carte ne transitent jamais par nos serveurs.",
    "La licence signée enregistrée sur ce PC contient l'identifiant du compte, le statut de l'offre, le nom du produit et un horodatage. La déconnexion la supprime.",
  ],
  privacy: "Lire la politique de confidentialité",
  checkoutNote:
    "Le paiement est géré par Stripe dans votre navigateur. Le prix final, les taxes et les conditions de paiement sont indiqués au paiement.",
};

const es: Copy = {
  nav: "Planes y precios",
  back: "Aplicaciones instaladas",
  title: "Desinstala gratis. Limpia a fondo con Pro.",
  intro:
    "Las desinstalaciones individuales son gratuitas y lo seguirán siendo. Pro se encarga de lo que viene después: los restos, varios programas a la vez y las entradas cuyo desinstalador ya no existe.",
  accountTitle: "Tu cuenta",
  signedOut: "No has iniciado sesión",
  signedOutBody:
    "Solo necesitas una cuenta para Pro. Una única cuenta gratuita sirve en PC Tweaker, Uninstaller y Tweaky Driver.",
  signIn: "Iniciar sesión",
  createAccount: "Crear cuenta",
  checking: "Comprobando tu cuenta…",
  offline: "No hemos podido verificar tu cuenta ahora mismo. Inténtalo de nuevo en un momento.",
  retry: "Reintentar",
  refresh: "Actualizar estado",
  freePlan: "Plan Free",
  proPlan: "Uninstaller Pro",
  active: "Activo",
  activeUntil: (date) => `Activo hasta el ${date}`,
  bonusUntil: (date) => `Incluido con PC Tweaker Lifetime hasta el ${date}`,
  loyaltyNote: (price) =>
    `PC Tweaker Pro está activo en esta cuenta: Uninstaller Pro cuesta ${price}/año.`,
  unverified:
    "Verifica tu dirección de correo antes de pasarte a Pro: abre el enlace que te hemos enviado.",
  promoTitle: "Oferta de Halloween",
  ends: "Fin de la oferta",
  endsIn: "Termina en",
  units: ["días", "horas", "minutos", "segundos"],
  promoReference:
    "Los precios tachados son los más bajos de los 30 días previos a la oferta. Precios sin IVA.",
  preview: "Vista previa de campaña · no activa",
  freeName: "Free",
  freeTagline: "Todo lo necesario para eliminar programas con seguridad.",
  freeNote: "Gratis para siempre. Sin necesidad de cuenta.",
  onFree: "Estás en el plan Free",
  includedInPro: "Incluido en tu plan",
  current: "Plan actual",
  proName: "Pro",
  proTagline:
    "Limpia lo que dejan atrás los desinstaladores y elimina varios programas de una vez.",
  recommended: "Recomendado",
  perYear: "/año",
  was: "Antes",
  now: "ahora",
  firstYear: (price) => `Primer año, luego ${price}/año`,
  loyaltyPrice: "Precio de fidelidad de PC Tweaker Pro. Facturación anual.",
  regularNote: "Facturación anual. Se renueva hasta que lo canceles.",
  goPro: "Pasarse a Pro",
  manage: "Gestionar suscripción",
  proActiveStatus: "Pro está activo en esta cuenta",
  bonusStatus: "Incluido con PC Tweaker Lifetime",
  signInFirst: "Inicia sesión o crea una cuenta gratuita para continuar.",
  checkoutOpened: "El pago está abierto en tu navegador. Cuando termines, vuelve aquí y actualiza.",
  checkoutDone: "He completado el pago — actualizar",
  checkoutError: "No se ha podido iniciar el pago. Inténtalo de nuevo.",
  portalError: "No se ha podido abrir el portal de facturación. Inténtalo de nuevo en un momento.",
  freeHighlights: [
    "Todos los programas del PC, incluidas las apps de Microsoft Store",
    "Nivel de confianza y resumen antes de cada desinstalación",
    "Desinstalaciones individuales, gratis para siempre",
    "Análisis de restos y Registro de eliminaciones",
  ],
  proHighlights: [
    "Todo lo de Free",
    "Limpieza de restos, con los archivos enviados a la Papelera",
    "Eliminación por lotes de varios programas, en un orden seguro",
    "Eliminación de entradas rotas",
  ],
  compareTitle: "Free y Pro de un vistazo",
  capability: "Qué incluye",
  included: "Incluido",
  notIncluded: "No incluido",
  rows: [
    "Inventario completo: programas de escritorio y apps de Microsoft Store",
    "Nivel de confianza, mapa de relaciones y resumen de eliminación",
    "Desinstalaciones individuales, con un punto de restauración intentado para los programas instalados para todos los usuarios",
    "Análisis de restos tras cada desinstalación",
    "Registro de eliminaciones con espacio liberado verificado",
    "Exportación del inventario y del registro",
    "Limpieza de restos: los archivos van a la Papelera",
    "Eliminación por lotes: varios programas, en un orden seguro",
    "Eliminación de entradas rotas cuyo desinstalador ya no existe",
  ],
  faqTitle: "Antes de elegir",
  renewQ: "¿Cómo funciona la renovación?",
  renewA:
    "Uninstaller Pro se factura una vez al año y se renueva automáticamente hasta que lo canceles.",
  promoTerms:
    "Oferta de Halloween: el descuento solo cubre el primer año; las renovaciones se cobran al precio habitual. Si inicias el pago en los últimos 30 minutos de la oferta, aún tienes 30 minutos para completarlo al precio de la oferta.",
  cancelQ: "¿Cómo cancelo?",
  cancelA:
    "Elige Gestionar suscripción en esta página: se abre en tu navegador el portal de facturación de Stripe, donde puedes cancelar, cambiar la tarjeta o descargar facturas. Cuando Pro termina, todo lo de Free sigue funcionando.",
  lifetimeQ: "Tengo PC Tweaker Lifetime. ¿Incluye Pro?",
  lifetimeA:
    "Sí: 12 meses de Uninstaller Pro desde tu primer inicio de sesión aquí, sin renovación automática. Inicia sesión con la misma cuenta y se aplica automáticamente.",
  loyaltyQ: "Ya pago PC Tweaker Pro.",
  loyaltyA: (price) =>
    `Cualquier plan de PC Tweaker activo desbloquea el precio de fidelidad: Uninstaller Pro cuesta ${price}/año en la misma cuenta. El pago lo aplica por ti.`,
  dataTitle: "Qué hace la app con tus datos",
  data: [
    "La lista de programas, los análisis de restos, el Registro de eliminaciones y el registro de diagnóstico se quedan en este PC. Nada se sube.",
    "Sin analítica, sin anuncios, sin SDK de seguimiento y sin servicio de informes de fallos.",
    "Solo necesitas una cuenta para Pro. Guarda tu correo, un hash de la contraseña, tu nombre, fecha de nacimiento y plan.",
    "El pago se realiza en Stripe Checkout en tu navegador. Los datos de la tarjeta nunca pasan por nuestros servidores.",
    "La licencia firmada guardada en este PC contiene el ID de cuenta, el estado del plan, el nombre del producto y una marca de tiempo. Al cerrar sesión se elimina.",
  ],
  privacy: "Leer la política de privacidad",
  checkoutNote:
    "Stripe gestiona el pago en tu navegador. El precio final, los impuestos y las condiciones de pago se muestran al pagar.",
};

const de: Copy = {
  nav: "Tarife und Preise",
  back: "Installierte Apps",
  title: "Kostenlos deinstallieren. Mit Pro gründlich aufräumen.",
  intro:
    "Einzelne Deinstallationen sind kostenlos und bleiben es. Pro übernimmt, was danach kommt: Reste, mehrere Programme auf einmal und Einträge, deren Deinstallationsprogramm fehlt.",
  accountTitle: "Ihr Konto",
  signedOut: "Sie sind nicht angemeldet",
  signedOutBody:
    "Ein Konto brauchen Sie nur für Pro. Ein kostenloses Konto gilt für PC Tweaker, Uninstaller und Tweaky Driver.",
  signIn: "Anmelden",
  createAccount: "Konto erstellen",
  checking: "Konto wird geprüft…",
  offline: "Ihr Konto konnte gerade nicht überprüft werden. Versuchen Sie es gleich noch einmal.",
  retry: "Erneut versuchen",
  refresh: "Status aktualisieren",
  freePlan: "Free-Tarif",
  proPlan: "Uninstaller Pro",
  active: "Aktiv",
  activeUntil: (date) => `Aktiv bis ${date}`,
  bonusUntil: (date) => `In PC Tweaker Lifetime enthalten bis ${date}`,
  loyaltyNote: (price) =>
    `PC Tweaker Pro ist auf diesem Konto aktiv: Uninstaller Pro kostet ${price}/Jahr.`,
  unverified:
    "Bestätigen Sie vor dem Upgrade Ihre E-Mail-Adresse: Öffnen Sie den Link, den wir Ihnen gesendet haben.",
  promoTitle: "Halloween-Angebot",
  ends: "Angebotsende",
  endsIn: "Endet in",
  units: ["Tage", "Stunden", "Minuten", "Sekunden"],
  promoReference:
    "Durchgestrichene Preise sind die niedrigsten Preise der 30 Tage vor dem Angebot. Preise ohne MwSt.",
  preview: "Kampagnenvorschau · nicht aktiv",
  freeName: "Free",
  freeTagline: "Alles, was Sie zum sicheren Entfernen von Programmen brauchen.",
  freeNote: "Dauerhaft kostenlos. Kein Konto nötig.",
  onFree: "Sie nutzen den Free-Tarif",
  includedInPro: "In Ihrem Tarif enthalten",
  current: "Aktueller Tarif",
  proName: "Pro",
  proTagline:
    "Räumen Sie auf, was Deinstallationsprogramme zurücklassen, und entfernen Sie mehrere Programme in einem Durchgang.",
  recommended: "Empfohlen",
  perYear: "/Jahr",
  was: "Vorher",
  now: "jetzt",
  firstYear: (price) => `Erstes Jahr, danach ${price}/Jahr`,
  loyaltyPrice: "Treuepreis für PC Tweaker Pro. Jährliche Abrechnung.",
  regularNote: "Jährliche Abrechnung. Verlängert sich bis zur Kündigung.",
  goPro: "Pro holen",
  manage: "Abo verwalten",
  proActiveStatus: "Pro ist auf diesem Konto aktiv",
  bonusStatus: "In PC Tweaker Lifetime enthalten",
  signInFirst: "Melden Sie sich an oder erstellen Sie ein kostenloses Konto, um fortzufahren.",
  checkoutOpened:
    "Die Zahlung ist in Ihrem Browser geöffnet. Kommen Sie danach hierher zurück und aktualisieren Sie.",
  checkoutDone: "Zahlung abgeschlossen — aktualisieren",
  checkoutError: "Die Zahlung konnte nicht gestartet werden. Bitte versuchen Sie es erneut.",
  portalError:
    "Das Abrechnungsportal konnte nicht geöffnet werden. Bitte versuchen Sie es gleich noch einmal.",
  freeHighlights: [
    "Alle Programme auf diesem PC, einschließlich Microsoft-Store-Apps",
    "Einschätzung und Übersicht vor jeder Deinstallation",
    "Einzelne Deinstallationen, dauerhaft kostenlos",
    "Suche nach Resten und Entfernungsprotokoll",
  ],
  proHighlights: [
    "Alles aus Free",
    "Bereinigung von Resten, Dateien landen im Papierkorb",
    "Stapelentfernung mehrerer Programme in sicherer Reihenfolge",
    "Entfernen defekter Einträge",
  ],
  compareTitle: "Free und Pro im Überblick",
  capability: "Was enthalten ist",
  included: "Enthalten",
  notIncluded: "Nicht enthalten",
  rows: [
    "Vollständige Übersicht: Desktopprogramme und Microsoft-Store-Apps",
    "Einschätzung, Abhängigkeiten und Übersicht vor dem Entfernen",
    "Einzelne Deinstallationen, mit versuchtem Wiederherstellungspunkt bei Programmen für alle Benutzer",
    "Suche nach Resten nach jeder Deinstallation",
    "Entfernungsprotokoll mit nachgemessenem freigegebenem Speicher",
    "Export von Übersicht und Protokoll",
    "Bereinigung von Resten: Dateien landen im Papierkorb",
    "Stapelentfernung: mehrere Programme in sicherer Reihenfolge",
    "Entfernen defekter Einträge, deren Deinstallationsprogramm fehlt",
  ],
  faqTitle: "Bevor Sie wählen",
  renewQ: "Wie funktioniert die Verlängerung?",
  renewA:
    "Uninstaller Pro wird einmal im Jahr abgerechnet und verlängert sich automatisch bis zur Kündigung.",
  promoTerms:
    "Halloween-Angebot: Der Rabatt gilt nur für das erste Jahr; Verlängerungen erfolgen zum regulären Preis. Wenn Sie den Checkout in den letzten 30 Minuten des Angebots öffnen, haben Sie noch 30 Minuten Zeit, ihn zum Angebotspreis abzuschließen.",
  cancelQ: "Wie kündige ich?",
  cancelA:
    "Wählen Sie auf dieser Seite „Abo verwalten“: Das Abrechnungsportal von Stripe öffnet sich in Ihrem Browser. Dort können Sie kündigen, die Karte ändern oder Rechnungen herunterladen. Wenn Pro endet, funktioniert alles aus Free weiter.",
  lifetimeQ: "Ich habe PC Tweaker Lifetime. Ist Pro enthalten?",
  lifetimeA:
    "Ja: 12 Monate Uninstaller Pro ab der ersten Anmeldung hier, ohne automatische Verlängerung. Melden Sie sich mit demselben Konto an, dann wird es automatisch angewendet.",
  loyaltyQ: "Ich zahle bereits für PC Tweaker Pro.",
  loyaltyA: (price) =>
    `Jeder aktive PC-Tweaker-Tarif schaltet den Treuepreis frei: Uninstaller Pro kostet ${price}/Jahr auf demselben Konto. Der Checkout berücksichtigt ihn automatisch.`,
  dataTitle: "Was die App mit Ihren Daten macht",
  data: [
    "Ihre Programmliste, die Suche nach Resten, das Entfernungsprotokoll und das Diagnoseprotokoll bleiben auf diesem PC. Nichts davon wird hochgeladen.",
    "Keine Analyse-Tools, keine Werbung, keine Tracking-SDKs und kein Absturzmeldedienst.",
    "Ein Konto ist nur für Pro nötig. Es speichert Ihre E-Mail-Adresse, einen Hash des Passworts, Ihren Namen, Ihr Geburtsdatum und Ihren Tarif.",
    "Die Zahlung läuft über Stripe Checkout in Ihrem Browser. Kartendaten gelangen nie auf unsere Server.",
    "Die signierte Lizenz auf diesem PC enthält Ihre Konto-ID, den Tarifstatus, den Produktnamen und einen Zeitstempel. Beim Abmelden wird sie gelöscht.",
  ],
  privacy: "Datenschutzerklärung lesen",
  checkoutNote:
    "Die Zahlung wird von Stripe in Ihrem Browser abgewickelt. Endpreis, Steuern und Zahlungsbedingungen werden beim Bezahlen angezeigt.",
};

const pt: Copy = {
  nav: "Planos e preços",
  back: "Aplicações instaladas",
  title: "Desinstale grátis. Faça a limpeza com o Pro.",
  intro:
    "As desinstalações individuais são gratuitas e vão continuar a sê-lo. O Pro trata do que vem depois: resíduos, vários programas de uma vez e entradas cujo desinstalador já não existe.",
  accountTitle: "A sua conta",
  signedOut: "Não tem sessão iniciada",
  signedOutBody:
    "Só precisa de conta para o Pro. Uma única conta gratuita funciona no PC Tweaker, no Uninstaller e no Tweaky Driver.",
  signIn: "Iniciar sessão",
  createAccount: "Criar conta",
  checking: "A verificar a conta…",
  offline: "Não foi possível verificar a sua conta neste momento. Tente novamente daqui a pouco.",
  retry: "Tentar novamente",
  refresh: "Atualizar estado",
  freePlan: "Plano Free",
  proPlan: "Uninstaller Pro",
  active: "Ativo",
  activeUntil: (date) => `Ativo até ${date}`,
  bonusUntil: (date) => `Incluído com o PC Tweaker Lifetime até ${date}`,
  loyaltyNote: (price) =>
    `O PC Tweaker Pro está ativo nesta conta: o Uninstaller Pro custa ${price}/ano.`,
  unverified:
    "Confirme o seu endereço de e-mail antes de passar para o Pro: abra a ligação que lhe enviámos.",
  promoTitle: "Oferta de Halloween",
  ends: "Fim da oferta",
  endsIn: "Termina em",
  units: ["dias", "horas", "minutos", "segundos"],
  promoReference:
    "Os preços riscados são os mais baixos dos 30 dias anteriores à oferta. Preços sem IVA.",
  preview: "Pré-visualização da campanha · inativa",
  freeName: "Free",
  freeTagline: "Tudo o que precisa para remover programas em segurança.",
  freeNote: "Grátis para sempre. Sem necessidade de conta.",
  onFree: "Está no plano Free",
  includedInPro: "Incluído no seu plano",
  current: "Plano atual",
  proName: "Pro",
  proTagline:
    "Limpe o que os desinstaladores deixam para trás e remova vários programas de uma só vez.",
  recommended: "Recomendado",
  perYear: "/ano",
  was: "Antes",
  now: "agora",
  firstYear: (price) => `Primeiro ano, depois ${price}/ano`,
  loyaltyPrice: "Preço de fidelização PC Tweaker Pro. Faturação anual.",
  regularNote: "Faturação anual. Renova até ao cancelamento.",
  goPro: "Passar para Pro",
  manage: "Gerir subscrição",
  proActiveStatus: "O Pro está ativo nesta conta",
  bonusStatus: "Incluído com o PC Tweaker Lifetime",
  signInFirst: "Inicie sessão ou crie uma conta gratuita para continuar.",
  checkoutOpened: "O pagamento está aberto no browser. Quando terminar, volte aqui e atualize.",
  checkoutDone: "Concluí o pagamento — atualizar",
  checkoutError: "Não foi possível iniciar o pagamento. Tente novamente.",
  portalError: "Não foi possível abrir o portal de faturação. Tente novamente daqui a pouco.",
  freeHighlights: [
    "Todos os programas deste PC, incluindo as apps da Microsoft Store",
    "Nível de confiança e resumo antes de cada desinstalação",
    "Desinstalações individuais, grátis para sempre",
    "Análise de resíduos e Registo de remoções",
  ],
  proHighlights: [
    "Tudo o que o Free inclui",
    "Limpeza de resíduos, com os ficheiros enviados para a Reciclagem",
    "Remoção em lote de vários programas, por uma ordem segura",
    "Remoção de entradas danificadas",
  ],
  compareTitle: "Free e Pro em resumo",
  capability: "O que está incluído",
  included: "Incluído",
  notIncluded: "Não incluído",
  rows: [
    "Inventário completo: programas de ambiente de trabalho e apps da Microsoft Store",
    "Nível de confiança, mapa de relações e resumo da remoção",
    "Desinstalações individuais, com tentativa de ponto de restauro para programas instalados para todos os utilizadores",
    "Análise de resíduos após cada desinstalação",
    "Registo de remoções com espaço libertado verificado",
    "Exportação do inventário e do registo",
    "Limpeza de resíduos: os ficheiros vão para a Reciclagem",
    "Remoção em lote: vários programas, por uma ordem segura",
    "Remoção de entradas danificadas cujo desinstalador já não existe",
  ],
  faqTitle: "Antes de escolher",
  renewQ: "Como funciona a renovação?",
  renewA:
    "O Uninstaller Pro é faturado uma vez por ano e renova-se automaticamente até ao cancelamento.",
  promoTerms:
    "Oferta de Halloween: o desconto aplica-se apenas ao primeiro ano; as renovações são feitas ao preço normal. Se iniciar o pagamento nos últimos 30 minutos da oferta, ainda tem 30 minutos para o concluir ao preço da oferta.",
  cancelQ: "Como cancelo?",
  cancelA:
    "Escolha Gerir subscrição nesta página: abre-se no browser o portal de faturação da Stripe, onde pode cancelar, alterar o cartão ou transferir faturas. Quando o Pro termina, tudo o que o Free inclui continua a funcionar.",
  lifetimeQ: "Tenho o PC Tweaker Lifetime. O Pro está incluído?",
  lifetimeA:
    "Sim: 12 meses de Uninstaller Pro a partir do primeiro acesso aqui, sem renovação automática. Inicie sessão com a mesma conta e é aplicado automaticamente.",
  loyaltyQ: "Já pago o PC Tweaker Pro.",
  loyaltyA: (price) =>
    `Qualquer plano PC Tweaker ativo desbloqueia o preço de fidelização: o Uninstaller Pro custa ${price}/ano na mesma conta. O pagamento aplica-o automaticamente.`,
  dataTitle: "O que a app faz com os seus dados",
  data: [
    "A lista de programas, as análises de resíduos, o Registo de remoções e o registo de diagnóstico ficam neste PC. Nada é enviado.",
    "Sem análises estatísticas, sem anúncios, sem SDK de rastreio e sem serviço de relatórios de falhas.",
    "Só precisa de conta para o Pro. A conta guarda o e-mail, um hash da palavra-passe, o nome, a data de nascimento e o plano.",
    "O pagamento é feito no Stripe Checkout, no browser. Os dados do cartão nunca passam pelos nossos servidores.",
    "A licença assinada guardada neste PC contém o ID da conta, o estado do plano, o nome do produto e uma data. Terminar sessão elimina-a.",
  ],
  privacy: "Ler a política de privacidade",
  checkoutNote:
    "O pagamento é tratado pela Stripe no browser. O preço final, os impostos e as condições de pagamento são apresentados no checkout.",
};

export const plansCopy: Record<Locale, Copy> = { en, it, fr, es, de, pt };
