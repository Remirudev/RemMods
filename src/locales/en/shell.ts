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
    launchGame: 'Launch TEKKEN 8',
    refreshMods: 'Refresh the mod list',
    openPaks: 'Open the Paks folder',
  },
} as const
