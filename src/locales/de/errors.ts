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
  },
  messages: {
    busy: 'Eine andere Änderung läuft noch. Versuch es gleich nochmal.',
    invalidInput: 'Der Wert für „{{field}}“ ist ungültig.',
    readOnly:
      'RemMods kann settings.json nicht sicher schreiben, deshalb werden Änderungen nicht gespeichert. Der Grund steht im Log.',
    internal: 'Ein unerwarteter Fehler ist aufgetreten. Details stehen im Log.',
    unknown: 'Ein unerwarteter Fehler ist aufgetreten. Details stehen im Log.',
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
}
