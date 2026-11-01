export const errors = {
  titles: {
    busy: 'RemMods is busy',
    io: 'File operation failed',
    invalidInput: 'Invalid value',
    readOnly: 'Settings are read-only',
    internal: 'Unexpected error',
    unknown: 'Something went wrong',
    stale: 'List out of date',
    gameNotSet: 'No game folder',
    invalidGameFolder: 'Not a TEKKEN 8 folder',
    launchFailed: 'TEKKEN 8 could not be started',
    revealFailed: 'Could not open the file manager',
  },
  messages: {
    busy: 'Another change is still running. Try again in a moment.',
    invalidInput: 'The value for “{{field}}” is not valid.',
    readOnly:
      'RemMods cannot safely write settings.json, so changes are not saved. The log names the cause.',
    internal: 'An unexpected error occurred. The log has the details.',
    unknown: 'An unexpected error occurred. The log has the details.',
    stale: 'The files changed on disk. RemMods refreshed the list; try again.',
    gameNotSet: 'Choose the TEKKEN 8 folder first.',
    launchFailed:
      'RemMods could not hand steam://rungameid/1778820 to Steam. Start the game from Steam; the log has the details.',
  },
  ioClasses: {
    inUse: 'The file is in use by another program.',
    accessDenied: 'Access was denied.',
    noSpace: 'There is not enough free space on the drive.',
    readOnlyFs: 'The drive is read-only.',
    crossDevice: 'The file would have to move to another drive.',
    nameTooLong: 'The name or path is too long.',
    notFound: 'The file or folder was not found.',
    unsupported: 'This is not supported on this system.',
    other: 'The operating system reported an error. The log has the details.',
  },
  folderProblems: {
    notFound: 'No TEKKEN 8 game files were found there.',
    ambiguous: 'Several folders there differ only in case, for example Polaris and polaris.',
    notUnicode: 'The path contains characters RemMods cannot store.',
    unreadable: 'The folder could not be read.',
  },
  revealProblems: {
    unsupportedName:
      'This name is not valid on Windows, so the file manager cannot show it. Renaming comes in a later version.',
    failed: 'No file manager could be opened. The log has the details.',
  },
} as const
