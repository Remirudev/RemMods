import type { settings as en } from '../en/settings'
import type { LocaleShape } from '../types'

export const settings: LocaleShape<typeof en> = {
  alerts: {
    readOnlyTitle: 'Einstellungen sind schreibgeschützt',
    readOnlyBody:
      'RemMods kann settings.json nicht sicher schreiben: Die Datei stammt von einer neueren RemMods-Version oder konnte nicht gelesen oder gesichert werden. Änderungen hier werden nicht gespeichert, damit die Datei unverändert bleibt. Der Grund steht im Log.',
    recoveredTitle: 'Einstellungen wurden zurückgesetzt',
    recoveredBody:
      'settings.json konnte nicht gelesen werden. RemMods hat daneben eine Sicherungskopie abgelegt und mit den Standardeinstellungen gestartet.',
    loadFailedTitle: 'Einstellungen konnten nicht geladen werden',
  },
  appearance: {
    title: 'Darstellung',
    description: 'Wie RemMods auf diesem Computer aussieht.',
    scheme: {
      label: 'Farbschema',
      dark: 'Dunkel',
      light: 'Hell',
      system: 'System',
    },
    uiScale: {
      label: 'Skalierung',
      hint: 'Tastenkürzel: Strg + = vergrößert, Strg + − verkleinert, Strg + 0 setzt zurück.',
    },
    titleBar: {
      label: 'Titelleiste',
      custom: 'Eigene',
      system: 'System',
      hint: 'Wird nach einem Neustart von RemMods übernommen.',
    },
  },
  game: {
    title: 'Spiel',
    description: 'Der TEKKEN-8-Ordner, mit dem RemMods arbeitet.',
    labels: {
      folder: 'Spielordner',
      paks: 'Paks-Ordner',
      source: 'Gefunden durch',
      build: 'Build',
      steam: 'Steam',
    },
    notSet: 'Kein Spielordner festgelegt.',
    missing: '{{path}} enthält TEKKEN 8 nicht mehr.',
    change: 'Ordner ändern …',
    detect: 'Erneut suchen',
    reveal: 'Paks-Ordner öffnen',
    other: 'Weitere Installationen',
  },
  diagnostics: {
    title: 'Diagnose',
    description: 'Angaben, die bei einer Fehlermeldung helfen.',
    labels: {
      version: 'Version',
      os: 'Betriebssystem',
      arch: 'Architektur',
      webview: 'Webview',
      configDir: 'Einstellungsordner',
      dataDir: 'Datenordner',
      logsDir: 'Log-Ordner',
    },
    osNames: {
      linux: 'Linux',
      windows: 'Windows',
      other: 'Anderes',
    },
    unavailable: 'Nicht verfügbar',
    openLogs: 'Log-Ordner öffnen',
  },
  about: {
    title: 'Über RemMods',
    description: 'Lizenz und Danksagung.',
    license: 'RemMods ist freie Software unter der MIT-Lizenz.',
    credit:
      'Es ist eine Portierung von TekkenModManager von Unfug, ebenfalls unter MIT-Lizenz. Danke für das Original.',
    source: 'Quellcode',
    copyLink: 'Link kopieren',
    linkCopied: 'Link kopiert',
  },
}
