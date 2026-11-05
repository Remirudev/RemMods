import { describe, expect, it } from 'vitest'
import type { ModInfo } from '../../lib/ipc/types'
import { FIELD_MAX_CHARS, fieldTooLong, isDirty, toEdit } from './metadata-form'

const INFO: ModInfo = {
  version: '1.0',
  category: 'Misc',
  description: '',
  author: 'Ann',
  origin: 'file',
  defaulted: { version: true, category: true, description: true, author: false },
}

describe('toEdit', () => {
  it('turns defaulted fields into empty strings and keeps real values', () => {
    expect(toEdit(INFO)).toEqual({ version: '', category: '', description: '', author: 'Ann' })
  })
})

describe('isDirty', () => {
  it('compares field by field', () => {
    const original = toEdit(INFO)
    expect(isDirty(original, original)).toBe(false)
    expect(isDirty({ ...original, category: 'Skin' }, original)).toBe(true)
    expect(isDirty({ ...original, author: '' }, original)).toBe(true)
  })
})

describe('fieldTooLong', () => {
  it('counts characters, not UTF-16 units', () => {
    const edit = {
      version: '',
      category: '',
      description: '😀'.repeat(FIELD_MAX_CHARS),
      author: '',
    }
    expect(fieldTooLong(edit)).toBeNull()
    expect(fieldTooLong({ ...edit, description: `${edit.description}x` })).toBe('description')
    expect(fieldTooLong({ ...edit, author: 'a'.repeat(FIELD_MAX_CHARS + 1) })).toBe('author')
  })
})
