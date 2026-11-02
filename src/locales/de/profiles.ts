import type { profiles as en } from '../en/profiles'
import type { LocaleShape } from '../types'

export const profiles: LocaleShape<typeof en> = {
  unnamed: 'Ohne Namen',
  panel: {
    save: 'Aktuellen Stand speichern…',
    readOnly:
      'profiles.json stammt von einem neueren RemMods, deshalb sind Profile hier schreibgeschützt. Aktualisiere RemMods, um sie zu ändern.',
    problem: {
      corrupt:
        'profiles.json konnte nicht gelesen werden. Eine Sicherung bleibt erhalten, das nächste Speichern beginnt eine neue Liste.',
      newerVersion: 'profiles.json stammt von einem neueren RemMods.',
      unreadable: 'profiles.json konnte nicht gelesen werden. Details stehen im Log.',
    },
    loadError: 'Die Profile konnten nicht geladen werden.',
  },
  mode: {
    additive: {
      label: 'Additiv',
      hint: 'Schaltet die Mods ein, die das Profil als an führt, und die aus, die es als aus führt. Andere Mods bleiben, wie sie sind.',
    },
    exclusive: {
      label: 'Exklusiv',
      hint: 'Schaltet zusätzlich jede aktive Mod aus, die das Profil nicht führt.',
    },
  },
  form: {
    saveTitle: 'Aktuellen Stand speichern',
    editTitle: 'Profil bearbeiten',
    name: 'Name',
    description: 'Beschreibung',
    mode: 'Modus',
    cancel: 'Abbrechen',
    save: 'Speichern',
    saveChanges: 'Änderungen speichern',
    problem: {
      taken: 'Ein Profil mit diesem Namen gibt es schon.',
      invalid: 'Gib einen Namen mit 1 bis 100 Zeichen ein.',
    },
  },
  card: {
    apply: 'Anwenden…',
    update: 'Mit aktuellem Stand aktualisieren',
    updateTitle: 'Profil aktualisieren?',
    updateBody: 'Die in „{{name}}“ gespeicherten Mods durch den aktuellen Stand ersetzen?',
    edit: 'Bearbeiten',
    delete: 'Löschen',
    deleteTitle: 'Profil löschen?',
    deleteBody: '„{{name}}“ löschen? Die Mods selbst bleiben unberührt.',
    fromOtherManager: 'Aus einem anderen Mod-Manager',
    imported: 'Importiert',
    unsaved: 'Ungespeicherte Änderungen seit dem letzten Anwenden',
    counts: '{{enabled}} an, {{disabled}} aus',
    missing_one: '{{count}} fehlt',
    missing_other: '{{count}} fehlen',
    lastApplied: 'Zuletzt angewendet {{when}}',
  },
  diff: {
    title: '„{{name}}“ anwenden',
    error: 'Die Änderungen konnten nicht berechnet werden.',
    changed:
      'Die Mods haben sich auf der Platte geändert. Die Liste unten ist aktuell; prüfe sie und wende erneut an.',
    enable_one: 'Einschalten ({{count}})',
    enable_other: 'Einschalten ({{count}})',
    disable_one: 'Ausschalten ({{count}})',
    disable_other: 'Ausschalten ({{count}})',
    skipped_one: 'Übersprungen ({{count}})',
    skipped_other: 'Übersprungen ({{count}})',
    missing_one: 'Fehlt ({{count}})',
    missing_other: 'Fehlen ({{count}})',
    notInProfile: 'nicht im Profil',
    skipReason: {
      ambiguous: 'mehrere Mods haben diesen Namen',
      linked: 'verknüpfter Ordner',
      unknownState: 'Dateien konnten nicht gelesen werden',
      noPakFiles: 'keine Pak-Dateien',
      invalidLogicName: 'LogicMods-Ordner ohne Namen',
    },
    nothing: 'Nichts zu ändern.',
    cancel: 'Abbrechen',
    apply: 'Anwenden',
  },
  apply: {
    done_one: 'Angewendet: {{count}} Mod geändert.',
    done_other: 'Angewendet: {{count}} Mods geändert.',
    snapshotSaved: 'Ein Schnappschuss des vorherigen Stands wurde gespeichert.',
  },
  snapshots: {
    title_one: 'Automatischer Schnappschuss ({{count}})',
    title_other: 'Automatische Schnappschüsse ({{count}})',
    label: 'Vor dem Anwenden von „{{name}}“ · {{date}}',
    labelDeleted: 'Vor dem Anwenden eines gelöschten Profils · {{date}}',
    deleteTitle: 'Schnappschuss löschen?',
    deleteBody: 'Dieser Schnappschuss wird gelöscht. Die Mods selbst bleiben unberührt.',
  },
  transfer: {
    menu: 'Import / Export',
    importOtherManager: 'Aus einem anderen Mod-Manager importieren…',
    importOtherManagerTitle: 'profiles.json des anderen Mod-Managers wählen',
    import: 'RemMods-Profile importieren…',
    importTitle: 'RemMods-Profilexport wählen',
    export: 'Exportieren…',
    exportTitle: 'Profile exportieren',
    imported:
      'Importiert: {{added}} neu, {{updated}} aktualisiert, {{unchanged}} unverändert, {{skipped}} übersprungen.',
    exported_one: '{{count}} Profil nach {{file}} exportiert.',
    exported_other: '{{count}} Profile nach {{file}} exportiert.',
  },
}
