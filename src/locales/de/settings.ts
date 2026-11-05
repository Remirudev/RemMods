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
  graphics: {
    title: 'Grafik',
    description: 'Wie RemMods sein Fenster unter Linux zeichnet.',
    label: 'Darstellung',
    modes: {
      auto: 'Automatisch',
      forceShm: 'GPU, Shared Memory',
      software: 'Software',
    },
    hint: 'Wird nach einem Neustart von RemMods übernommen. Wähle Software, wenn das Fenster leer bleibt oder flackert.',
    readOnly:
      'graphics.json stammt von einer neueren RemMods-Version oder konnte nicht gelesen werden, deshalb lässt sich der Modus hier nicht ändern.',
    safeFlag: 'Diese Sitzung läuft im sicheren Grafikmodus (gestartet mit --safe-graphics).',
    alwaysSoftware: 'Immer Software-Rendering verwenden',
    userEnv:
      'Deine Umgebung setzt WebKit-Grafikvariablen; RemMods lässt sie unverändert und setzt keine eigenen.',
    step: {
      title: 'Grafikmodus geändert',
      body: 'RemMods ist beim letzten Mal nicht richtig gestartet und hat deshalb von {{from}} zu {{to}} gewechselt.',
      keep: 'Behalten',
      revert: 'Zurückwechseln',
    },
  },
  updates: {
    title: 'Updates',
    description: 'Neue Versionen von RemMods. RemMods sucht nur, wenn du es darum bittest.',
    labels: {
      installedAs: 'Installiert als',
      current: 'Version',
    },
    kinds: {
      appImage: 'AppImage',
      nsis: 'Windows-Installationsprogramm',
      deb: 'Debian-Paket (.deb)',
      rpm: 'RPM-Paket',
      aur: 'AUR-Paket (remmods-bin)',
      local: 'Lokale Installation (cachyos-install.sh)',
      unknown: 'Unbekannt',
    },
    check: 'Nach Updates suchen',
    upToDate: 'RemMods {{version}} ist aktuell.',
    available: 'RemMods {{version}} ist verfügbar.',
    published: 'Veröffentlicht am {{date}}',
    notes: 'Versionshinweise',
    install: 'Installieren und neu starten',
    confirm: {
      title: 'RemMods {{version}} installieren?',
      body: 'RemMods wird geschlossen, um das Update zu installieren.',
      installer: 'Das Installationsprogramm zeigt seinen Fortschritt und startet RemMods wieder.',
      cancel: 'Abbrechen',
    },
    progress: {
      title: 'RemMods wird aktualisiert',
      downloading: 'Wird heruntergeladen … {{percent}} %',
      downloadingSize: 'Wird heruntergeladen … {{size}}',
      installing: 'Wird installiert …',
      restarting: 'RemMods wird neu gestartet …',
    },
    notices: {
      package:
        'Aus einem Paket installiert: Lade das neue {{format}}-Paket von der Release-Seite herunter und installiere es mit deiner Paketverwaltung.',
      aur: 'Aus dem AUR installiert: Aktualisiere mit deinem AUR-Helfer, zum Beispiel paru -Syu.',
      local:
        'Mit cachyos-install.sh installiert: Führe es im RemMods-Ordner erneut aus, um zu aktualisieren.',
      unknown:
        'Diese Kopie wurde nicht aus einem Release-Paket installiert. Lade die neue Version von der Release-Seite herunter.',
      notConfigured: 'Dieser Build hat keinen Update-Kanal (Entwicklungs- oder Test-Build).',
    },
    releases: 'Release-Seite',
    staleCheck: 'Die Update-Informationen sind veraltet. Suche erneut nach Updates.',
  },
  itemPictures: {
    title: 'Item-Bilder und -Namen',
    description:
      'RemMods liest sie einmal aus deinem eigenen Spiel und deinen Mods und bewahrt sie auf diesem PC auf. Nichts wird irgendwohin gesendet. Neue kommen hinzu, solange das Spiel geschlossen ist.',
    summary: '{{pictures}} Bilder ({{size}}), {{names}} Namen',
    empty: 'Noch nichts gespeichert. RemMods füllt das im Hintergrund.',
    failed_one: '{{count}} Bild konnte nicht gelesen werden.',
    failed_other: '{{count}} Bilder konnten nicht gelesen werden.',
    refresh: 'Jetzt prüfen',
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
      installedAs: 'Installiert als',
      graphicsMode: 'Grafikmodus',
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
    description: 'Quellcode und Hinweise zu Drittanbietern.',
    source: 'Quellcode',
    copyLink: 'Link kopieren',
    linkCopied: 'Link kopiert',
    notices: {
      open: 'Hinweise zu Drittanbietern',
      title: 'Hinweise zu Drittanbietern',
      tabs: {
        rust: 'Rust-Crates',
        npm: 'npm-Pakete und Schriften',
      },
      loadFailed: 'Die Hinweise konnten nicht geladen werden.',
    },
  },
}
