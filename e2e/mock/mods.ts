import type {
  CharacterHint,
  DeleteRequest,
  FolderBusy,
  MetadataRequest,
  ModContents,
  ModDetails,
  ModItem,
  ModItems,
  ModRow,
  ModTarget,
  RenameOutcome,
  RenameRequest,
  ToggleOutcome,
  ToggleRequest,
  TrashOutcome,
  TrashRequest,
} from '../../src/lib/ipc/types'
import { TEST_INSTALL } from '../../src/test/fixtures'
import { changed, fail, findRow, isEnabled, removeRow, rowFor, setEnabled, targetOf } from './state'
import type { EventEmitter, MockState } from './types'

const BUSY_LIMIT_MS = 30_000

// Settles a pending folder-busy wait; only `mods_busy_cancel` calls these.
const cancellers = new Map<number, () => void>()

function busyPayload(
  waitId: number,
  folder: string,
  phase: FolderBusy['phase'],
  elapsedMs: number,
): FolderBusy {
  return {
    waitId,
    phase,
    folder,
    holders: [],
    elapsedMs,
    limitMs: BUSY_LIMIT_MS,
    cancellable: true,
  }
}

// A folder another program holds: the toggle waits (announcing it about once a second) until the
// user cancels, then rejects with busyCancelled and leaves every row as it was.
function waitForBusyFolder(state: MockState, row: ModRow, emit: EventEmitter): Promise<never> {
  const waitId = state.nextId
  state.nextId += 1
  // The real payload names the folder below Paks, never the absolute path.
  const folder = `${row.root}/${row.folderName}`
  state.waits.push({ waitId, key: row.key, folder })
  return new Promise<never>((_resolve, reject) => {
    let elapsed = 1000
    void emit('folder-busy', busyPayload(waitId, folder, 'waiting', elapsed))
    const timer = setInterval(() => {
      elapsed += 1000
      void emit('folder-busy', busyPayload(waitId, folder, 'waiting', elapsed))
    }, 1000)
    cancellers.set(waitId, () => {
      clearInterval(timer)
      cancellers.delete(waitId)
      state.waits = state.waits.filter((wait) => wait.waitId !== waitId)
      void emit('folder-busy', busyPayload(waitId, folder, 'cancelled', elapsed))
      reject({ kind: 'busyCancelled' })
    })
  })
}

export function busyCancel(waitId: number): boolean {
  const cancel = cancellers.get(waitId)
  if (!cancel) return false
  cancel()
  return true
}

export function toggle(
  state: MockState,
  request: ToggleRequest,
  emit: EventEmitter,
): ToggleOutcome | Promise<never> {
  const rows = request.targets.map((target) => rowFor(state, target))
  const busy = rows.find((row) => state.busyKeys.includes(row.key))
  if (busy) return waitForBusyFolder(state, busy, emit)
  const blocked = rows.find((row) => row.toggleBlock !== null)
  if (blocked?.toggleBlock) fail({ kind: 'toggleBlocked', reason: blocked.toggleBlock })
  for (const row of rows) setEnabled(row, request.enable)
  return { snapshot: changed(state), skipped: [], unmanaged: 0 }
}

export function disableAll(state: MockState): ToggleOutcome {
  const rows = state.rows.filter((row) => isEnabled(row) && row.toggleBlock === null)
  for (const row of rows) setEnabled(row, false)
  return { snapshot: changed(state), skipped: [], unmanaged: 0 }
}

export function rename(state: MockState, request: RenameRequest): RenameOutcome {
  const row = rowFor(state, request.target)
  const name = request.newName.trim()
  if (name === '') fail({ kind: 'invalidName', issue: 'empty' })
  const key = name.toLowerCase()
  if (state.rows.some((other) => other !== row && other.key === key)) fail({ kind: 'targetExists' })
  const oldKey = row.key
  row.name = name
  row.folderName = name
  row.key = key
  row.modRef = `m:${key}`
  row.location = `${TEST_INSTALL.paks}/${row.root}/${name}`
  return {
    snapshot: changed(state),
    oldKey,
    newKey: key,
    newRef: row.modRef,
    rekey: { profiles: 0, meta: false, skippedAmbiguous: false, failed: false },
  }
}

export function setMetadata(state: MockState, request: MetadataRequest) {
  const row = rowFor(state, request.target)
  row.info = {
    ...request.info,
    origin: 'file',
    defaulted: { version: false, category: false, description: false, author: false },
  }
  return changed(state)
}

export function details(state: MockState, modRef: string): ModDetails {
  const row = findRow(state, modRef) ?? fail({ kind: 'stale' })
  return {
    modRef,
    location: row.location,
    files: [
      {
        relPath: `${row.folderName}.pak`,
        kind: 'pak',
        live: isEnabled(row),
        sizeBytes: row.sizeBytes ?? 0,
      },
    ],
    truncated: false,
    otherFiles: 0,
    added: null,
    source: null,
  }
}

export function contents(state: MockState, modRef: string): ModContents {
  if (!findRow(state, modRef)) fail({ kind: 'stale' })
  return { modRef, base: 'complete', entries: [], total: 0, truncated: false }
}

const KUNIMITSU: CharacterHint = { code: 'ker', name: 'Kunimitsu', common: false }
const REINA: CharacterHint = { code: 'zbr', name: 'Reina', common: false }

function item(character: CharacterHint, overrides: Partial<ModItem>): ModItem {
  return {
    key: `${character.code}_bdu_blouse`,
    character,
    slot: { code: 'bdu', slot: 'upper' },
    name: 'Frilled blouse',
    nameSource: 'itemList',
    gamePicture: `game-${character.code}`,
    modPicture: `mod-${character.code}`,
    live: true,
    origin: 'replaces',
    ...overrides,
  }
}

// Every mod: an outfit for two characters, Kunimitsu with one new item and one hidden piece.
export function items(state: MockState, modRef: string): ModItems {
  const row = findRow(state, modRef)
  if (!row) fail({ kind: 'stale' })
  return {
    modRef,
    ownPicture: state.ownPictures[row.key] ?? null,
    items: [
      item(KUNIMITSU, {}),
      item(REINA, { name: 'Rüschenbluse (Reina)' }),
      item(KUNIMITSU, {
        key: 'ker_bdf_yor',
        slot: { code: 'bdf', slot: 'fullBody' },
        name: 'Yor Style',
        nameSource: 'modText',
        gamePicture: null,
        origin: 'adds',
      }),
      item(KUNIMITSU, {
        key: 'ker_sho_hidden',
        slot: { code: 'sho', slot: 'shoes' },
        name: 'Barefoot',
        modPicture: null,
      }),
    ],
  }
}

export function setPicture(state: MockState, target: ModTarget): null {
  state.ownPictures[rowFor(state, target).key] = 'own-picture'
  return null
}

export function clearPicture(state: MockState, target: ModTarget): null {
  delete state.ownPictures[rowFor(state, target).key]
  return null
}

export function trash(state: MockState, request: TrashRequest): TrashOutcome {
  const row = rowFor(state, request.target)
  const refusal = state.trashRefusals[row.key]
  if (refusal) fail({ kind: 'trashUnavailable', reason: refusal.reason, size: row.sizeBytes })
  removeRow(state, row)
  state.trashed.push(row.key)
  return { snapshot: changed(state), key: row.key, items: 1, kind: state.trashKind }
}

// Single use and bound to the exact ordered target list it was issued for.
export function issueToken(state: MockState, targets: ModTarget[]): string {
  const token = `token-${state.nextId}-${Math.random().toString(16).slice(2, 10)}`
  state.nextId += 1
  state.tokens.push({ token, targets: targets.map((target) => ({ ...target })) })
  state.issuedTokens.push(token)
  return token
}

function sameTargets(left: readonly ModTarget[], right: readonly ModTarget[]): boolean {
  return (
    left.length === right.length &&
    left.every(
      (target, index) => target.modRef === right[index]?.modRef && target.key === right[index]?.key,
    )
  )
}

export function consumeToken(state: MockState, token: string, targets: ModTarget[]): void {
  const issued = state.tokens.find((candidate) => candidate.token === token)
  state.tokens = state.tokens.filter((candidate) => candidate.token !== token)
  if (!issued || !sameTargets(issued.targets, targets)) fail({ kind: 'deleteTokenInvalid' })
}

export function deletePrepare(state: MockState, target: ModTarget) {
  const row = rowFor(state, target)
  if (row.kind === 'loose') fail({ kind: 'manageBlocked', action: 'delete', reason: 'loose' })
  return {
    token: issueToken(state, [targetOf(row)]),
    key: row.key,
    name: row.name,
    size: row.sizeBytes,
    expiresInSecs: 120,
  }
}

export function deletePermanently(state: MockState, request: DeleteRequest) {
  consumeToken(state, request.token, [request.target])
  const row = rowFor(state, request.target)
  removeRow(state, row)
  state.deleted.push(row.key)
  return { snapshot: changed(state), key: row.key, leftover: null }
}
