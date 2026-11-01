export const onboarding = {
  title: 'Where is TEKKEN 8?',
  description: 'RemMods needs the TEKKEN 8 game folder. For now it only reads it.',
  missing: {
    title: 'The saved game folder is gone',
    body: '{{path}} no longer contains TEKKEN 8. Choose the folder again.',
  },
  found: { title: 'Found through Steam', none: 'Steam did not report a TEKKEN 8 installation.' },
  choose: {
    button: 'Choose folder…',
    dialogTitle: 'Choose the TEKKEN 8 folder',
    hint: 'Pick the TEKKEN 8 folder or any folder inside it, such as Paks.',
    cancelled: 'No folder was chosen. You can also type the path.',
  },
  manual: {
    toggle: 'Enter the path instead',
    label: 'Path to the TEKKEN 8 folder',
    placeholder: '…/steamapps/common/TEKKEN 8',
    submit: 'Use this path',
    notAbsolute: 'Enter the full path to the folder.',
  },
  invalid: { title: 'That is not a TEKKEN 8 folder', body: '{{path}}: {{reason}}' },
  detectAgain: 'Search again',
} as const
