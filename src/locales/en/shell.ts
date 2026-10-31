export const shell = {
  titleBar: {
    title: 'RemMods',
    minimize: 'Minimize',
    maximize: 'Maximize',
    restore: 'Restore',
    close: 'Close',
  },
  rail: {
    label: 'Main navigation',
    lightTheme: 'Switch to the light theme',
    darkTheme: 'Switch to the dark theme',
  },
  panels: {
    mods: {
      title: 'Mods',
      description: 'Your installed TEKKEN 8 mods.',
      emptyTitle: 'No mods yet',
      emptyDescription: 'Your mods show up here in Phase 1.',
    },
    profiles: {
      title: 'Profiles',
      description: 'Saved sets of enabled mods.',
      emptyTitle: 'No profiles yet',
      emptyDescription: 'Saved sets of mods show up here once profiles arrive.',
    },
    settings: {
      title: 'Settings',
      description: 'Appearance, diagnostics and information about RemMods.',
    },
  },
  palette: {
    placeholder: 'Type a command…',
    nothingFound: 'No matching commands',
    goTo: 'Go to {{panel}}',
    toggleScheme: 'Toggle light / dark theme',
  },
} as const
