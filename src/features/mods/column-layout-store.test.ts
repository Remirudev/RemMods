import { afterEach, describe, expect, it, vi } from 'vitest'
import { COLUMN_LAYOUT_KEY, defaultColumnLayout } from './column-layout'

class MemoryStorage {
  readonly items = new Map<string, string>()
  getItem(key: string): string | null {
    return this.items.get(key) ?? null
  }
  setItem(key: string, value: string): void {
    this.items.set(key, value)
  }
  removeItem(key: string): void {
    this.items.delete(key)
  }
}

const failing = {
  getItem: () => {
    throw new Error('storage disabled')
  },
  setItem: () => {
    throw new Error('storage disabled')
  },
  removeItem: () => {
    throw new Error('storage disabled')
  },
}

// The store reads storage once, when its module loads: each call is a fresh app start.
async function startWith(storage: object) {
  vi.stubGlobal('window', { localStorage: storage })
  vi.resetModules()
  const { useColumnLayoutStore } = await import('./column-layout-store')
  return useColumnLayoutStore
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('column layout store', () => {
  it('starts with the defaults when nothing is stored', async () => {
    const store = await startWith(new MemoryStorage())
    const { sizing, visibility } = store.getState()
    expect({ sizing, visibility }).toEqual(defaultColumnLayout())
  })

  it('keeps widths and visible columns for the next start', async () => {
    const storage = new MemoryStorage()
    const first = await startWith(storage)
    first.getState().setWidth('name', 412)
    first.getState().saveLayout()
    first.getState().setColumnVisible('version', true)
    first.getState().setColumnVisible('issues', false)

    const next = await startWith(storage)
    expect(next.getState().sizing).toEqual({ name: 412 })
    expect(next.getState().visibility).toMatchObject({ version: true, issues: false })
  })

  it('saves a dragged width only when the drag commits', async () => {
    const storage = new MemoryStorage()
    const store = await startWith(storage)
    store.getState().setWidth('state', 200)
    expect(storage.getItem(COLUMN_LAYOUT_KEY)).toBeNull()
    store.getState().saveLayout()
    expect(JSON.parse(storage.getItem(COLUMN_LAYOUT_KEY) ?? '{}').widths).toEqual({ state: 200 })
  })

  it('clamps widths to the column limits', async () => {
    const store = await startWith(new MemoryStorage())
    store.getState().setWidth('size', 1)
    expect(store.getState().sizing.size).toBe(70)
  })

  it('ignores invalid stored data', async () => {
    const storage = new MemoryStorage()
    storage.setItem(COLUMN_LAYOUT_KEY, '{"widths":{"name":"wide","bogus":300},"hidden":{}}')
    const store = await startWith(storage)
    const { sizing, visibility } = store.getState()
    expect({ sizing, visibility }).toEqual(defaultColumnLayout())

    storage.setItem(COLUMN_LAYOUT_KEY, '{ not json')
    const again = await startWith(storage)
    expect(again.getState().sizing).toEqual({})
  })

  it('resets one width, and the whole layout back to the defaults', async () => {
    const storage = new MemoryStorage()
    const store = await startWith(storage)
    store.getState().setWidth('name', 500)
    store.getState().setWidth('kind', 150)
    store.getState().resetWidth('name')
    expect(store.getState().sizing).toEqual({ kind: 150 })
    expect(JSON.parse(storage.getItem(COLUMN_LAYOUT_KEY) ?? '{}').widths).toEqual({ kind: 150 })

    store.getState().resetLayout()
    const { sizing, visibility } = store.getState()
    expect({ sizing, visibility }).toEqual(defaultColumnLayout())
    expect(storage.getItem(COLUMN_LAYOUT_KEY)).toBeNull()
  })

  it('keeps working in memory when storage throws', async () => {
    const store = await startWith(failing)
    expect(store.getState().sizing).toEqual({})
    store.getState().setColumnVisible('kind', false)
    store.getState().setWidth('name', 300)
    store.getState().saveLayout()
    store.getState().resetLayout()
    expect(store.getState().visibility.kind).toBe(true)
  })
})
