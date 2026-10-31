import type { shell as en } from '../en/shell'
import type { LocaleShape } from '../types'

export const shell: LocaleShape<typeof en> = {
  titleBar: {
    title: 'RemMods',
    minimize: 'Minimieren',
    maximize: 'Maximieren',
    restore: 'Wiederherstellen',
    close: 'Schließen',
  },
  rail: {
    label: 'Hauptnavigation',
    lightTheme: 'Zum hellen Design wechseln',
    darkTheme: 'Zum dunklen Design wechseln',
  },
  panels: {
    mods: {
      title: 'Mods',
      description: 'Deine installierten TEKKEN-8-Mods.',
      emptyTitle: 'Noch keine Mods',
      emptyDescription: 'Deine Mods erscheinen hier ab Phase 1.',
    },
    profiles: {
      title: 'Profile',
      description: 'Gespeicherte Zusammenstellungen aktivierter Mods.',
      emptyTitle: 'Noch keine Profile',
      emptyDescription:
        'Gespeicherte Mod-Zusammenstellungen erscheinen hier, sobald es Profile gibt.',
    },
    settings: {
      title: 'Einstellungen',
      description: 'Darstellung, Diagnose und Informationen zu RemMods.',
    },
  },
  palette: {
    placeholder: 'Befehl eingeben …',
    nothingFound: 'Keine passenden Befehle',
    goTo: 'Zu {{panel}} wechseln',
    toggleScheme: 'Zwischen hellem und dunklem Design wechseln',
  },
}
