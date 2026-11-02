import { QueryClient } from '@tanstack/react-query'
import { describe, expect, it } from 'vitest'
import { IpcError } from '../../lib/ipc/ipc-error'
import type {
  AppError,
  BulkDeletePrepared,
  BulkOutcome,
  ModsSnapshot,
  ModTarget,
} from '../../lib/ipc/types'
import { storeOrRefetchMods } from '../../shared/mods-cache'
import { queryKeys } from '../../shared/query-keys'
import {
  makeBulkOutcome,
  makeBulkPrepared,
  makeBulkRow,
  makeModRow,
  makePreparedRow,
  makeSnapshot,
} from '../../test/fixtures'
import { deleteMany, trashMany } from './bulk-flow'
import { bulkSelection } from './mod-actions'
import type { BulkDeps, BulkDialog, BulkNotice, BulkResults, BulkSelection } from './types'

const ALPHA = makeModRow('Alpha')
const BRAVO = makeModRow('Bravo')
const CHARLIE = makeModRow('Charlie')
const LOOSE = makeModRow('Loose', { kind: 'loose' })

function fail(error: AppError): IpcError {
  return new IpcError('test', error)
}

function refs(targets: readonly ModTarget[]): string {
  return targets.map((target) => target.key).join(',')
}

interface Script {
  answers?: boolean[]
  running?: boolean[]
  results?: ('deleteUnavailable' | 'close')[]
  trash?: (applyWhileRunning: boolean) => BulkOutcome | Error
  prepare?: (targets: ModTarget[], token: string) => BulkDeletePrepared
  remove?: (token: string, applyWhileRunning: boolean) => BulkOutcome | Error
}

interface Harness {
  deps: BulkDeps
  calls: string[]
  dialogs: BulkDialog[]
  notices: BulkNotice[]
  results: BulkResults[]
  failures: unknown[]
  stored: (ModsSnapshot | null)[]
}

function allEligible(targets: ModTarget[], token: string): BulkDeletePrepared {
  const rows = targets.map((target) => ({ ...makePreparedRow(target.key), target }))
  return makeBulkPrepared(rows, token)
}

function deletedAll(): BulkOutcome {
  return makeBulkOutcome([makeBulkRow('bravo', { kind: 'deleted', leftover: null })])
}

function harness(script: Script = {}): Harness {
  const answers = [...(script.answers ?? [])]
  const running = [...(script.running ?? [])]
  const resultAnswers = [...(script.results ?? [])]
  let tokens = 0
  const h: Harness = {
    deps: undefined as never,
    calls: [],
    dialogs: [],
    notices: [],
    results: [],
    failures: [],
    stored: [],
  }
  h.deps = {
    api: {
      trashMany: async ({ targets, applyWhileRunning }) => {
        h.calls.push(`trash:${refs(targets)}:${applyWhileRunning}`)
        const result = script.trash?.(applyWhileRunning) ?? makeBulkOutcome([])
        if (result instanceof Error) throw result
        return result
      },
      prepareMany: async (targets) => {
        tokens += 1
        const token = `t${tokens}`
        h.calls.push(`prepare:${refs(targets)}:${token}`)
        return (script.prepare ?? allEligible)(targets, token)
      },
      deleteMany: async ({ token, targets, applyWhileRunning }) => {
        h.calls.push(`delete:${refs(targets)}:${token}:${applyWhileRunning}`)
        const result = script.remove?.(token, applyWhileRunning) ?? deletedAll()
        if (result instanceof Error) throw result
        return result
      },
    },
    ask: async (dialog) => {
      h.dialogs.push(dialog)
      return answers.shift() ?? false
    },
    askRunning: async () => {
      h.calls.push('askRunning')
      return running.shift() ?? false
    },
    store: (snapshot) => h.stored.push(snapshot),
    notice: (notice) => h.notices.push(notice),
    showResults: async (results) => {
      h.results.push(results)
      return resultAnswers.shift() ?? 'close'
    },
    fail: (error) => h.failures.push(error),
  }
  return h
}

function trashSelection(): BulkSelection {
  return bulkSelection([ALPHA, BRAVO, LOOSE], 'trash')
}

// Alpha moved; Bravo (a folder) and Loose (loose files: never deletable) were refused.
function partlyTrashed(snapshot: ModsSnapshot | null = makeSnapshot([], { revision: 2 })) {
  return makeBulkOutcome(
    [
      makeBulkRow('Alpha', { kind: 'trashed', items: 1, trash: 'home' }),
      makeBulkRow('Bravo', { kind: 'unavailable', reason: 'otherDevice', size: 4096 }),
      makeBulkRow('Loose', { kind: 'unavailable', reason: 'otherDevice', size: 10 }),
    ],
    snapshot,
  )
}

describe('bulk delete safety', () => {
  it('nothing deleted without the second confirm', async () => {
    const first = harness({ answers: [true, false] })
    await deleteMany(bulkSelection([ALPHA, BRAVO], 'delete'), first.deps)
    expect(first.calls).toEqual(['prepare:alpha,bravo:t1'])
    expect(first.dialogs.map((dialog) => dialog.kind)).toEqual([
      'bulkDeleteFirst',
      'bulkDeleteFinal',
    ])

    const second = harness({ answers: [true], results: ['close'], trash: () => partlyTrashed() })
    await trashMany(trashSelection(), second.deps)
    expect(second.calls).toEqual(['trash:alpha,bravo,loose:false'])
    expect(second.results).toHaveLength(1)
  })

  it('unavailable rows go through prepare and one token', async () => {
    const h = harness({
      answers: [true],
      results: ['deleteUnavailable'],
      trash: () => partlyTrashed(),
    })
    await trashMany(trashSelection(), h.deps)

    expect(h.results[0]?.action).toBe('trash')
    expect(h.results[0]?.subjects?.map((subject) => subject.kind)).toEqual([
      'folder',
      'folder',
      'loose',
    ])
    expect(h.calls).toEqual([
      'trash:alpha,bravo,loose:false',
      'prepare:bravo:t1',
      'delete:bravo:t1:false',
    ])
    expect(h.dialogs.map((dialog) => dialog.kind)).toEqual(['bulkTrash'])
    expect(h.notices).toEqual([{ kind: 'bulkDeleteDone', count: 1 }])
  })

  it('a changed set asks again', async () => {
    const refused: AppError = { kind: 'manageBlocked', action: 'delete', reason: 'journalPending' }
    const h = harness({
      answers: [true, false],
      results: ['deleteUnavailable'],
      trash: () =>
        makeBulkOutcome([
          makeBulkRow('Bravo', { kind: 'unavailable', reason: 'otherDevice', size: 1 }),
          makeBulkRow('Charlie', { kind: 'unavailable', reason: 'otherDevice', size: 2 }),
        ]),
      prepare: (targets, token) =>
        makeBulkPrepared(
          [makePreparedRow('Bravo', 1), makePreparedRow('Charlie', 2, refused)].filter((row) =>
            targets.some((target) => target.key === row.target.key),
          ),
          token,
        ),
    })
    await trashMany(bulkSelection([BRAVO, CHARLIE], 'trash'), h.deps)

    expect(h.dialogs.map((dialog) => dialog.kind)).toEqual(['bulkTrash', 'bulkDeleteFinal'])
    expect(h.calls).toEqual(['trash:bravo,charlie:false', 'prepare:bravo,charlie:t1'])
  })

  it('snapshot null invalidates instead of storing', async () => {
    const h = harness({ answers: [true], results: ['close'], trash: () => partlyTrashed(null) })
    await trashMany(trashSelection(), h.deps)
    expect(h.stored).toEqual([null])
    expect(h.results).toHaveLength(1)

    const client = new QueryClient()
    const cached = makeSnapshot([ALPHA])
    client.setQueryData(queryKeys.mods, cached)
    storeOrRefetchMods(client, null)
    expect(client.getQueryData(queryKeys.mods)).toBe(cached)
    expect(client.getQueryState(queryKeys.mods)?.isInvalidated).toBe(true)
    const fresh = makeSnapshot([], { revision: 3 })
    storeOrRefetchMods(client, fresh)
    expect(client.getQueryData(queryKeys.mods)).toEqual(fresh)
  })

  it('expired token shows a notice and never retries', async () => {
    const h = harness({
      answers: [true, true],
      remove: () => fail({ kind: 'deleteTokenInvalid' }),
    })
    await deleteMany(bulkSelection([BRAVO], 'delete'), h.deps)
    expect(h.calls).toEqual(['prepare:bravo:t1', 'delete:bravo:t1:false'])
    expect(h.notices).toEqual([{ kind: 'deleteExpired' }])
    expect(h.failures).toEqual([])
  })

  it('running game after a spent token asks, prepares fresh, calls once with applyWhileRunning', async () => {
    const h = harness({
      answers: [true, true],
      running: [true],
      remove: (_token, applyWhileRunning) =>
        applyWhileRunning ? deletedAll() : fail({ kind: 'gameRunning' }),
    })
    await deleteMany(bulkSelection([BRAVO], 'delete'), h.deps)
    expect(h.calls).toEqual([
      'prepare:bravo:t1',
      'delete:bravo:t1:false',
      'askRunning',
      'prepare:bravo:t2',
      'delete:bravo:t2:true',
    ])
    expect(h.notices).toEqual([{ kind: 'bulkDeleteDone', count: 1 }])
  })

  it('a declined running dialog makes no further call', async () => {
    const h = harness({
      answers: [true, true],
      running: [false],
      remove: () => fail({ kind: 'gameRunning' }),
    })
    await deleteMany(bulkSelection([BRAVO], 'delete'), h.deps)
    expect(h.calls).toEqual(['prepare:bravo:t1', 'delete:bravo:t1:false', 'askRunning'])
    expect(h.failures).toEqual([])
  })
})

describe('bulk results', () => {
  it('all trashed shows a toast, no results dialog', async () => {
    const h = harness({
      answers: [true],
      trash: () =>
        makeBulkOutcome([
          makeBulkRow('Alpha', { kind: 'trashed', items: 1, trash: 'recycleBin' }),
          makeBulkRow('Bravo', { kind: 'trashed', items: 1, trash: 'recycleBin' }),
        ]),
    })
    await trashMany(bulkSelection([ALPHA, BRAVO], 'trash'), h.deps)
    expect(h.notices).toEqual([{ kind: 'bulkTrashDone', count: 2, trash: 'recycleBin' }])
    expect(h.results).toEqual([])
  })

  it('failed and notRun rows reach the results', async () => {
    const rows = [
      makeBulkRow('Alpha', { kind: 'failed', error: { kind: 'busyCancelled' } }),
      makeBulkRow('Bravo', { kind: 'notRun' }),
    ]
    const h = harness({ answers: [true, true], remove: () => makeBulkOutcome(rows) })
    await deleteMany(bulkSelection([ALPHA, BRAVO], 'delete'), h.deps)
    expect(h.results).toEqual([{ action: 'delete', rows }])
    expect(h.notices).toEqual([])
  })

  it('a running game on trash asks once and repeats the same call', async () => {
    const h = harness({
      answers: [true],
      running: [true],
      trash: (applyWhileRunning) =>
        applyWhileRunning ? makeBulkOutcome([]) : fail({ kind: 'gameRunning' }),
    })
    await trashMany(bulkSelection([ALPHA], 'trash'), h.deps)
    expect(h.calls).toEqual(['trash:alpha:false', 'askRunning', 'trash:alpha:true'])
  })

  it('prepare refusals without a token are shown, nothing is deleted', async () => {
    const refusal: AppError = { kind: 'stale' }
    const h = harness({
      answers: [true],
      prepare: (targets, _token) =>
        makeBulkPrepared(
          targets.map((target) => ({ ...makePreparedRow(target.key, null, refusal), target })),
          null,
        ),
    })
    await deleteMany(bulkSelection([BRAVO], 'delete'), h.deps)
    expect(h.calls).toEqual(['prepare:bravo:t1'])
    expect(h.results[0]?.rows.map((row) => row.result.kind)).toEqual(['failed'])
  })
})
