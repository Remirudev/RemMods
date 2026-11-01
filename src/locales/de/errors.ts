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
