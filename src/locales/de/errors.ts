import type { errors as en } from '../en/errors'
import type { LocaleShape } from '../types'

export const errors: LocaleShape<typeof en> = {
  titles: {
    busy: 'RemMods ist beschäftigt',
    io: 'Dateivorgang fehlgeschlagen',
    invalidInput: 'Ungültiger Wert',
    readOnly: 'Einstellungen sind schreibgeschützt',
    internal: 'Unerwarteter Fehler',
    unknown: 'Etwas ist schiefgelaufen',
    stale: 'Liste veraltet',
    gameNotSet: 'Kein Spielordner',
    invalidGameFolder: 'Kein TEKKEN-8-Ordner',
    launchFailed: 'TEKKEN 8 konnte nicht gestartet werden',
    revealFailed: 'Dateimanager konnte nicht geöffnet werden',
    gameRunning: 'TEKKEN 8 läuft',
    targetExists: 'Name bereits vergeben',
    outOfScope: 'Außerhalb der Mod-Ordner',
    toggleBlocked: 'Diese Mod kann nicht geändert werden',
    batchFailed: 'Änderungen nicht angewendet',
    invalidName: 'Name nicht erlaubt',
    manageBlocked: 'Das geht nicht',
    trashUnavailable: 'Papierkorb nicht nutzbar',
    deleteTokenInvalid: 'Bestätigung abgelaufen',
    modinfoAmbiguous: 'modinfo.json ist nicht eindeutig',
    profileNotFound: 'Profil nicht gefunden',
    profileNameTaken: 'Profilname schon vergeben',
    importFailed: 'Import fehlgeschlagen',
    archiveFailed: 'Dieses Archiv lässt sich nicht installieren',
    installBlocked: 'Installation nicht möglich',
    installBusy: 'Eine andere Installation läuft',
    installCancelled: 'Installation abgebrochen',
    noSpace: 'Nicht genug freier Speicher',
    sourceRejected: 'Das lässt sich nicht installieren',
  },
  messages: {
    busy: 'Eine andere Änderung läuft noch. Versuch es gleich nochmal.',
    invalidInput: 'Der Wert für „{{field}}“ ist ungültig.',
    readOnly:
      'RemMods kann settings.json nicht sicher schreiben, deshalb werden Änderungen nicht gespeichert. Der Grund steht im Log.',
    internal: 'Ein unerwarteter Fehler ist aufgetreten. Details stehen im Log.',
    unknown: 'Ein unerwarteter Fehler ist aufgetreten. Details stehen im Log.',
    stale:
      'Die Dateien haben sich geändert. RemMods hat die Liste aktualisiert; versuch es noch einmal.',
    gameNotSet: 'Wähle zuerst den TEKKEN-8-Ordner.',
    launchFailed:
      'RemMods konnte steam://rungameid/1778820 nicht an Steam übergeben. Starte das Spiel über Steam; Details stehen im Log.',
    gameRunning: 'TEKKEN 8 läuft.',
    targetExists: 'Eine Datei mit dem neuen Namen existiert bereits. Nichts wurde geändert.',
    outOfScope: 'RemMods ändert nur Dateien in den Mod-Ordnern.',
    batchFailed:
      'Eine Datei konnte nicht umbenannt werden ({{reason}}). Alle Änderungen wurden rückgängig gemacht.',
    batchFolderInUse:
      'Ein Ordner wird von einem anderen Programm verwendet. Schließe es oder TEKKEN 8 und versuch es nochmal.',
    batchStuck_one:
      '{{count}} Umbenennung konnte nicht rückgängig gemacht werden. RemMods holt das beim nächsten Start nach.',
    batchStuck_other:
      '{{count}} Umbenennungen konnten nicht rückgängig gemacht werden. RemMods holt das beim nächsten Start nach.',
    probeInUse: 'Eine Mod-Datei wird verwendet. Schließe zuerst TEKKEN 8.',
    trashUnavailable:
      'Das Verschieben in den Papierkorb ist hier nicht möglich. Nichts wurde geändert.',
    deleteTokenInvalid:
      'Die Löschbestätigung ist abgelaufen. Nichts wurde gelöscht; starte noch einmal.',
    modinfoAmbiguous:
      'Mehrere modinfo.json-Dateien unterscheiden sich nur in der Schreibweise. Entferne zuerst die überzähligen.',
    profileNotFound: 'Dieses Profil gibt es nicht mehr. Die Liste wurde aktualisiert.',
    profileNameTaken: 'Ein Profil mit diesem Namen gibt es schon.',
    installBusy: 'Warte, bis die laufende Installation fertig ist, und versuche es dann erneut.',
    installCancelled: 'Es wurde nichts installiert.',
    noSpace: 'Die Installation braucht {{needed}}, aber nur {{available}} sind frei.',
  },
  archiveFailed: {
    notAnArchive: 'Diese Datei ist kein zip-, 7z- oder rar-Archiv oder sie ist beschädigt.',
    multiVolume: 'Dieses Archiv ist auf mehrere Teile verteilt.',
    selfExtracting: 'Das ist ein selbstentpackendes Programm, das RemMods nie ausführt.',
    unsupportedMethod: 'Das Archiv nutzt ein Kompressionsverfahren, das RemMods nicht lesen kann.',
    rarDisabled: 'Diese RemMods-Version kann keine rar-Archive lesen.',
    encrypted: 'Das Archiv ist passwortgeschützt.',
    corrupt: 'Das Archiv ist beschädigt oder stimmt nicht mit seinen Angaben überein.',
    memoryLimit: 'Zum Entpacken dieses Archivs wäre mehr Arbeitsspeicher nötig als erlaubt.',
    tooManyEntries: 'Das Archiv enthält zu viele Dateien.',
    tooLarge: 'Das Archiv wird größer entpackt, als RemMods erlaubt.',
  },
  installBlocked: {
    issues: 'Die Quelle hat Probleme, die die Installation verhindern.',
    invalidName: 'Ein Name ist nicht erlaubt oder kollidiert mit einem anderen.',
    unresolvedClash: 'Bei einem Groß-/Kleinschreibungs-Konflikt fehlt noch eine Wahl.',
    noSelection: 'Es ist nichts ausgewählt.',
    variantConflict: 'Von mehreren Alternativen lässt sich nur eine installieren.',
    replaceNotConfirmed: 'Das Ersetzen einer installierten Mod wurde nicht bestätigt.',
    replaceMismatch: 'Die Ersetzen-Auswahl passt nicht zu den installierten Mods.',
    ambiguous:
      'Mehrere installierte Mods passen zum Namen. Kläre das zuerst im Reiter „Reparieren“.',
    targetChanged: 'Die Mod-Ordner haben sich während der Vorbereitung geändert.',
  },
  sourceRejected: {
    unsupportedType: 'Nur zip-, 7z- und rar-Archive sowie Ordner lassen sich installieren.',
    missing: 'RemMods kommt nicht heran. Kopiere es zuerst in einen lokalen Ordner.',
    link: 'Verknüpfungen werden nicht verfolgt. Wähle die echte Datei oder den echten Ordner.',
    insidePaks: 'Dieser Ordner liegt im Paks-Ordner des Spiels.',
    tooMany: 'Zu viele Elemente auf einmal.',
  },
  manageBlocked: {
    action: {
      rename: 'RemMods benennt diesen Eintrag nicht um.',
      trash: 'RemMods verschiebt diesen Eintrag nicht in den Papierkorb.',
      delete: 'RemMods löscht diesen Eintrag nicht.',
      metadata: 'RemMods bearbeitet die Infos dieses Eintrags nicht.',
    },
    reason: {
      loose: 'Lose Dateien haben keinen Ordner, der bearbeitet werden könnte.',
      linked: 'Verknüpfte Mod-Ordner ändert RemMods nicht.',
      archive: 'Archive bleiben unberührt.',
      leftover: 'Übrig gebliebene Ordner lassen sich nicht umbenennen oder bearbeiten.',
      unmanaged: 'Dieser Eintrag wird nicht von RemMods verwaltet.',
      outsideRoots:
        'Der Paks-Ordner selbst ist schreibgeschützt; nur die Mod-Ordner werden geändert.',
      journalPending:
        'Eine unfertige Änderung bezieht sich noch auf diesen Ordner. Starte RemMods zuerst neu.',
    },
  },
  trashUnavailable: {
    otherDevice: 'Dieser Ordner liegt auf einem anderen Laufwerk als dein Papierkorb.',
    noUsableTrash: 'Für dieses Laufwerk wurde kein nutzbarer Papierkorb gefunden.',
    notSupported: 'Das Verschieben in den Papierkorb ist auf diesem System noch nicht möglich.',
  },
  importFailed: {
    tooLarge: 'Die Datei ist zu groß für eine Profilliste.',
    notJson: 'Die Datei ist kein gültiges JSON.',
    wrongShape: 'Die Datei enthält keine Profilliste.',
    wrongFormat: 'Die Datei ist kein RemMods-Profilexport.',
    newerVersion: 'Die Datei stammt von einem neueren RemMods. Aktualisiere zuerst RemMods.',
  },
  toggleBlocked: {
    ambiguous: 'Mehrere Mods haben diesen Namen. Benenne zuerst eine um.',
    linked: 'Verknüpfte Mod-Ordner ändert RemMods nicht.',
    unknownState: 'Die Dateien der Mod konnten nicht gelesen werden.',
    noPakFiles: 'Diese Mod enthält keine Pak-Dateien.',
    invalidLogicName: 'Dieser LogicMods-Ordner hat keinen Namen.',
  },
  dialogs: {
    ok: 'OK',
    running: {
      title: 'TEKKEN 8 läuft',
      body: 'Änderungen gelten ab dem nächsten Spielstart. Bereits geladene Mods bleiben bis dahin geladen.',
      confirm: 'Für nächsten Start anwenden',
      cancel: 'Abbrechen',
    },
    busy: {
      title: 'Ein anderes RemMods ändert Dateien',
      body: 'Warte, bis es fertig ist, und versuch es dann nochmal.',
      retry: 'Nochmal versuchen',
      cancel: 'Abbrechen',
    },
    stale: {
      title: 'Die Modliste hat sich auf dem Datenträger geändert',
      body: 'RemMods hat die Liste neu geladen. Prüfe die Mods und versuch es nochmal.',
    },
  },
  ioClasses: {
    inUse: 'Die Datei wird von einem anderen Programm verwendet.',
    accessDenied: 'Der Zugriff wurde verweigert.',
    noSpace: 'Auf dem Laufwerk ist nicht genug Platz frei.',
    readOnlyFs: 'Das Laufwerk ist schreibgeschützt.',
    crossDevice: 'Die Datei müsste auf ein anderes Laufwerk verschoben werden.',
    nameTooLong: 'Der Name oder Pfad ist zu lang.',
    notFound: 'Die Datei oder der Ordner wurde nicht gefunden.',
    unsupported: 'Das wird auf diesem System nicht unterstützt.',
    other: 'Das Betriebssystem hat einen Fehler gemeldet. Details stehen im Log.',
    alreadyExists: 'Eine Datei mit diesem Namen existiert bereits.',
  },
  ioClassNames: {
    inUse: 'in Benutzung',
    accessDenied: 'Zugriff verweigert',
    noSpace: 'kein Speicherplatz',
    readOnlyFs: 'schreibgeschütztes Laufwerk',
    crossDevice: 'anderes Laufwerk',
    nameTooLong: 'Name zu lang',
    notFound: 'nicht gefunden',
    unsupported: 'nicht unterstützt',
    other: 'sonstiger Fehler',
    alreadyExists: 'Name existiert',
  },
  folderProblems: {
    notFound: 'Dort wurden keine Spieldateien von TEKKEN 8 gefunden.',
    ambiguous:
      'Dort unterscheiden sich mehrere Ordner nur in Groß- und Kleinschreibung, zum Beispiel Polaris und polaris.',
    notUnicode: 'Der Pfad enthält Zeichen, die RemMods nicht speichern kann.',
    unreadable: 'Der Ordner konnte nicht gelesen werden.',
  },
  revealProblems: {
    unsupportedName:
      'Dieser Name ist unter Windows ungültig, deshalb kann der Dateimanager ihn nicht anzeigen. Umbenennen kommt in einer späteren Version.',
    failed: 'Es konnte kein Dateimanager geöffnet werden. Details stehen im Log.',
  },
}
