export const errors = {
  titles: {
    busy: 'RemMods is busy',
    io: 'File operation failed',
    invalidInput: 'Invalid value',
    readOnly: 'Settings are read-only',
    internal: 'Unexpected error',
    unknown: 'Something went wrong',
  },
  messages: {
    busy: 'Another change is still running. Try again in a moment.',
    invalidInput: 'The value for “{{field}}” is not valid.',
    readOnly:
      'RemMods cannot safely write settings.json, so changes are not saved. The log names the cause.',
    internal: 'An unexpected error occurred. The log has the details.',
    unknown: 'An unexpected error occurred. The log has the details.',
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
} as const
