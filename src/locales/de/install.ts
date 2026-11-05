import type { install as en } from '../en/install'
import type { LocaleShape } from '../types'

export const install: LocaleShape<typeof en> = {
  overlay: {
    title: 'Archive oder Ordner zum Installieren hier ablegen',
    picture: 'Hier ablegen, um es als Bild von {{name}} zu nehmen',
    count_one: '{{count}} Element',
    count_other: '{{count}} Elemente',
  },
  picture: {
    noModTitle: 'Zu welcher Mod gehört dieses Bild?',
    noMod: 'Öffne zuerst die Details einer Mod und lege das Bild dann noch einmal ab.',
  },
  pick: {
    archives: 'Mod-Archive auswählen',
    folder: 'Mod-Ordner auswählen',
  },
  menu: {
    button: 'Installieren…',
    archives: 'Aus Archiven…',
    folder: 'Aus einem Ordner…',
  },
  palette: {
    archives: 'Archive installieren…',
    folder: 'Ordner installieren…',
  },
  row: {
    install: 'Installieren',
    installLabel: '{{name}} installieren',
  },
  rejected: {
    title: 'Einige Elemente lassen sich nicht installieren',
    reasons: {
      unsupportedType: 'Weder ein zip-, 7z- oder rar-Archiv noch ein Ordner.',
      missing:
        'RemMods kommt nicht heran. Stammt es von einem Netzwerk- oder Cloud-Ort, kopiere es zuerst in einen lokalen Ordner.',
      link: 'Verknüpfungen werden nicht verfolgt. Wähle die echte Datei oder den echten Ordner.',
      insidePaks: 'Dieser Ordner liegt im Paks-Ordner des Spiels.',
      tooMany: 'Zu viele Elemente auf einmal. Lege den Rest danach noch einmal ab.',
    },
  },
  modal: {
    title: 'Mod installieren',
    titleFor: '{{name}} installieren',
    waiting_one: 'Noch {{count}} wartet',
    waiting_other: 'Noch {{count}} warten',
  },
  inspecting: '{{name}} wird gelesen…',
  actions: {
    install: 'Installieren',
    cancel: 'Abbrechen',
    close: 'Schließen',
    next: 'Weiter',
    back: 'Zurück',
    skip: 'Überspringen',
  },
  formats: {
    zip: 'ZIP',
    sevenZip: '7z',
    rar: 'RAR',
    folder: 'Ordner',
    image: 'Bild',
  },
  summary: {
    files_one: '{{count}} Datei',
    files_other: '{{count}} Dateien',
    free: '{{size}} frei',
    freeUnknown: 'Freier Speicher unbekannt',
  },
  root: {
    label: 'Installieren in',
    options: {
      standard: 'Mods',
      legacy: '~mods',
      logic: 'LogicMods',
    },
    reasons: {
      pathSegment: 'Das Archiv nennt diesen Ordner.',
      configLua: 'Eine config.lua neben den Paks kennzeichnet eine LogicMods-Mod.',
      default: 'Der übliche Ordner für Pak-Mods.',
      chosen: 'Du hast diesen Ordner gewählt.',
    },
  },
  encoding: {
    label: 'Kodierung der Dateinamen',
    hint: 'Die Dateinamen sehen nach einer alten Kodierung aus. Wähle die, die die richtigen Zeichen zeigt.',
    options: {
      auto: 'Automatisch',
      shiftJis: 'Shift_JIS (Japanisch)',
      gbk: 'GBK (Chinesisch)',
      big5: 'Big5 (Chinesisch)',
      eucKr: 'EUC-KR (Koreanisch)',
      ibm866: 'IBM866 (Russisch)',
      windows1251: 'Windows-1251 (Kyrillisch)',
    },
  },
  candidates: {
    title: 'Was installiert wird',
    variantHint: 'Das sind Alternativen. Wähle eine.',
    nameLabel: 'Ordnername von {{name}}',
    from: 'Aus {{inner}}',
    payload_one: '{{count}} Pak-Datei',
    payload_other: '{{count}} Pak-Dateien',
    shippedOff: 'Kommt ausgeschaltet: {{count}}. Sie bleiben aus.',
    willRename: 'Wird in „{{name}}“ umbenannt.',
    nameProblem: 'Dieser Name lässt sich so nicht verwenden.',
    replaces: 'Ersetzt „{{rel}}“.',
    replaceSwitch: 'Ersetzen (die alte Version kommt in den Papierkorb)',
    ambiguous:
      'Mehrere installierte Mods passen zu diesem Namen. Kläre das zuerst im Reiter „Reparieren“.',
    alsoIn: 'Eine Mod mit diesem Namen gibt es auch in {{roots}}.',
    pathTooLong: 'Einige Pfade sind für das Spiel womöglich zu lang.',
  },
  fixes: {
    autoFix: 'Automatisch umbenennen',
    autoFixHint: 'Ersetzt Zeichen, die nicht auf jedem System erlaubt sind.',
  },
  clashes: {
    title: 'Namen, die sich nur in der Groß-/Kleinschreibung unterscheiden',
    hint: 'Wähle pro Gruppe die Datei, die bleibt. Die anderen werden nicht installiert.',
  },
  issues: {
    title: 'Hinweise',
    severity: {
      block: 'Blockiert',
      warn: 'Bitte prüfen',
      info: 'Zur Information',
    },
    unsafePath: 'Dateipfade, die den Mod-Ordner verlassen. Es wurde nichts entpackt.',
    link: 'Verknüpfungen im Archiv werden übersprungen: {{count}}',
    special: 'Sonderdateien (Geräte, Pipes) werden übersprungen: {{count}}',
    antiItem: 'Löschmarkierungen im Archiv werden übersprungen: {{count}}',
    duplicateIdentical: 'Identische doppelte Dateien werden nur einmal übernommen: {{count}}',
    duplicateDiffers: 'Dateien mit gleichem Pfad, aber anderem Inhalt: {{count}}',
    duplicateRaw: 'Mehrere Einträge mit demselben Rohnamen: {{count}}',
    caseClash:
      'Dateien, deren Namen sich nur in der Groß-/Kleinschreibung unterscheiden: {{count}}',
    invalidName: 'Namen, die nicht auf jedem System erlaubt sind: {{count}}',
    notPortable: 'Namen, die hier funktionieren, aber nicht unter Windows: {{count}}',
    undecodable: 'Dateinamen ließen sich nicht lesen. Probiere eine andere Kodierung.',
    legacyNames: 'Die Dateinamen nutzen eine alte Kodierung. Prüfe, ob sie richtig aussehen.',
    nestedArchive: 'Archive im Archiv werden nicht installiert. Entpacke sie zuerst: {{count}}',
    shipsDisabled: 'Dateien, die ausgeschaltet kommen, bleiben ausgeschaltet: {{count}}',
    shipsBothStates: 'Ein Pak kommt ein- und ausgeschaltet. Das Archiv muss erst repariert werden.',
    scriptMod: 'Das ist eine Skript-Mod (UE4SS). RemMods installiert die noch nicht.',
    noPayload: 'Keine Pak-Dateien gefunden.',
    mixedRoots: 'Das Archiv mischt Mods-, ~mods- und LogicMods-Ordner.',
    outsideRoot: 'Dateien außerhalb des Mod-Ordners werden übersprungen: {{count}}',
    extraFiles: 'Andere Dateien neben den Mods werden nicht installiert: {{count}}',
    pathTooLong: 'Pfade, die für das Spiel womöglich zu lang sind: {{count}}',
    alsoInRoot: 'Eine Mod mit diesem Namen gibt es auch in einem anderen Ordner: {{count}}',
    ambiguous:
      'Mehrere installierte Mods passen zu einem Namen. Kläre das zuerst im Reiter „Reparieren“.',
    replaces: 'Eine installierte Mod wird ersetzt. Die alte Version kommt in den Papierkorb.',
    noSpace: 'Der freie Speicher reicht womöglich nicht.',
    emptyDirs: 'Leere Ordner werden ignoriert: {{count}}',
  },
  blockers: {
    blocked: 'Die Quelle hat Probleme, die die Installation verhindern.',
    noSelection: 'Wähle mindestens eine Mod.',
    ambiguous: 'Eine installierte Mod mit demselben Namen ist nicht eindeutig.',
    replaceNotConfirmed: 'Bestätige, welche installierten Mods ersetzt werden.',
    unresolvedClash: 'Wähle für jeden Groß-/Kleinschreibungs-Konflikt die Datei, die bleibt.',
    invalidNames: 'Schalte das automatische Umbenennen ein oder benenne die Mods selbst um.',
    noSpace: 'Es ist nicht genug freier Speicher vorhanden.',
  },
  notices: {
    stale:
      'Auf dem Datenträger hat sich etwas geändert, deshalb wurde die Quelle neu gelesen. Prüfe die Liste und versuche es erneut.',
    busy: 'Eine andere Installation läuft. Versuche es erneut, wenn sie fertig ist.',
  },
  running: {
    title: 'TEKKEN 8 läuft',
    body: 'Die Mod wird für den nächsten Start installiert. Bereits geladene Mods bleiben bis dahin unverändert.',
    question: 'Für den nächsten Start installieren?',
    confirm: 'Für den nächsten Start installieren',
  },
  progress: {
    phase: {
      preparing: 'Vorbereitung…',
      extracting: 'Wird entpackt…',
      committing: 'Mods werden platziert…',
      cleaningUp: 'Wird aufgeräumt…',
    },
    files: '{{done}} / {{total}} Dateien',
    sizes: '{{done}} von {{total}}',
  },
  result: {
    title: 'Installiert',
    installed_one: '{{count}} Mod installiert',
    installed_other: '{{count}} Mods installiert',
    replacedTrashed: 'Ersetzt. Die alte Version liegt im Papierkorb.',
    replacedKept:
      'Ersetzt. Die alte Version konnte nicht in den Papierkorb und bleibt unter Reparatur.',
    files_one: '{{count}} Datei',
    files_other: '{{count}} Dateien',
    cancelled: 'Abgebrochen. Es wurde nichts installiert.',
  },
  failure: {
    title: 'Installation fehlgeschlagen',
    hints: {
      extractFirst:
        'Entpacke es mit einem anderen Programm und installiere stattdessen den Ordner.',
    },
  },
}
