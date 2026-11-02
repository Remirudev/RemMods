import type { ColumnLayout, ColumnSizeLimits, HideableColumnId, ModColumnId } from './types'

// Versioned: a stored layout in another shape gets a new key instead of a migration.
export const COLUMN_LAYOUT_KEY = 'rmods.modsColumns.v1'

export const MOD_COLUMN_IDS: readonly ModColumnId[] = [
  'name',
  'state',
  'kind',
  'version',
  'category',
  'size',
  'conflicts',
  'contentConflicts',
  'issues',
]

export const HIDEABLE_COLUMN_IDS: readonly HideableColumnId[] = [
  'state',
  'kind',
  'version',
  'category',
  'size',
  'conflicts',
  'contentConflicts',
  'issues',
]

// Version and Category mostly show the modinfo.json defaults; hiding them leaves Name room at the
// default window size.
const DEFAULT_HIDDEN: ReadonlySet<string> = new Set<HideableColumnId>(['version', 'category'])

// Name's `size` is the least it gets while it fills the free space (no stored width).
export const COLUMN_SIZES: Readonly<Record<ModColumnId, ColumnSizeLimits>> = {
  name: { size: 240, minSize: 120, maxSize: 1200 },
  state: { size: 130, minSize: 80, maxSize: 480 },
  kind: { size: 100, minSize: 70, maxSize: 480 },
  version: { size: 90, minSize: 60, maxSize: 480 },
  category: { size: 130, minSize: 70, maxSize: 480 },
  size: { size: 100, minSize: 70, maxSize: 480 },
  conflicts: { size: 120, minSize: 80, maxSize: 480 },
  contentConflicts: { size: 140, minSize: 80, maxSize: 480 },
  issues: { size: 110, minSize: 70, maxSize: 480 },
}

const RESIZE_STEP = 10
const RESIZE_STEP_LARGE = 50

export function isModColumnId(id: string): id is ModColumnId {
  return (MOD_COLUMN_IDS as readonly string[]).includes(id)
}

export function clampColumnWidth(id: ModColumnId, width: number): number {
  const { minSize, maxSize } = COLUMN_SIZES[id]
  return Math.round(Math.min(maxSize, Math.max(minSize, width)))
}

// Arrow keys on a resize handle; Shift takes bigger steps. `null` for any other key.
export function keyboardWidth(width: number, key: string, large: boolean): number | null {
  const step = large ? RESIZE_STEP_LARGE : RESIZE_STEP
  if (key === 'ArrowRight') return width + step
  if (key === 'ArrowLeft') return width - step
  return null
}

function visibilityHiding(hidden: ReadonlySet<string>): Record<string, boolean> {
  return Object.fromEntries(HIDEABLE_COLUMN_IDS.map((id) => [id, !hidden.has(id)]))
}

export function defaultColumnLayout(): ColumnLayout {
  return { sizing: {}, visibility: visibilityHiding(DEFAULT_HIDDEN) }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function isValidWidth(id: ModColumnId, width: unknown): width is number {
  const { minSize, maxSize } = COLUMN_SIZES[id]
  return typeof width === 'number' && Number.isFinite(width) && width >= minSize && width <= maxSize
}

function parseWidths(value: unknown): Record<string, number> {
  const sizing: Record<string, number> = {}
  if (!isRecord(value)) return sizing
  for (const id of MOD_COLUMN_IDS) {
    const width = value[id]
    if (isValidWidth(id, width)) sizing[id] = Math.round(width)
  }
  return sizing
}

function parseHidden(value: unknown): Record<string, boolean> {
  if (!Array.isArray(value)) return visibilityHiding(DEFAULT_HIDDEN)
  return visibilityHiding(new Set(value.filter((id): id is string => typeof id === 'string')))
}

// Unknown columns and widths out of range are dropped one by one; a value that is not a layout at
// all gives the defaults.
export function parseColumnLayout(raw: string | null): ColumnLayout {
  if (raw === null) return defaultColumnLayout()
  let data: unknown
  try {
    data = JSON.parse(raw)
  } catch {
    return defaultColumnLayout()
  }
  if (!isRecord(data)) return defaultColumnLayout()
  return { sizing: parseWidths(data.widths), visibility: parseHidden(data.hidden) }
}

export function serializeColumnLayout(layout: ColumnLayout): string {
  const widths: Record<string, number> = {}
  for (const id of MOD_COLUMN_IDS) {
    const width = layout.sizing[id]
    if (width !== undefined) widths[id] = width
  }
  const hidden = HIDEABLE_COLUMN_IDS.filter((id) => layout.visibility[id] === false)
  return JSON.stringify({ widths, hidden })
}
