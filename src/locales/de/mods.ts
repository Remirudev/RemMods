import type { mods as en } from '../en/mods'
import type { LocaleShape } from '../types'

export const mods: LocaleShape<typeof en> = {
  header: {
    refresh: 'Aktualisieren',
    openPaks: 'Paks-Ordner öffnen',
    scannedAt: 'Gelesen um {{time}}',
    conflicts_one: '{{count}} Mod im Konflikt',
    conflicts_other: '{{count}} Mods im Konflikt',
  },
  tabs: {
    mods: 'Mods',
    archives: 'Archive',
    unmanaged: 'Noch geladen',
    issues: 'Probleme',
    repair: 'Reparieren',
  },
  filters: {
    search: 'Mods durchsuchen',
    searchPlaceholder: 'Name, Kategorie, Autor oder Typ',
    state: 'Status',
    states: {
      all: 'Alle',
      enabled: 'Aktiv',
      partial: 'Teilweise',
      disabledLoaded: 'Inaktiv · wird geladen',
      disabled: 'Inaktiv',
      noPakFiles: 'Keine Pak-Dateien',
      unknown: 'Unbekannt',
    },
    kinds: 'Typ',
    conflictsOnly: 'Konflikte',
    contentConflictsOnly: 'Ersetzt gleiche Dateien',
    issuesOnly: 'Probleme',
    clear: 'Filter zurücksetzen',
    count_one: '{{count}} von {{total}} Mod',
    count_other: '{{count}} von {{total}} Mods',
  },
  columns: {
    toggle: 'An',
    name: 'Name',
    state: 'Status',
    kind: 'Typ',
    version: 'Version',
    category: 'Kategorie',
    size: 'Größe',
    conflicts: 'Konflikte',
    issues: 'Probleme',
  },
  states: {
    enabled: 'Aktiv',
    partial: 'Teilweise',
    disabledLoaded: 'Inaktiv · wird geladen',
    disabled: 'Inaktiv',
    noPakFiles: 'Keine Pak-Dateien',
    unknown: 'Unbekannt',
  },
  stateHints: {
    enabled: 'Alle .pak-Dateien sind aktiv.',
    partial: 'Einige .pak-Dateien sind aktiv, andere sind abgeschaltet (-x).',
    disabledLoaded: 'Nur dem Namen nach deaktiviert: Das Spiel lädt seine .pak-Dateien weiterhin.',
    disabled: 'Alle .pak-Dateien sind abgeschaltet (-x).',
    noPakFiles: 'Enthält keine .pak- oder .pak-x-Datei.',
    unknown:
      'RemMods konnte die .pak-Dateien nicht zählen: Der Ordner ist nicht lesbar, oder sie liegen in einem verknüpften Ordner.',
  },
  toggle: { enable: '{{name}} aktivieren', disable: '{{name}} deaktivieren' },
  actionBar: {
    label: 'Ausgewählte Mods',
    selected_one: '{{count}} ausgewählt',
    selected_other: '{{count}} ausgewählt',
    enable: 'Aktivieren',
    disable: 'Deaktivieren',
    trash_one: 'In den {{place}} ({{count}})',
    trash_other: 'In den {{place}} ({{count}})',
    delete_one: 'Endgültig löschen… ({{count}})',
    delete_other: 'Endgültig löschen… ({{count}})',
    clear: 'Auswahl aufheben (Esc)',
  },
  bulk: {
    skippedByMatrix_one:
      '{{count}} ausgewählte Mod wird übersprungen (verknüpft, lose Dateien oder Archiv).',
    skippedByMatrix_other:
      '{{count}} ausgewählte Mods werden übersprungen (verknüpft, lose Dateien oder Archiv).',
    trashConfirm: {
      title: 'In den {{place}} verschieben?',
      body_one:
        '{{count}} Mod ({{size}}) in den {{place}} verschieben? Du kannst sie dort wiederherstellen.',
      body_other:
        '{{count}} Mods ({{size}}) in den {{place}} verschieben? Du kannst sie dort wiederherstellen.',
      more: 'und {{count}} weitere',
      confirm: 'In den {{place}}',
    },
    deleteFirst: {
      title: 'Endgültig löschen?',
      body_one:
        'Das löscht {{count}} Mod für immer, statt sie in den {{place}} zu verschieben. Vor dem Löschen wirst du noch einmal gefragt.',
      body_other:
        'Das löscht {{count}} Mods für immer, statt sie in den {{place}} zu verschieben. Vor dem Löschen wirst du noch einmal gefragt.',
      confirm: 'Weiter',
    },
    deleteFinal: {
      title: 'Diese Mods für immer löschen?',
      body: 'Diese Mods werden endgültig gelöscht:',
      skipped: 'Übersprungen: {{reason}}',
      total: 'Insgesamt: {{size}}',
      confirm: 'Endgültig löschen ({{count}})',
    },
    results: {
      title: 'Nicht alles hat geklappt',
      trashed: 'In den Papierkorb verschoben',
      recycled: 'In den Papierkorb verschoben',
      deleted: 'Gelöscht',
      leftover:
        'Gelöscht, aber ein versteckter Teil ist übrig: {{path}}. Der Reiter Reparieren listet ihn.',
      unavailable: 'Nicht verschoben: {{reason}}',
      notDeletable: 'Nicht verschoben, und hier kann sie nicht gelöscht werden: {{reason}}',
      failed: 'Fehlgeschlagen: {{message}}',
      cancelled: 'Beim Warten auf den Ordner abgebrochen; diese Mod wurde nicht geändert.',
      notRun: 'Nicht versucht (abgebrochen)',
      deleteUnavailable_one: 'Endgültig löschen ({{count}})',
      deleteUnavailable_other: 'Endgültig löschen ({{count}})',
      deleteWarning_one: 'Diese Mod ({{size}}) würde stattdessen endgültig gelöscht.',
      deleteWarning_other: 'Diese {{count}} Mods ({{size}}) würden stattdessen endgültig gelöscht.',
      total: 'Insgesamt: {{size}}',
      close: 'Schließen',
    },
    done: {
      trash_one: '{{count}} Mod wurde in den Papierkorb verschoben.',
      trash_other: '{{count}} Mods wurden in den Papierkorb verschoben.',
      recycle_one: '{{count}} Mod wurde in den Papierkorb verschoben.',
      recycle_other: '{{count}} Mods wurden in den Papierkorb verschoben.',
      delete_one: '{{count}} Mod wurde gelöscht.',
      delete_other: '{{count}} Mods wurden gelöscht.',
    },
    nothingEligible: 'Keine dieser Mods kann gelöscht werden. Nichts wurde geändert.',
  },
  folderBusy: {
    title: 'Warte auf einen belegten Ordner',
    body: '„{{folder}}“ wird von einem anderen Programm benutzt, zum Beispiel einem Explorer-Fenster, einem Virenscan oder der Suchindizierung. RemMods versucht es bis zu {{limit}} s lang erneut.',
    bodyUndo:
      'RemMods stellt „{{folder}}“ wieder her und muss darauf warten; das lässt sich nicht abbrechen.',
    elapsed: 'Wartet seit {{seconds}} s',
    cancel: 'Abbrechen',
    cancelling: 'Wird abgebrochen…',
    holders: 'Benutzt von: {{names}}',
  },
  quickApply: {
    button: 'Profil anwenden',
    empty: 'Noch keine gespeicherten Profile',
    last: 'Zuletzt angewendet',
    readOnly: 'Profile können gerade nicht gespeichert werden; siehe Bereich Profile',
  },
  watch: {
    limitReached:
      'Die automatische Aktualisierung ist aus: Das Systemlimit für überwachte Ordner ist erreicht. Drücke F5, nachdem du Mod-Dateien außerhalb von RemMods geändert hast.',
    failed:
      'Die automatische Aktualisierung ist nicht verfügbar. Drücke F5, nachdem du Mod-Dateien außerhalb von RemMods geändert hast.',
  },
  disableAll: {
    button: 'Alle Mods deaktivieren',
    confirmTitle: 'Alle Mods deaktivieren?',
    confirmBody_one: '{{count}} Mod wird ausgeschaltet. Nicht verwaltete Dateien bleiben geladen.',
    confirmBody_other:
      '{{count}} Mods werden ausgeschaltet. Nicht verwaltete Dateien bleiben geladen.',
    confirm: 'Alle deaktivieren',
    cancel: 'Abbrechen',
    done_one: '{{count}} Mod deaktiviert',
    done_other: '{{count}} Mods deaktiviert',
    skipped: '{{count}} übersprungen',
    unmanaged_one: '{{count}} nicht verwaltete Datei noch geladen',
    unmanaged_other: '{{count}} nicht verwaltete Dateien noch geladen',
  },
  kinds: { standard: 'Mods', legacy: '~mods', logic: 'LogicMods', loose: 'Lose' },
  size: { unknown: 'Unbekannt' },
  info: { defaults: { category: 'Sonstiges', author: 'Unbekannt' } },
  badges: {
    conflicts_one: '{{count}} Konflikt',
    conflicts_other: '{{count}} Konflikte',
    issues_one: '{{count}} Problem',
    issues_other: '{{count}} Probleme',
    ambiguous: 'Mehrdeutig',
    contentConflicts_one: 'Ersetzt gleiche Dateien · {{count}}',
    contentConflicts_other: 'Ersetzt gleiche Dateien · {{count}}',
    contentConflictsTooltip: 'Andere Versionen derselben Dateien in: {{names}}',
  },
  table: { label: 'Installierte Mods', noMatch: 'Kein Mod passt zu den Filtern.' },
  empty: {
    title: 'Keine Mods gefunden',
    description: 'Mods, ~mods und LogicMods sind in {{paks}} leer oder fehlen.',
  },
  error: { title: 'Die Mod-Ordner konnten nicht gelesen werden', retry: 'Erneut versuchen' },
  notices: {
    caseVariantRoots:
      'Ordner, die sich nur in Groß- und Kleinschreibung unterscheiden, liegen nebeneinander: {{names}}. Das Spiel lädt womöglich beide.',
    unreadableRoot: '{{root}} konnte nicht gelesen werden: {{reason}}',
  },
  drawer: {
    key: 'Schlüssel',
    folder: 'Ort',
    kind: 'Typ',
    state: 'Status',
    size: 'Größe',
    payload: 'Pak-Dateien',
    payloadValue: '{{livePak}} aktiv, {{offPak}} aus',
    payloadCompanions_one: '{{count}} Begleitdatei',
    payloadCompanions_other: '{{count}} Begleitdateien',
    info: 'modinfo.json',
    edit: 'Bearbeiten',
    added: 'Hinzugefügt',
    source: 'Quelle',
    infoFile: 'Aus modinfo.json gelesen.',
    infoMissing: 'Keine modinfo.json; es werden Standardwerte angezeigt.',
    infoInvalid: 'modinfo.json konnte nicht gelesen werden; es werden Standardwerte angezeigt.',
    version: 'Version',
    category: 'Kategorie',
    author: 'Autor',
    description: 'Beschreibung',
    descriptionEmpty: 'Keine Beschreibung.',
    conflicts: 'Konflikte',
    conflictsNone: 'Kein anderer Mod verwendet dieselben .pak-Namen.',
    conflictPaks: 'Gemeinsam: {{paks}}',
    issues: 'Probleme',
    issuesNone: 'Nichts zu melden.',
    files: 'Pak-Dateien auf der Festplatte',
    filesLoading: 'Ordner wird gelesen …',
    filesError: 'Der Ordner konnte nicht gelesen werden.',
    filesTruncated: 'Die ersten {{count}} Dateien werden angezeigt.',
    otherFiles_one: 'und {{count}} weitere Datei',
    otherFiles_other: 'und {{count}} weitere Dateien',
    live: 'aktiv',
    off: 'aus',
    reveal: 'Im Dateimanager zeigen',
    close: 'Schließen',
  },
  content: {
    title: 'Inhalt',
    summary_one: '{{count}} Asset in {{containers}} Container(n)',
    summary_other: '{{count}} Assets in {{containers}} Container(n)',
    noUtoc: 'Diese Mod hat keine .utoc-Dateien.',
    characters: 'Charaktere',
    slots: 'Slots',
    allCharacters: 'Alle Charaktere',
    rawCodeHint: 'Interner Code, noch nicht zugeordnet',
    filter: 'Assets filtern',
    showAll: 'Alle {{count}} anzeigen',
    truncated: 'Nur die ersten {{shown}} von {{total}} Dateien werden gezeigt.',
    origin: { adds: 'Neu', replaces: 'Ersetzt', unknown: 'Unbekannt' },
    baseNote: {
      partial:
        'Ob eine Datei Spielinhalt ersetzt oder hinzufügt, ist unbekannt: Der Index des Spiels ließ sich nicht vollständig lesen.',
      unavailable:
        'Der Index des Spiels ließ sich nicht lesen. RemMods erkennt daher nicht, welche Dateien Spieldateien ersetzen, und prüft keine Konflikte.',
      loading:
        'Der Index des Spiels wird noch gelesen. Konflikte zwischen Mods erscheinen, sobald er bereit ist.',
    },
    sharedWith_one: 'Andere Version in {{count}} anderen Mod',
    sharedWith_other: 'Andere Versionen in {{count}} anderen Mods',
    conflictReplaces_one: 'Beide ersetzen diese Spieldatei',
    conflictReplaces_other: 'Ersetzt diese Spieldatei, wie {{count}} andere Mods',
    sharedIdentical_one: '{{count}} Datei teilt diese Mod mit anderen Mods als identische Kopie.',
    sharedIdentical_other:
      '{{count}} Dateien teilt diese Mod mit anderen Mods als identische Kopien.',
    sharedOtherVersions_one:
      '{{count}} gemeinsame Mod-Datei in anderen Versionen. Sie ist keine Spieldatei, daher kein Konflikt.',
    sharedOtherVersions_other:
      '{{count}} gemeinsame Mod-Dateien in anderen Versionen. Sie sind keine Spieldateien, daher kein Konflikt.',
    internalOverlaps_one: '{{count}} Asset ist in mehreren Containern dieser Mod enthalten.',
    internalOverlaps_other: '{{count}} Assets sind in mehreren Containern dieser Mod enthalten.',
    status: {
      read: 'Gelesen',
      notIoStore: 'Kein IoStore-Container',
      unsupportedVersion: 'Nicht unterstützte Version {{version}}',
      encrypted: 'Verschlüsselt: Inhalt nicht lesbar',
      noIndex: 'Kein Dateiindex',
      malformed: 'Beschädigt oder unbekanntes Format',
      tooLarge: 'Index zu groß',
      unreadable: 'Nicht lesbar',
    },
    unsupportedNote: 'Nicht unterstützt: {{status}}',
    off: 'deaktiviert',
  },
  contentConflicts: {
    title: 'Ersetzt dieselben Dateien',
    item_one: '{{count}} Datei in einer anderen Version als {{name}}',
    item_other: '{{count}} Dateien in einer anderen Version als {{name}}',
    winner: 'Welche davon das Spiel zeigt, ist noch nicht bekannt; das wird im Spiel geprüft.',
  },
  slots: {
    upper: 'Oberkörper',
    lower: 'Unterkörper',
    bottom: 'Hose/Rock',
    fullBody: 'Ganzkörper',
    hair: 'Haare',
    head: 'Kopf',
    fullHead: 'Ganzer Kopf',
    hat: 'Hut',
    face: 'Gesicht',
    facePaint: 'Gesichtsbemalung',
    faceHair: 'Gesichtsbehaarung',
    eyeMakeup: 'Augen-Make-up',
    eyes: 'Augen',
    cheek: 'Wange',
    lips: 'Lippen',
    glasses: 'Brille',
    accessory: 'Accessoire',
    shoes: 'Schuhe',
    extra: 'Extra',
  },
  archives: {
    description:
      'Archive, die in Paks, Mods, ~mods oder LogicMods liegen. Das Spiel ignoriert sie; installiere eines, um es als Mod zu entpacken.',
    empty: 'Keine Archive.',
    reveal: 'Im Dateimanager zeigen',
  },
  unmanaged: {
    description: 'Aktive Pak-Dateien, die RemMods nicht verwaltet. Das Spiel lädt sie trotzdem.',
    empty: 'Nichts Unverwaltetes.',
    kinds: {
      paksFile: 'Lose Datei in Paks',
      folder: 'Ordner in Paks',
      linkedFolder: 'Verknüpfter Ordner',
      tempFolder: 'Unvollständige Installation',
    },
    liveFiles_one: '{{count}} aktive Datei',
    liveFiles_other: '{{count}} aktive Dateien',
    notInspected: 'Nicht untersucht',
    copyOf: 'Kopie von Dateien aus {{mods}}',
    reveal: 'Im Dateimanager zeigen',
  },
  issues: {
    description: 'Was RemMods aufgefallen ist. Hier wird nichts automatisch geändert.',
    empty: 'Keine Probleme.',
    fix: 'Beheben',
    leftovers_one:
      '{{count}} versteckter Ordner einer unvollständigen Installation (nicht geladen)',
    leftovers_other: '{{count}} versteckte Ordner unvollständiger Installationen (nicht geladen)',
  },
  issue: {
    ambiguousKey:
      'Eine andere Zeile hat denselben Schlüssel. Änderungen an diesen Zeilen werden blockiert.',
    caseCollision:
      'Unterscheidet sich von {{others}} nur in Groß-/Kleinschreibung oder Unicode-Form; Windows und manche Mod-Programme behandeln sie womöglich als einen Namen.',
    caseVariantRoot:
      'Liegt in {{root}}, das sich nur in Groß- und Kleinschreibung von einem anderen Stammordner unterscheidet.',
    nonUnicodeName: 'Der Name ist kein gültiges Unicode und wird maskiert angezeigt.',
    invalidName: 'Der Name ist unter Windows ungültig: {{problem}}.',
    nameNotPortable: 'RemMods würde diesen Namen nicht anlegen: {{problem}}.',
    disabledPrefixName:
      'Benannt wie ein deaktivierter LogicMod, aber in diesem Ordner ist es ein normaler Mod.',
    emptyName: 'Der Ordner heißt nur „.disabled_“.',
    logicDisabledButLoaded:
      'Nur dem Namen nach deaktiviert: Das Spiel lädt seine .pak-Dateien weiterhin.',
    logicEnabledNotLoaded:
      'Dem Namen nach aktiv, aber alle .pak-Dateien sind abgeschaltet (-x), daher lädt das Spiel ihn nicht.',
    modinfoInvalid: 'modinfo.json konnte nicht gelesen werden; es werden Standardwerte angezeigt.',
    modinfoCaseVariants:
      'Mehrere modinfo.json-Dateien unterscheiden sich nur in Groß- und Kleinschreibung; keine wird verwendet.',
    pathTooLong:
      'Pfade erreichen {{longest}} Zeichen; Windows und das Spiel können ab 259 (Ordner: 247) scheitern.',
    linkedRoot: 'Sein Stammordner ist eine Verknüpfung zu einem anderen Ort.',
    linkedMod: 'Dieser Mod ist eine Verknüpfung zu einem anderen Ordner.',
    containsLink:
      'Dateien im verknüpften Ordner werden nicht gezählt; das Spiel lädt sie trotzdem.',
    looseCopy:
      'Eine Kopie von {{files}} liegt an anderer Stelle in Paks und wird weiterhin geladen.',
    looseCaseVariants:
      'Seine Dateien haben Namen, die sich nur in Groß- und Kleinschreibung unterscheiden.',
    nonUnicodeFiles_one: '{{count}} Dateiname ist kein gültiges Unicode.',
    nonUnicodeFiles_other: '{{count}} Dateinamen sind kein gültiges Unicode.',
    invalidFileNames_one: '{{count}} Dateiname ist unter Windows ungültig.',
    invalidFileNames_other: '{{count}} Dateinamen sind unter Windows ungültig.',
    nonPortableFileNames_one:
      '{{count}} Dateiname verstößt gegen die Namensregeln von RemMods (zu lang oder versteckte Zeichen).',
    nonPortableFileNames_other:
      '{{count}} Dateinamen verstoßen gegen die Namensregeln von RemMods (zu lang oder versteckte Zeichen).',
    unreadable: 'Ein Teil des Ordners konnte nicht gelesen werden: {{reason}}',
  },
  nameProblems: {
    empty: 'er ist leer',
    dotName: 'er ist „.“ oder „..“',
    tooLong: 'er ist zu lang',
    forbiddenChar: 'er enthält < > : " / \\ | ? oder *',
    controlChar: 'er enthält ein Steuerzeichen',
    bidiControl: 'er enthält ein unsichtbares Schreibrichtungszeichen',
    trailingDotOrSpace: 'er endet mit einem Punkt oder Leerzeichen',
    reservedDeviceName: 'er ist ein reservierter Gerätename wie CON, NUL oder COM1',
    disabledPrefix: 'er beginnt mit .disabled_',
    tempName: 'er sieht wie der Ordner einer unvollständigen Installation aus',
  },
  rowMenu: {
    label: 'Aktionen für {{name}}',
    rename: 'Umbenennen…',
    trash: 'In den {{place}}',
    delete: 'Endgültig löschen…',
  },
  confirm: {
    otherManagerRunning:
      'Ein anderer Mod-Manager läuft und bemerkt diese Änderung womöglich erst nach einem Neustart.',
  },
  rename: {
    title: 'Mod umbenennen',
    label: 'Neuer Name',
    submit: 'Umbenennen',
    cancel: 'Abbrechen',
    done: '„{{from}}“ wurde in „{{to}}“ umbenannt.',
    rekeyFailed:
      'Die Mod wurde umbenannt, aber Profile oder gespeicherte Daten konnten nicht angepasst werden. Details stehen im Log.',
  },
  nameIssue: {
    empty: 'Der Name darf nicht leer sein.',
    dot_name: 'Ein oder zwei Punkte sind kein Name.',
    too_long: 'Der Name ist zu lang (höchstens 255 Bytes).',
    forbidden_char: 'Der Name darf kein < > : " / \\ | ? * enthalten.',
    control_char: 'Der Name darf keine Steuerzeichen enthalten.',
    bidi_control: 'Der Name darf keine Steuerzeichen für die Schreibrichtung enthalten.',
    trailing_dot_or_space: 'Der Name darf nicht mit Punkt oder Leerzeichen enden.',
    reserved_device_name: 'Windows reserviert diesen Namen (CON, NUL, COM1 und ähnliche).',
    disabled_prefix: 'Der Name darf nicht mit „.disabled_“ beginnen.',
    temp_name: 'Der Name sieht nach einem unfertigen Installationsordner aus.',
    unchanged: 'Der Name ist derselbe wie vorher.',
    exists: 'Eine andere Mod oder ein anderer Ordner heißt schon so.',
    invalid: 'Dieser Name kann nicht verwendet werden.',
  },
  trash: {
    place: { trash: 'Papierkorb', recycleBin: 'Papierkorb' },
    confirm: {
      title: 'In den {{place}} verschieben?',
      body: '„{{name}}“ ({{size}}) in den {{place}} verschieben? Du kannst es dort wiederherstellen.',
      confirm: 'In den {{place}}',
    },
    done: '„{{name}}“ wurde in den Papierkorb verschoben.',
    doneRecycle: '„{{name}}“ wurde in den Papierkorb von Windows verschoben.',
    unavailable: {
      title: 'Der {{place}} ist nicht nutzbar',
      otherDevice:
        'Dieser Ordner liegt auf einem anderen Laufwerk als dein Papierkorb, deshalb kann er ihn nicht aufnehmen.',
      noUsableTrash: 'Für dieses Laufwerk wurde kein nutzbarer Papierkorb gefunden.',
      notSupported: 'Das Verschieben in den {{place}} ist in diesem Lauf nicht verfügbar.',
      recycleRefused:
        'Windows hat diesen Ordner nicht in den Papierkorb verschoben (der Papierkorb ist vielleicht aus oder zu klein dafür, oder das Laufwerk hat keinen).',
      notShellSafe:
        'Windows kann diesen Namen nicht in den Papierkorb verschieben. Benenne ihn zuerst unter Reparieren um oder lösche ihn endgültig.',
      warning: '„{{name}}“ ({{size}}) würde stattdessen endgültig gelöscht.',
      confirm: 'Endgültig löschen',
    },
  },
  delete: {
    cancel: 'Abbrechen',
    confirm: {
      title: 'Endgültig löschen?',
      body: 'Das löscht „{{name}}“ für immer, statt es in den {{place}} zu verschieben. Vor dem Löschen wirst du noch einmal gefragt.',
      confirm: 'Weiter',
    },
    final: {
      title: '{{name}} für immer löschen?',
      body: '„{{name}}“ ({{size}}) wird endgültig gelöscht.',
      irreversible: 'Das lässt sich nicht rückgängig machen.',
      confirm: 'Endgültig löschen',
    },
    done: '„{{name}}“ wurde gelöscht.',
    expired:
      'Die Bestätigung ist abgelaufen. Nichts wurde gelöscht; starte noch einmal, wenn du es weiter willst.',
    leftover:
      'Ein Teil der Mod konnte nicht entfernt werden und bleibt versteckt: {{path}}. Der Reiter Reparieren listet ihn.',
  },
  metadata: {
    save: 'Speichern',
    cancel: 'Abbrechen',
    confirmTitle: 'modinfo.json speichern?',
    ambiguous:
      'Mehrere Dateien unterscheiden sich nur in der Groß- und Kleinschreibung (modinfo.json). Entferne zuerst die überzähligen.',
    tooLong: 'Ein Feld ist länger als {{max}} Zeichen.',
  },
  repair: {
    description:
      'Dinge, die RemMods gefunden hat und die deine Aufmerksamkeit brauchen. Jede Aktion fragt nach, bevor sie etwas ändert.',
    empty: 'Nichts zu reparieren.',
    kinds: {
      leftover: 'Übrig gebliebener Ordner',
      ambiguousKey: 'Gleicher Name doppelt',
      invalidName: 'Ungültiger Name',
      nonUnicodeName: 'Nicht lesbarer Name',
      logicDisabledButLoaded: 'Per Name aus, aber geladen',
      logicEnabledNotLoaded: 'Per Name an, aber nicht geladen',
      tempFolderLive: 'Unfertiger Installationsordner',
      stuckJournal: 'Unfertige Änderung',
    },
    hints: {
      leftover:
        'Ein versteckter Ordner von einem unterbrochenen Löschen oder Installieren. Daraus wird nichts geladen.',
      ambiguousKey:
        'Mehrere Mods teilen sich einen Namen, deshalb kann RemMods sie nicht unterscheiden. Benenne eine um.',
      invalidName:
        'Der Ordnername verletzt eine Namensregel. Ein gültiger Name wird vorgeschlagen.',
      nonUnicodeName:
        'Der Ordnername lässt sich nicht als Text lesen. Ein gültiger Name wird vorgeschlagen.',
      logicDisabledButLoaded: 'Der Ordner ist als aus markiert, das Spiel lädt aber seine Dateien.',
      logicEnabledNotLoaded:
        'Der Ordner ist als an markiert, seine Dateien sind aber ausgeschaltet.',
      tempFolderLive:
        'Dieser Ordner enthält noch aktive Spieldateien und sieht nach einer nicht beendeten Installation aus.',
      stuckJournal:
        'Eine Umbenennung ließ sich nicht rückgängig machen. RemMods versucht es beim nächsten Start erneut; Details stehen im Log.',
    },
    actions: {
      rename: 'Umbenennen…',
      trash: 'In den {{place}}',
      deletePermanently: 'Endgültig löschen…',
      reveal: 'Im Dateimanager zeigen',
      finishDisable: 'Ausschalten abschließen',
      finishEnable: 'Einschalten abschließen',
      openLogs: 'Log öffnen',
    },
  },
  severity: { error: 'Fehler', warning: 'Warnung', info: 'Hinweis' },
}
