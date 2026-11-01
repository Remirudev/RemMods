import type { ModRow } from '../../lib/ipc/types'
import type { NameIssueCode } from './types'

const FORBIDDEN = new Set(['<', '>', ':', '"', '/', '\\', '|', '?', '*'])
const PLAIN_DEVICES = new Set(['con', 'prn', 'aux', 'nul', 'conin$', 'conout$'])
const BIDI = /[؜‎‏‪-‮⁦-⁩]/u
const CONTROL = /\p{Cc}/u
const NAME_MAX_BYTES = 255
const TEMP_MARKER = '_tmp_'
const TEMP_SUFFIX = /^[0-9a-fA-F]{32}$/
const DISABLED_PREFIX = '.disabled_'
const PORT_DIGITS = new Set(['1', '2', '3', '4', '5', '6', '7', '8', '9', '¹', '²', '³'])

// A hint for the dialog only: the core's fold_key decides. Same intent: NFC plus caseless.
export function foldName(name: string): string {
  return name.normalize('NFC').toLocaleLowerCase('und')
}

function isReservedDeviceName(name: string): boolean {
  const stem = (name.split('.')[0] ?? '').replace(/ +$/u, '').toLowerCase()
  if (PLAIN_DEVICES.has(stem)) return true
  const chars = [...stem]
  const port = chars.slice(0, 3).join('')
  return (port === 'com' || port === 'lpt') && chars.length === 4 && PORT_DIGITS.has(chars[3] ?? '')
}

function isTempName(name: string): boolean {
  if (!name.startsWith('_')) return false
  const marker = name.lastIndexOf(TEMP_MARKER)
  return marker >= 0 && TEMP_SUFFIX.test(name.slice(marker + TEMP_MARKER.length))
}

// Mirrors the core's validate_name (N1-N10) in its order, then the unchanged and exists hints.
export function checkName(
  name: string,
  current: string,
  siblingsFolded: ReadonlySet<string>,
): NameIssueCode | null {
  if (name === current) return 'unchanged'
  if (name === '') return 'empty'
  if (name === '.' || name === '..') return 'dot_name'
  if (new TextEncoder().encode(name).length > NAME_MAX_BYTES) return 'too_long'
  if ([...name].some((char) => FORBIDDEN.has(char))) return 'forbidden_char'
  if (CONTROL.test(name)) return 'control_char'
  if (BIDI.test(name)) return 'bidi_control'
  if (name.endsWith('.') || name.endsWith(' ')) return 'trailing_dot_or_space'
  if (isReservedDeviceName(name)) return 'reserved_device_name'
  if (foldName(name).startsWith(DISABLED_PREFIX)) return 'disabled_prefix'
  if (isTempName(name)) return 'temp_name'
  if (siblingsFolded.has(foldName(name))) return 'exists'
  return null
}

// Folded names of the other rows in the same root, both on-disk forms for LogicMods folders.
export function siblingFolds(rows: readonly ModRow[], row: ModRow | undefined): Set<string> {
  const folds = new Set<string>()
  if (!row) return folds
  for (const other of rows) {
    if (other.modRef === row.modRef || other.root !== row.root) continue
    folds.add(foldName(other.folderName))
    folds.add(foldName(other.name))
    if (other.kind === 'logic') folds.add(foldName(`${DISABLED_PREFIX}${other.name}`))
  }
  return folds
}
