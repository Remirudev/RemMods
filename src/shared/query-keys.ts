export const queryKeys = {
  platformInfo: ['platform-info'] as const,
  settings: ['settings'] as const,
  gameStatus: ['game-status'] as const,
  mods: ['mods'] as const,
  watchStatus: ['watchStatus'] as const,
  modDetailsAll: ['mods', 'details'] as const,
  modDetails: (modRef: string) => ['mods', 'details', modRef] as const,
}
