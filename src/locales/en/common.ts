export const common = {
  appName: 'RemMods',
  actions: {
    tryAgain: 'Try again',
    reload: 'Reload app',
  },
  errorBoundary: {
    title: 'Something broke in {{area}}.',
    titleGeneric: 'Something broke.',
    hint: 'The rest of RemMods keeps working. Try again, or reload the app if it keeps happening.',
  },
} as const
