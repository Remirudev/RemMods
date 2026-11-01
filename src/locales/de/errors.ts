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
