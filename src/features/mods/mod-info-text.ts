import type { TFunction } from 'i18next'
import type { ModInfo } from '../../lib/ipc/types'
import type { ModInfoField } from './types'

// Category and author defaults are words, so they are shown translated; the version default "1.0"
// and the empty description stay as they are. Search and sort keep using the raw values.
export function modInfoText(info: ModInfo, field: ModInfoField, t: TFunction<'mods'>): string {
  if (!info.defaulted[field]) return info[field]
  switch (field) {
    case 'category':
      return t(($) => $.info.defaults.category)
    case 'author':
      return t(($) => $.info.defaults.author)
    case 'version':
    case 'description':
      return info[field]
  }
}
