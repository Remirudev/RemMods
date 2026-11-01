import type { ModInfo, ModInfoEdit } from '../../lib/ipc/types'
import type { ModInfoField } from './types'

export const FIELD_MAX_CHARS = 10_000

export const EDIT_FIELDS: readonly ModInfoField[] = ['version', 'category', 'author', 'description']

// A defaulted field starts empty: the default shows as a placeholder, and saving it blank lets
// the core write the default again.
export function toEdit(info: ModInfo): ModInfoEdit {
  const pick = (field: ModInfoField) => (info.defaulted[field] ? '' : info[field])
  return {
    version: pick('version'),
    category: pick('category'),
    description: pick('description'),
    author: pick('author'),
  }
}

export function isDirty(edit: ModInfoEdit, original: ModInfoEdit): boolean {
  return EDIT_FIELDS.some((field) => edit[field] !== original[field])
}

export function fieldTooLong(edit: ModInfoEdit): ModInfoField | null {
  return EDIT_FIELDS.find((field) => [...edit[field]].length > FIELD_MAX_CHARS) ?? null
}
