import type { Locale } from "./i18n";

/** Free-trial wording for Plans & pricing, the account panel and the notice
 *  shown on return. German follows the app's informal "du". */
type TrialCopy = {
  start: string;
  subscribeNow: string;
  subscribe: string;
  subscribeNowNote: string;
  thenYear: string;
  thenYearPromo: string;
  used: string;
  trial: string;
  leftDays: string;
  leftHours: string;
  leftMinutes: string;
  firstCharge: string;
  noCharge: string;
  paymentFailed: string;
  paymentFailedHint: string;
  updatePayment: string;
  ended: string;
  trialUsed: string;
  pausedTitle: string;
  pausedBody: string;
  trialEndedTitle: string;
  endedTitle: string;
  endedBody: string;
  converted: string;
  seePlans: string;
  notNow: string;
};

export const trialCopy: Record<Locale, TrialCopy> = {
  en: {
    start: "Start {days}-day free trial",
    subscribeNow: "Subscribe now",
    subscribe: "Subscribe",
    subscribeNowNote: "Or subscribe now: {price} charged today, no trial.",
    thenYear: "Then {price} a year, first charge on {date}. Cancel anytime in Manage subscription.",
    thenYearPromo:
      "Then {price} for the first year, charged on {date}, and {regular} a year after that. Cancel anytime in Manage subscription.",
    used: "This account has already used its free trial. The plan starts as a paid subscription straight away.",
    trial: "Free trial",
    leftDays: "{days} d {hours} h left",
    leftHours: "{hours} h {minutes} min left",
    leftMinutes: "{minutes} min left",
    firstCharge: "First charge on {date}",
    noCharge: "Canceled: ends on {date}, no charge",
    paymentFailed: "Payment failed",
    paymentFailedHint: "Update your payment method to turn Pro back on.",
    updatePayment: "Update payment method",
    ended: "Ended",
    trialUsed: "Free trial used",
    pausedTitle: "Pro is paused",
    pausedBody:
      "The latest payment didn't go through, so cleaning leftover registry keys and startup entries, and batch removal, are locked until it does. Programs you already removed stay removed, and anything moved to the Recycle Bin can still be restored from there.",
    trialEndedTitle: "Your free trial has ended",
    endedTitle: "Your Pro plan has ended",
    endedBody:
      "Cleaning leftover registry keys and startup entries, and batch removal, are locked now. Single uninstalls, the leftover scan, cleaning leftover files and folders, and the Removal Ledger stay free, and anything moved to the Recycle Bin can still be restored from there.",
    converted: "Your free trial is now a Pro subscription. Thank you!",
    seePlans: "See plans",
    notNow: "Not now",
  },
  it: {
    start: "Inizia {days} giorni di prova gratuita",
    subscribeNow: "Abbonati subito",
    subscribe: "Abbonati",
    subscribeNowNote: "Oppure abbonati subito: {price} addebitati oggi, senza prova.",
    thenYear:
      "Poi {price} ogni anno, primo addebito il {date}. Disdici quando vuoi da Gestisci abbonamento.",
    thenYearPromo:
      "Poi {price} il primo anno, addebitati il {date}, e {regular} all'anno in seguito. Disdici quando vuoi da Gestisci abbonamento.",
    used: "Questo account ha già usato la sua prova gratuita. Il piano parte subito come abbonamento a pagamento.",
    trial: "Prova gratuita",
    leftDays: "Mancano {days} g {hours} h",
    leftHours: "Mancano {hours} h {minutes} min",
    leftMinutes: "Mancano {minutes} min",
    firstCharge: "Primo addebito il {date}",
    noCharge: "Disdetta: termina il {date}, nessun addebito",
    paymentFailed: "Pagamento non riuscito",
    paymentFailedHint: "Aggiorna il metodo di pagamento per riattivare Pro.",
    updatePayment: "Aggiorna il metodo di pagamento",
    ended: "Terminato",
    trialUsed: "Prova gratuita già usata",
    pausedTitle: "Pro è in pausa",
    pausedBody:
      "L'ultimo pagamento non è andato a buon fine, quindi la pulizia delle chiavi di registro e delle voci di avvio residue e la rimozione multipla restano bloccate finché non riesce. I programmi già rimossi restano rimossi e ciò che è finito nel Cestino si può ancora ripristinare da lì.",
    trialEndedTitle: "La prova gratuita è terminata",
    endedTitle: "Il tuo piano Pro è terminato",
    endedBody:
      "La pulizia delle chiavi di registro e delle voci di avvio residue e la rimozione multipla ora sono bloccate. Disinstallazioni singole, ricerca dei residui, pulizia di file e cartelle e Registro rimozioni restano gratuiti e ciò che è finito nel Cestino si può ancora ripristinare da lì.",
    converted: "La prova gratuita ora è un abbonamento Pro. Grazie!",
    seePlans: "Vedi i piani",
    notNow: "Non ora",
  },
  fr: {
    start: "Commencer l'essai gratuit de {days} jours",
    subscribeNow: "S'abonner maintenant",
    subscribe: "S'abonner",
    subscribeNowNote: "Ou abonnez-vous maintenant : {price} prélevés aujourd'hui, sans essai.",
    thenYear:
      "Ensuite {price} par an, premier prélèvement le {date}. Résiliable à tout moment depuis Gérer l'abonnement.",
    thenYearPromo:
      "Ensuite {price} la première année, prélevés le {date}, puis {regular} par an. Résiliable à tout moment depuis Gérer l'abonnement.",
    used: "Ce compte a déjà utilisé son essai gratuit. L'offre démarre directement en abonnement payant.",
    trial: "Essai gratuit",
    leftDays: "Encore {days} j {hours} h",
    leftHours: "Encore {hours} h {minutes} min",
    leftMinutes: "Encore {minutes} min",
    firstCharge: "Premier prélèvement le {date}",
    noCharge: "Résilié : fin le {date}, aucun prélèvement",
    paymentFailed: "Paiement refusé",
    paymentFailedHint: "Mettez à jour votre moyen de paiement pour réactiver Pro.",
    updatePayment: "Mettre à jour le moyen de paiement",
    ended: "Terminé",
    trialUsed: "Essai gratuit déjà utilisé",
    pausedTitle: "Pro est en pause",
    pausedBody:
      "Le dernier paiement n'a pas abouti : le nettoyage des clés de registre et des entrées de démarrage laissées derrière et la suppression par lot restent verrouillés jusqu'à ce qu'il réussisse. Les programmes déjà supprimés le restent, et ce qui a été placé dans la Corbeille peut toujours y être restauré.",
    trialEndedTitle: "Votre essai gratuit est terminé",
    endedTitle: "Votre offre Pro est terminée",
    endedBody:
      "Le nettoyage des clés de registre et des entrées de démarrage laissées derrière et la suppression par lot sont maintenant verrouillés. Les désinstallations individuelles, la recherche de restes, le nettoyage des fichiers et dossiers et le Registre des suppressions restent gratuits, et ce qui a été placé dans la Corbeille peut toujours y être restauré.",
    converted: "Votre essai gratuit est devenu un abonnement Pro. Merci !",
    seePlans: "Voir les offres",
    notNow: "Plus tard",
  },
  es: {
    start: "Empezar la prueba gratuita de {days} días",
    subscribeNow: "Suscribirse ahora",
    subscribe: "Suscribirse",
    subscribeNowNote: "O suscríbete ahora: {price} con cargo hoy, sin prueba.",
    thenYear:
      "Después, {price} al año, con el primer cargo el {date}. Cancela cuando quieras desde Gestionar suscripción.",
    thenYearPromo:
      "Después, {price} el primer año, con cargo el {date}, y {regular} al año a partir de entonces. Cancela cuando quieras desde Gestionar suscripción.",
    used: "Esta cuenta ya ha usado su prueba gratuita. El plan empieza directamente como suscripción de pago.",
    trial: "Prueba gratuita",
    leftDays: "Quedan {days} d {hours} h",
    leftHours: "Quedan {hours} h {minutes} min",
    leftMinutes: "Quedan {minutes} min",
    firstCharge: "Primer cargo el {date}",
    noCharge: "Cancelada: termina el {date}, sin cargo",
    paymentFailed: "Pago rechazado",
    paymentFailedHint: "Actualiza tu método de pago para volver a activar Pro.",
    updatePayment: "Actualizar método de pago",
    ended: "Terminado",
    trialUsed: "Prueba gratuita ya usada",
    pausedTitle: "Pro está en pausa",
    pausedBody:
      "El último pago no se ha completado, así que la limpieza de las claves del registro y las entradas de inicio, y la eliminación por lotes quedan bloqueadas hasta que se complete. Los programas que ya quitaste siguen quitados y lo que se movió a la Papelera aún se puede restaurar desde allí.",
    trialEndedTitle: "Tu prueba gratuita ha terminado",
    endedTitle: "Tu plan Pro ha terminado",
    endedBody:
      "La limpieza de las claves del registro y las entradas de inicio, y la eliminación por lotes ya están bloqueadas. Las desinstalaciones individuales, la búsqueda de restos, la limpieza de archivos y carpetas y el Registro de eliminaciones siguen siendo gratis, y lo que se movió a la Papelera aún se puede restaurar desde allí.",
    converted: "Tu prueba gratuita ya es una suscripción Pro. ¡Gracias!",
    seePlans: "Ver planes",
    notNow: "Ahora no",
  },
  de: {
    start: "{days} Tage kostenlos testen",
    subscribeNow: "Jetzt abonnieren",
    subscribe: "Abonnieren",
    subscribeNowNote: "Oder jetzt abonnieren: {price} werden heute abgebucht, ohne Testphase.",
    thenYear:
      "Danach {price} pro Jahr, erste Abbuchung am {date}. Jederzeit unter „Abo verwalten“ kündbar.",
    thenYearPromo:
      "Danach {price} für das erste Jahr, abgebucht am {date}, anschließend {regular} pro Jahr. Jederzeit unter „Abo verwalten“ kündbar.",
    used: "Dieses Konto hat seine kostenlose Testphase bereits genutzt. Der Tarif startet sofort als kostenpflichtiges Abonnement.",
    trial: "Kostenlose Testphase",
    leftDays: "Noch {days} T. {hours} Std.",
    leftHours: "Noch {hours} Std. {minutes} Min.",
    leftMinutes: "Noch {minutes} Min.",
    firstCharge: "Erste Abbuchung am {date}",
    noCharge: "Gekündigt: endet am {date}, keine Abbuchung",
    paymentFailed: "Zahlung fehlgeschlagen",
    paymentFailedHint: "Aktualisiere deine Zahlungsmethode, um Pro wieder freizuschalten.",
    updatePayment: "Zahlungsmethode aktualisieren",
    ended: "Beendet",
    trialUsed: "Kostenlose Testphase bereits genutzt",
    pausedTitle: "Pro ist pausiert",
    pausedBody:
      "Die letzte Zahlung ist nicht durchgegangen, daher bleiben die Bereinigung übrig gebliebener Registrierungsschlüssel und Autostart-Einträge sowie die Stapelentfernung gesperrt, bis sie gelingt. Bereits entfernte Programme bleiben entfernt, und was in den Papierkorb verschoben wurde, lässt sich dort weiterhin wiederherstellen.",
    trialEndedTitle: "Deine kostenlose Testphase ist beendet",
    endedTitle: "Dein Pro-Tarif ist beendet",
    endedBody:
      "Die Bereinigung übrig gebliebener Registrierungsschlüssel und Autostart-Einträge sowie die Stapelentfernung sind jetzt gesperrt. Einzelne Deinstallationen, die Restesuche, die Bereinigung von Dateien und Ordnern und das Entfernungsprotokoll bleiben kostenlos, und was in den Papierkorb verschoben wurde, lässt sich dort weiterhin wiederherstellen.",
    converted: "Deine kostenlose Testphase ist jetzt ein Pro-Abo. Vielen Dank!",
    seePlans: "Tarife ansehen",
    notNow: "Nicht jetzt",
  },
  pt: {
    start: "Experimentar {days} dias grátis",
    subscribeNow: "Subscrever agora",
    subscribe: "Subscrever",
    subscribeNowNote: "Ou subscreva agora: {price} cobrados hoje, sem avaliação gratuita.",
    thenYear:
      "Depois, {price} por ano, com a primeira cobrança a {date}. Pode cancelar quando quiser em Gerir subscrição.",
    thenYearPromo:
      "Depois, {price} no primeiro ano, cobrados a {date}, e {regular} por ano a seguir. Pode cancelar quando quiser em Gerir subscrição.",
    used: "Esta conta já utilizou a sua avaliação gratuita. O plano começa de imediato como subscrição paga.",
    trial: "Avaliação gratuita",
    leftDays: "Faltam {days} d {hours} h",
    leftHours: "Faltam {hours} h {minutes} min",
    leftMinutes: "Faltam {minutes} min",
    firstCharge: "Primeira cobrança a {date}",
    noCharge: "Cancelada: termina a {date}, sem cobrança",
    paymentFailed: "Pagamento recusado",
    paymentFailedHint: "Atualize o método de pagamento para reativar o Pro.",
    updatePayment: "Atualizar método de pagamento",
    ended: "Terminado",
    trialUsed: "Avaliação gratuita já utilizada",
    pausedTitle: "O Pro está em pausa",
    pausedBody:
      "O último pagamento não foi concluído, por isso a limpeza das chaves do registo e das entradas de arranque e a remoção em lote ficam bloqueadas até que seja. Os programas já removidos continuam removidos e o que foi para a Reciclagem ainda pode ser restaurado a partir de lá.",
    trialEndedTitle: "A sua avaliação gratuita terminou",
    endedTitle: "O seu plano Pro terminou",
    endedBody:
      "A limpeza das chaves do registo e das entradas de arranque e a remoção em lote estão agora bloqueadas. As desinstalações individuais, a análise de resíduos, a limpeza de ficheiros e pastas e o Registo de remoções continuam gratuitos, e o que foi para a Reciclagem ainda pode ser restaurado a partir de lá.",
    converted: "A sua avaliação gratuita passou a ser uma subscrição Pro. Obrigado!",
    seePlans: "Ver planos",
    notNow: "Agora não",
  },
};

/** "{name}" placeholders, filled in order of the object given. */
export function fill(template: string, values: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (all, key: string) =>
    key in values ? String(values[key]) : all,
  );
}
