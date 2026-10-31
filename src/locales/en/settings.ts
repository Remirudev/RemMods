export const settings = {
  alerts: {
    readOnlyTitle: 'Settings are read-only',
    readOnlyBody:
      'RemMods cannot safely write settings.json: it comes from a newer version of RemMods, or it could not be read or backed up. Changes made here are not saved, so the file stays intact. The log names the cause.',
    recoveredTitle: 'Settings were reset',
    recoveredBody:
      'settings.json could not be read. RemMods kept a backup copy next to it and started with the default settings.',
    loadFailedTitle: 'Settings could not be loaded',
  },
  appearance: {
    title: 'Appearance',
    description: 'How RemMods looks on this computer.',
    scheme: {
      label: 'Color scheme',
      dark: 'Dark',
      light: 'Light',
      system: 'System',
    },
    uiScale: {
      label: 'UI scale',
      hint: 'Shortcuts: Ctrl + = to enlarge, Ctrl + − to shrink, Ctrl + 0 to reset.',
    },
    titleBar: {
      label: 'Title bar',
      custom: 'Custom',
      system: 'System',
      hint: 'Applies after RemMods restarts.',
    },
  },
  diagnostics: {
    title: 'Diagnostics',
    description: 'Details to include when you report a problem.',
    labels: {
      version: 'Version',
      os: 'Operating system',
      arch: 'Architecture',
      webview: 'Webview',
      configDir: 'Settings folder',
      dataDir: 'Data folder',
      logsDir: 'Log folder',
    },
    osNames: {
      linux: 'Linux',
      windows: 'Windows',
      other: 'Other',
    },
    unavailable: 'Not available',
    openLogs: 'Open logs folder',
  },
  about: {
    title: 'About',
    description: 'License and credits.',
    license: 'RemMods is free software under the MIT license.',
    credit:
      'It is a port of TekkenModManager by Unfug, which is MIT-licensed as well. Thank you for the original.',
    source: 'Source code',
    copyLink: 'Copy link',
    linkCopied: 'Link copied',
  },
} as const
