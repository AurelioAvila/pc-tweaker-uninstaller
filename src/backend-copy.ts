import { currentLocale, type Locale } from "./i18n";

/**
 * The Rust side reports outcomes in English (they are also written to the
 * Removal Ledger and the diagnostic log, which stay English so support can
 * read them). The UI shows them through `backendText`, which swaps each known
 * message for the app's language and leaves anything else untouched.
 */
type Line = string | ((...groups: string[]) => string);
type Entry = [RegExp, Partial<Record<Locale, Line>>];

const OK = /^Uninstalled successfully\.$/;

const ENTRIES: Entry[] = [
  [
    OK,
    {
      it: "Disinstallato correttamente.",
      fr: "Désinstallation réussie.",
      es: "Desinstalado correctamente.",
      de: "Erfolgreich deinstalliert.",
      pt: "Desinstalado com êxito.",
    },
  ],
  [
    /^Uninstalled successfully\. A restart is required to finish removing files\.$/,
    {
      it: "Disinstallato correttamente. Serve un riavvio per completare la rimozione dei file.",
      fr: "Désinstallation réussie. Un redémarrage est nécessaire pour finir de supprimer les fichiers.",
      es: "Desinstalado correctamente. Se requiere un reinicio para terminar de eliminar los archivos.",
      de: "Erfolgreich deinstalliert. Ein Neustart ist nötig, um die Dateien vollständig zu entfernen.",
      pt: "Desinstalado com êxito. É necessário reiniciar para terminar a remoção dos ficheiros.",
    },
  ],
  [
    /^The uninstall was cancelled by the user\.$/,
    {
      it: "La disinstallazione è stata annullata dall'utente.",
      fr: "La désinstallation a été annulée par l'utilisateur.",
      es: "El usuario canceló la desinstalación.",
      de: "Die Deinstallation wurde vom Benutzer abgebrochen.",
      pt: "A desinstalação foi cancelada pelo utilizador.",
    },
  ],
  [
    /^Windows Installer reports this product is not installed .{1,3} it may already be removed\. Refresh the list to check\.$/,
    {
      it: "Windows Installer segnala che questo prodotto non è installato: potrebbe essere già stato rimosso. Aggiorna l'elenco per verificare.",
      fr: "Windows Installer indique que ce produit n'est pas installé : il a peut-être déjà été supprimé. Actualisez la liste pour vérifier.",
      es: "Windows Installer indica que este producto no está instalado: puede que ya se haya eliminado. Actualiza la lista para comprobarlo.",
      de: "Windows Installer meldet, dass dieses Produkt nicht installiert ist – es wurde möglicherweise bereits entfernt. Aktualisieren Sie die Liste, um das zu prüfen.",
      pt: "O Windows Installer indica que este produto não está instalado: pode já ter sido removido. Atualize a lista para confirmar.",
    },
  ],
  [
    /^The uninstaller exited with code (-?\d+)\. The program may not be fully removed .{1,3} refresh the list to check\.$/,
    {
      it: (c) =>
        `Il programma di disinstallazione è terminato con codice ${c}. Il programma potrebbe non essere stato rimosso del tutto: aggiorna l'elenco per verificare.`,
      fr: (c) =>
        `Le désinstalleur s'est terminé avec le code ${c}. Le programme n'est peut-être pas entièrement supprimé : actualisez la liste pour vérifier.`,
      es: (c) =>
        `El desinstalador terminó con el código ${c}. Puede que el programa no se haya eliminado por completo: actualiza la lista para comprobarlo.`,
      de: (c) =>
        `Das Deinstallationsprogramm wurde mit dem Code ${c} beendet. Das Programm ist möglicherweise nicht vollständig entfernt – aktualisieren Sie die Liste, um das zu prüfen.`,
      pt: (c) =>
        `O desinstalador terminou com o código ${c}. O programa pode não ter sido totalmente removido: atualize a lista para confirmar.`,
    },
  ],
  [
    /^The uninstaller was terminated before reporting a result\.$/,
    {
      it: "Il programma di disinstallazione è stato interrotto prima di comunicare un esito.",
      fr: "Le désinstalleur a été arrêté avant d'avoir signalé un résultat.",
      es: "El desinstalador se cerró antes de informar de un resultado.",
      de: "Das Deinstallationsprogramm wurde beendet, bevor es ein Ergebnis gemeldet hat.",
      pt: "O desinstalador foi terminado antes de comunicar um resultado.",
    },
  ],
  [
    /^Per-user uninstall: no restore point will be created \(that requires administrator rights\)\.$/,
    {
      it: "Disinstallazione per utente: non verrà creato alcun punto di ripristino (serve l'accesso da amministratore).",
      fr: "Désinstallation par utilisateur : aucun point de restauration ne sera créé (cela exige les droits d'administrateur).",
      es: "Desinstalación por usuario: no se creará ningún punto de restauración (requiere derechos de administrador).",
      de: "Deinstallation pro Benutzer: Es wird kein Wiederherstellungspunkt erstellt (dafür sind Administratorrechte nötig).",
      pt: "Desinstalação por utilizador: não será criado nenhum ponto de restauro (isso requer direitos de administrador).",
    },
  ],
  [
    /^This uninstaller is the program's own\. It may show its own windows and ask its own questions\.$/,
    {
      it: "Questo è il programma di disinstallazione del programma stesso. Può mostrare finestre proprie e fare domande proprie.",
      fr: "Il s'agit du désinstalleur propre au programme. Il peut afficher ses propres fenêtres et poser ses propres questions.",
      es: "Este es el desinstalador del propio programa. Puede mostrar sus propias ventanas y hacer sus propias preguntas.",
      de: "Dies ist das Deinstallationsprogramm des Programms selbst. Es kann eigene Fenster anzeigen und eigene Fragen stellen.",
      pt: "Este é o desinstalador do próprio programa. Pode mostrar as suas próprias janelas e fazer as suas próprias perguntas.",
    },
  ],
  [
    /^This program was installed for your account only, and its uninstaller asks for administrator rights from a folder your account can change\. It was not started with elevation\. If you trust it, run its uninstaller from Windows Settings\.$/,
    {
      it: "Questo programma è stato installato solo per il tuo account e il suo programma di disinstallazione chiede i diritti di amministratore da una cartella che il tuo account può modificare. Non è stato avviato con privilegi elevati. Se ti fidi, eseguilo dalle Impostazioni di Windows.",
      fr: "Ce programme a été installé uniquement pour votre compte, et son désinstalleur demande les droits d'administrateur depuis un dossier que votre compte peut modifier. Il n'a pas été lancé avec élévation. Si vous lui faites confiance, lancez son désinstalleur depuis les Paramètres Windows.",
      es: "Este programa se instaló solo para tu cuenta y su desinstalador solicita derechos de administrador desde una carpeta que tu cuenta puede modificar. No se inició con privilegios elevados. Si confías en él, ejecuta su desinstalador desde Configuración de Windows.",
      de: "Dieses Programm wurde nur für Ihr Konto installiert, und sein Deinstallationsprogramm fordert Administratorrechte aus einem Ordner an, den Ihr Konto ändern kann. Es wurde nicht mit erhöhten Rechten gestartet. Wenn Sie ihm vertrauen, führen Sie sein Deinstallationsprogramm über die Windows-Einstellungen aus.",
      pt: "Este programa foi instalado apenas para a sua conta e o seu desinstalador pede direitos de administrador a partir de uma pasta que a sua conta pode alterar. Não foi iniciado com privilégios elevados. Se confiar nele, execute o respetivo desinstalador a partir das Definições do Windows.",
    },
  ],
  [
    /^This program's uninstall command is not safe to run automatically\. Use the program's own uninstaller instead\.$/,
    {
      it: "Il comando di disinstallazione di questo programma non è sicuro da eseguire in automatico. Usa invece il programma di disinstallazione del programma stesso.",
      fr: "La commande de désinstallation de ce programme n'est pas sûre à exécuter automatiquement. Utilisez plutôt le désinstalleur propre au programme.",
      es: "El comando de desinstalación de este programa no es seguro para ejecutarlo automáticamente. Usa en su lugar el desinstalador del propio programa.",
      de: "Der Deinstallationsbefehl dieses Programms ist nicht sicher für die automatische Ausführung. Verwenden Sie stattdessen das Deinstallationsprogramm des Programms selbst.",
      pt: "O comando de desinstalação deste programa não é seguro para execução automática. Utilize antes o desinstalador do próprio programa.",
    },
  ],
  [
    /^This program does not declare an uninstall command\.$/,
    {
      it: "Questo programma non dichiara alcun comando di disinstallazione.",
      fr: "Ce programme ne déclare aucune commande de désinstallation.",
      es: "Este programa no declara ningún comando de desinstalación.",
      de: "Dieses Programm deklariert keinen Deinstallationsbefehl.",
      pt: "Este programa não declara nenhum comando de desinstalação.",
    },
  ],
  [
    /^The uninstaller executable no longer exists on disk\.$/,
    {
      it: "Il file eseguibile del programma di disinstallazione non esiste più sul disco.",
      fr: "L'exécutable du désinstalleur n'existe plus sur le disque.",
      es: "El ejecutable del desinstalador ya no existe en el disco.",
      de: "Die ausführbare Datei des Deinstallationsprogramms existiert nicht mehr auf dem Datenträger.",
      pt: "O executável do desinstalador já não existe no disco.",
    },
  ],
  [
    /^This program's uninstall command must be run manually\.$/,
    {
      it: "Il comando di disinstallazione di questo programma va eseguito manualmente.",
      fr: "La commande de désinstallation de ce programme doit être lancée manuellement.",
      es: "El comando de desinstalación de este programa debe ejecutarse manualmente.",
      de: "Der Deinstallationsbefehl dieses Programms muss manuell ausgeführt werden.",
      pt: "O comando de desinstalação deste programa tem de ser executado manualmente.",
    },
  ],
  [
    /^Another uninstall is already running\. Wait for it to finish\.$/,
    {
      it: "È già in corso un'altra disinstallazione. Attendi che finisca.",
      fr: "Une autre désinstallation est déjà en cours. Attendez qu'elle se termine.",
      es: "Ya hay otra desinstalación en curso. Espera a que termine.",
      de: "Es läuft bereits eine andere Deinstallation. Warten Sie, bis sie beendet ist.",
      pt: "Já está a decorrer outra desinstalação. Aguarde que termine.",
    },
  ],
  [
    /^The elevated uninstall finished but left no report\.$/,
    {
      it: "La disinstallazione con privilegi elevati è terminata ma non ha lasciato alcun rapporto.",
      fr: "La désinstallation avec élévation s'est terminée sans laisser de rapport.",
      es: "La desinstalación con privilegios elevados terminó sin dejar ningún informe.",
      de: "Die Deinstallation mit erhöhten Rechten wurde beendet, hat aber keinen Bericht hinterlassen.",
      pt: "A desinstalação com privilégios elevados terminou sem deixar relatório.",
    },
  ],
  [
    /^The elevated uninstall report could not be read\.$/,
    {
      it: "Impossibile leggere il rapporto della disinstallazione con privilegi elevati.",
      fr: "Le rapport de la désinstallation avec élévation n'a pas pu être lu.",
      es: "No se pudo leer el informe de la desinstalación con privilegios elevados.",
      de: "Der Bericht der Deinstallation mit erhöhten Rechten konnte nicht gelesen werden.",
      pt: "Não foi possível ler o relatório da desinstalação com privilégios elevados.",
    },
  ],
  [
    /^Windows refused to start the uninstaller even with elevation\.$/,
    {
      it: "Windows ha rifiutato di avviare il programma di disinstallazione anche con privilegi elevati.",
      fr: "Windows a refusé de lancer le désinstalleur, même avec élévation.",
      es: "Windows se negó a iniciar el desinstalador incluso con privilegios elevados.",
      de: "Windows hat das Starten des Deinstallationsprogramms auch mit erhöhten Rechten verweigert.",
      pt: "O Windows recusou-se a iniciar o desinstalador, mesmo com privilégios elevados.",
    },
  ],
  [
    /^The uninstall was refused before it started\.$/,
    {
      it: "La disinstallazione è stata rifiutata prima dell'avvio.",
      fr: "La désinstallation a été refusée avant son démarrage.",
      es: "La desinstalación se rechazó antes de empezar.",
      de: "Die Deinstallation wurde vor dem Start abgelehnt.",
      pt: "A desinstalação foi recusada antes de começar.",
    },
  ],
  [
    /^The uninstaller could not be started: ([\s\S]*)$/,
    {
      it: (e) => `Impossibile avviare il programma di disinstallazione: ${e}`,
      fr: (e) => `Le désinstalleur n'a pas pu être lancé : ${e}`,
      es: (e) => `No se pudo iniciar el desinstalador: ${e}`,
      de: (e) => `Das Deinstallationsprogramm konnte nicht gestartet werden: ${e}`,
      pt: (e) => `Não foi possível iniciar o desinstalador: ${e}`,
    },
  ],
  [
    /^The uninstall did not run: ([\s\S]*?)\. No changes were made by this app\.$/,
    {
      it: (e) => `La disinstallazione non è stata eseguita: ${e}. L'app non ha modificato nulla.`,
      fr: (e) => `La désinstallation n'a pas eu lieu : ${e}. L'application n'a rien modifié.`,
      es: (e) => `La desinstalación no se ejecutó: ${e}. La aplicación no hizo ningún cambio.`,
      de: (e) => `Die Deinstallation wurde nicht ausgeführt: ${e}. Die App hat nichts verändert.`,
      pt: (e) => `A desinstalação não foi executada: ${e}. A aplicação não fez nenhuma alteração.`,
    },
  ],
  [
    /^The broken entry was removed from Installed programs\. Its leftover files can be scanned next\.$/,
    {
      it: "La voce danneggiata è stata rimossa da Programmi installati. Ora puoi cercarne i file residui.",
      fr: "L'entrée endommagée a été retirée des programmes installés. Vous pouvez maintenant rechercher les fichiers qu'elle a laissés.",
      es: "La entrada dañada se quitó de Programas instalados. Ahora puedes buscar los archivos que dejó.",
      de: "Der defekte Eintrag wurde aus den installierten Programmen entfernt. Als Nächstes können Sie nach den übrig gebliebenen Dateien suchen.",
      pt: "A entrada danificada foi removida dos Programas instalados. Agora pode procurar os ficheiros que deixou para trás.",
    },
  ],
  [
    /^Windows marks this entry as a system component or an update, so it is not removed\.$/,
    {
      it: "Windows contrassegna questa voce come componente di sistema o aggiornamento, quindi non viene rimossa.",
      fr: "Windows signale cette entrée comme composant système ou mise à jour : elle n'est donc pas retirée.",
      es: "Windows marca esta entrada como componente del sistema o actualización, así que no se quita.",
      de: "Windows kennzeichnet diesen Eintrag als Systemkomponente oder Update, daher wird er nicht entfernt.",
      pt: "O Windows marca esta entrada como componente de sistema ou atualização, por isso não é removida.",
    },
  ],
  [
    /^This program's uninstaller is still present\. Uninstall it normally instead\.$/,
    {
      it: "Il programma di disinstallazione di questo programma è ancora presente. Disinstallalo normalmente.",
      fr: "Le désinstalleur de ce programme est toujours présent. Désinstallez-le normalement.",
      es: "El desinstalador de este programa sigue presente. Desinstálalo con normalidad.",
      de: "Das Deinstallationsprogramm dieses Programms ist noch vorhanden. Deinstallieren Sie es stattdessen normal.",
      pt: "O desinstalador deste programa ainda existe. Desinstale-o normalmente.",
    },
  ],
  [
    /^An active Uninstaller Pro license is required to remove broken entries\.$/,
    {
      it: "Per rimuovere le voci danneggiate serve una licenza Uninstaller Pro attiva.",
      fr: "Une licence Uninstaller Pro active est requise pour retirer les entrées endommagées.",
      es: "Se necesita una licencia activa de Uninstaller Pro para quitar entradas dañadas.",
      de: "Zum Entfernen defekter Einträge ist eine aktive Uninstaller-Pro-Lizenz erforderlich.",
      pt: "É necessária uma licença Uninstaller Pro ativa para remover entradas danificadas.",
    },
  ],
  [
    /^An active Uninstaller Pro license is required for cleanup\.$/,
    {
      it: "Per la pulizia serve una licenza Uninstaller Pro attiva.",
      fr: "Une licence Uninstaller Pro active est requise pour le nettoyage.",
      es: "Se necesita una licencia activa de Uninstaller Pro para la limpieza.",
      de: "Für die Bereinigung ist eine aktive Uninstaller-Pro-Lizenz erforderlich.",
      pt: "É necessária uma licença Uninstaller Pro ativa para a limpeza.",
    },
  ],
  [
    /^That package is not installed\.$/,
    {
      it: "Quel pacchetto non è installato.",
      fr: "Ce paquet n'est pas installé.",
      es: "Ese paquete no está instalado.",
      de: "Dieses Paket ist nicht installiert.",
      pt: "Esse pacote não está instalado.",
    },
  ],
  [
    /^This package is part of Windows and stays installed\.$/,
    {
      it: "Questo pacchetto fa parte di Windows e resta installato.",
      fr: "Ce paquet fait partie de Windows et reste installé.",
      es: "Este paquete forma parte de Windows y permanece instalado.",
      de: "Dieses Paket gehört zu Windows und bleibt installiert.",
      pt: "Este pacote faz parte do Windows e permanece instalado.",
    },
  ],
  [
    /^Windows refused the removal: ([\s\S]*)$/,
    {
      it: (e) => `Windows ha rifiutato la rimozione: ${e}`,
      fr: (e) => `Windows a refusé la suppression : ${e}`,
      es: (e) => `Windows rechazó la eliminación: ${e}`,
      de: (e) => `Windows hat die Entfernung verweigert: ${e}`,
      pt: (e) => `O Windows recusou a remoção: ${e}`,
    },
  ],
  [
    /^Restore points require administrator rights; this per-user (?:uninstall runs|entry is removed) without one\.$/,
    {
      it: "I punti di ripristino richiedono i diritti di amministratore: questa operazione per utente prosegue senza.",
      fr: "Les points de restauration exigent les droits d'administrateur : cette opération par utilisateur se déroule sans point de restauration.",
      es: "Los puntos de restauración requieren derechos de administrador: esta operación por usuario se realiza sin ninguno.",
      de: "Wiederherstellungspunkte erfordern Administratorrechte: Dieser Vorgang pro Benutzer läuft ohne einen solchen.",
      pt: "Os pontos de restauro requerem direitos de administrador: esta operação por utilizador decorre sem nenhum.",
    },
  ],
  [
    /^Windows reported success, but lists no restore point\.$/,
    {
      it: "Windows ha segnalato l'esito positivo, ma non elenca alcun punto di ripristino.",
      fr: "Windows a signalé une réussite, mais n'affiche aucun point de restauration.",
      es: "Windows informó de que todo fue bien, pero no muestra ningún punto de restauración.",
      de: "Windows meldete Erfolg, listet aber keinen Wiederherstellungspunkt auf.",
      pt: "O Windows indicou êxito, mas não apresenta nenhum ponto de restauro.",
    },
  ],
  [
    /^System Restore is not available on this Windows edition\.$/,
    {
      it: "Il Ripristino configurazione di sistema non è disponibile in questa edizione di Windows.",
      fr: "La restauration du système n'est pas disponible dans cette édition de Windows.",
      es: "Restaurar sistema no está disponible en esta edición de Windows.",
      de: "Die Systemwiederherstellung ist in dieser Windows-Edition nicht verfügbar.",
      pt: "O Restauro do Sistema não está disponível nesta edição do Windows.",
    },
  ],
  [
    /^System Restore is disabled on this system\.$/,
    {
      it: "Il Ripristino configurazione di sistema è disattivato su questo sistema.",
      fr: "La restauration du système est désactivée sur ce système.",
      es: "Restaurar sistema está desactivado en este sistema.",
      de: "Die Systemwiederherstellung ist auf diesem System deaktiviert.",
      pt: "O Restauro do Sistema está desativado neste sistema.",
    },
  ],
  [
    /^Windows refused to create a restore point \(status (\d+)\)\.$/,
    {
      it: (c) => `Windows ha rifiutato di creare un punto di ripristino (stato ${c}).`,
      fr: (c) => `Windows a refusé de créer un point de restauration (état ${c}).`,
      es: (c) => `Windows se negó a crear un punto de restauración (estado ${c}).`,
      de: (c) =>
        `Windows hat das Erstellen eines Wiederherstellungspunkts verweigert (Status ${c}).`,
      pt: (c) => `O Windows recusou-se a criar um ponto de restauro (estado ${c}).`,
    },
  ],
];

/** The message in the app's language; unknown messages come back unchanged. */
export function backendText(message: string): string {
  const locale = currentLocale();
  if (locale === "en") return message;
  for (const [pattern, lines] of ENTRIES) {
    const match = pattern.exec(message);
    const line = lines[locale];
    if (!match || line === undefined) continue;
    return typeof line === "string" ? line : line(...match.slice(1));
  }
  return message;
}
