import { deResources, enResources } from '../../src/lib/i18n/resources'

type Values = Record<string, string | number>

function lookup(tree: unknown, path: readonly string[]): unknown {
  let node = tree
  for (const part of path) {
    if (typeof node !== 'object' || node === null) return undefined
    node = (node as Record<string, unknown>)[part]
  }
  return node
}

function translate(resources: object, key: string, values: Values): string {
  const path = key.split('.')
  let text = lookup(resources, path)
  // English plural forms: `_one` for a count of 1, `_other` for every other count.
  if (text === undefined && typeof values.count === 'number') {
    const suffix = values.count === 1 ? '_one' : '_other'
    text = lookup(resources, [...path.slice(0, -1), `${path[path.length - 1]}${suffix}`])
  }
  if (typeof text !== 'string') throw new Error(`no locale string for ${key}`)
  const filled = text.replace(/\{\{(\w+)\}\}/g, (_match, name: string) => {
    const value = values[name]
    if (value === undefined) throw new Error(`${key} needs a value for {{${name}}}`)
    return String(value)
  })
  return filled
}

// The UI's own strings, so a spec never hard-codes English: `t('mods.actionBar.trash', { count: 3,
// place: t('mods.trash.place.trash') })`. Keys start with the namespace.
export function t(key: string, values: Values = {}): string {
  return translate(enResources, key, values)
}

export function tDe(key: string, values: Values = {}): string {
  return translate(deResources, key, values)
}
