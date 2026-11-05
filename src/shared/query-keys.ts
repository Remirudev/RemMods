export const queryKeys = {
  platformInfo: ['platform-info'] as const,
  settings: ['settings'] as const,
  gameStatus: ['game-status'] as const,
  mods: ['mods'] as const,
  repair: ['repair'] as const,
  profiles: ['profiles'] as const,
  profileDiff: (id: string, direction: string) => ['profiles', 'diff', id, direction] as const,
  watchStatus: ['watchStatus'] as const,
  updateChannel: ['update-channel'] as const,
  graphics: ['graphics'] as const,
  notices: ['notices'] as const,
  modDetailsAll: ['mods', 'details'] as const,
  modDetails: (modRef: string) => ['mods', 'details', modRef] as const,
  modContents: (modRef: string) => ['mods', 'details', 'contents', modRef] as const,
  modItemsAll: ['mods', 'details', 'items'] as const,
  modItems: (modRef: string, language: string) =>
    ['mods', 'details', 'items', modRef, language] as const,
  assets: ['assets'] as const,
  assetsStatus: (language: string) => ['assets', 'status', language] as const,
  // Outside `assets`: a picture never changes, so "assets changed" need not load it again.
  itemPicture: (id: string) => ['item-picture', id] as const,
}
