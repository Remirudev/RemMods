import { describe, expect, it } from 'vitest'
import { deResources, enResources } from './resources'

type Tree = { readonly [key: string]: string | Tree }

const PLACEHOLDER = /\{\{\s*([\w.]+)\s*\}\}/g

function flatten(tree: Tree, prefix = ''): Map<string, string> {
  const entries = new Map<string, string>()
  for (const [key, value] of Object.entries(tree)) {
    const path = prefix ? `${prefix}.${key}` : key
    if (typeof value === 'string') {
      entries.set(path, value)
      continue
    }
    for (const [nestedKey, nestedValue] of flatten(value, path)) {
      entries.set(nestedKey, nestedValue)
    }
  }
  return entries
}

function placeholders(text: string): string[] {
  return [...text.matchAll(PLACEHOLDER)].map((match) => match[1] ?? '').sort()
}

const english = flatten(enResources)
const german = flatten(deResources)

describe('locales', () => {
  it('have the same keys in English and German', () => {
    expect([...german.keys()].sort()).toEqual([...english.keys()].sort())
  })

  it('use the same placeholders in English and German', () => {
    for (const [key, text] of english) {
      expect(placeholders(german.get(key) ?? ''), key).toEqual(placeholders(text))
    }
  })

  it('have no empty strings', () => {
    for (const [key, text] of [...english, ...german]) {
      expect(text.trim(), key).not.toBe('')
    }
  })
})
