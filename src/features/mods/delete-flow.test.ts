import { describe, expect, it } from 'vitest'
import { IpcError } from '../../lib/ipc/ipc-error'
import type { AppError, DeletePrepared, ModsSnapshot } from '../../lib/ipc/types'
import { makeModRow, makeSnapshot } from '../../test/fixtures'
import { deletePermanently, trashWithFallback } from './delete-flow'
import { rowSubject } from './manage-model'
import type { FlowDeps, FlowDialog, FlowNotice } from './types'

const SUBJECT = rowSubject(makeModRow('Alpha'))
const SNAPSHOT: ModsSnapshot = makeSnapshot([], { revision: 2 })

function fail(kind: AppError): IpcError {
  return new IpcError('test', kind)
}

function prepared(token: string): DeletePrepared {
  return { token, key: 'alpha', name: 'Alpha', size: 2048, expiresInSecs: 120 }
}

interface Harness {
  deps: FlowDeps
  calls: string[]
  dialogs: FlowDialog['kind'][]
  notices: FlowNotice[]
  failures: unknown[]
  stored: ModsSnapshot[]
}

interface Script {
  answers?: boolean[]
  running?: boolean[]
  trash?: (applyWhileRunning: boolean) => unknown
  remove?: (token: string, applyWhileRunning: boolean) => unknown
}

function harness(script: Script = {}): Harness {
  const answers = [...(script.answers ?? [])]
  const running = [...(script.running ?? [])]
  let tokens = 0
  const h: Harness = {
    deps: undefined as never,
    calls: [],
    dialogs: [],
    notices: [],
    failures: [],
    stored: [],
  }
  h.deps = {
    api: {
      trash: async ({ applyWhileRunning }) => {
        h.calls.push(`trash:${applyWhileRunning}`)
        const result = script.trash ? script.trash(applyWhileRunning) : { snapshot: SNAPSHOT }
        if (result instanceof Error) throw result
        return { snapshot: SNAPSHOT, key: 'alpha', items: 1, kind: 'home', ...(result as object) }
      },
      prepare: async () => {
        tokens += 1
        h.calls.push(`prepare:t${tokens}`)
        return prepared(`t${tokens}`)
      },
      remove: async ({ token, applyWhileRunning }) => {
        h.calls.push(`remove:${token}:${applyWhileRunning}`)
        const result = script.remove ? script.remove(token, applyWhileRunning) : null
        if (result instanceof Error) throw result
        return { snapshot: SNAPSHOT, key: 'alpha', leftover: (result as string | null) ?? null }
      },
    },
    ask: async (dialog) => {
      h.dialogs.push(dialog.kind)
      return answers.shift() ?? false
    },
    askRunning: async () => {
      h.dialogs.push('running' as never)
      return running.shift() ?? false
    },
    store: (snapshot) => h.stored.push(snapshot),
    notice: (notice) => h.notices.push(notice),
    fail: (error) => h.failures.push(error),
  }
  return h
}

describe('trashWithFallback', () => {
  it('trashes and never calls a delete', async () => {
    const h = harness({ answers: [true] })
    await trashWithFallback(SUBJECT, h.deps)
    expect(h.calls).toEqual(['trash:false'])
    expect(h.stored).toEqual([SNAPSHOT])
    expect(h.notices).toEqual([{ kind: 'trashDone', name: 'Alpha' }])
  })

  it('does nothing when the first confirm is cancelled', async () => {
    const h = harness({ answers: [false] })
    await trashWithFallback(SUBJECT, h.deps)
    expect(h.calls).toEqual([])
  })

  it('makes no prepare or delete call when the unavailable confirm is cancelled', async () => {
    const h = harness({
      answers: [true, false],
      trash: () => fail({ kind: 'trashUnavailable', reason: 'otherDevice', size: 10 }),
    })
    await trashWithFallback(SUBJECT, h.deps)
    expect(h.calls).toEqual(['trash:false'])
    expect(h.dialogs).toEqual(['trash', 'trashUnavailable'])
    expect(h.stored).toEqual([])
  })

  it('prepares and deletes with that token after the red confirm', async () => {
    const h = harness({
      answers: [true, true],
      trash: () => fail({ kind: 'trashUnavailable', reason: 'noUsableTrash', size: null }),
    })
    await trashWithFallback(SUBJECT, h.deps)
    expect(h.calls).toEqual(['trash:false', 'prepare:t1', 'remove:t1:false'])
    expect(h.notices).toEqual([{ kind: 'deleteDone', name: 'Alpha' }])
  })

  it('asks about a running game once, then trashes with applyWhileRunning', async () => {
    const h = harness({
      answers: [true],
      running: [true],
      trash: (running) => (running ? { snapshot: SNAPSHOT } : fail({ kind: 'gameRunning' })),
    })
    await trashWithFallback(SUBJECT, h.deps)
    expect(h.calls).toEqual(['trash:false', 'trash:true'])
  })

  it('stops when the running confirm is cancelled', async () => {
    const h = harness({
      answers: [true],
      running: [false],
      trash: () => fail({ kind: 'gameRunning' }),
    })
    await trashWithFallback(SUBJECT, h.deps)
    expect(h.calls).toEqual(['trash:false'])
    expect(h.failures).toEqual([])
  })

  it('reports any other failure', async () => {
    const error = fail({ kind: 'stale' })
    const h = harness({ answers: [true], trash: () => error })
    await trashWithFallback(SUBJECT, h.deps)
    expect(h.failures).toEqual([error])
  })
})

describe('deletePermanently', () => {
  it('needs both confirms: cancelling the first makes no prepare call', async () => {
    const h = harness({ answers: [false] })
    await deletePermanently(SUBJECT, h.deps)
    expect(h.calls).toEqual([])
  })

  it('cancelling the second confirm makes no delete call', async () => {
    const h = harness({ answers: [true, false] })
    await deletePermanently(SUBJECT, h.deps)
    expect(h.calls).toEqual(['prepare:t1'])
    expect(h.dialogs).toEqual(['deleteFirst', 'deleteFinal'])
  })

  it('deletes with the prepared token after two confirms', async () => {
    const h = harness({ answers: [true, true], remove: () => null })
    await deletePermanently(SUBJECT, h.deps)
    expect(h.calls).toEqual(['prepare:t1', 'remove:t1:false'])
    expect(h.stored).toEqual([SNAPSHOT])
  })

  it('points to Repair when something is left over', async () => {
    const h = harness({ answers: [true, true], remove: () => 'Mods/_x_tmp_1' })
    await deletePermanently(SUBJECT, h.deps)
    expect(h.notices).toContainEqual({ kind: 'deleteLeftover', path: 'Mods/_x_tmp_1' })
  })

  it('shows a notice for an invalid token and does not retry', async () => {
    const h = harness({ answers: [true, true], remove: () => fail({ kind: 'deleteTokenInvalid' }) })
    await deletePermanently(SUBJECT, h.deps)
    expect(h.calls).toEqual(['prepare:t1', 'remove:t1:false'])
    expect(h.notices).toEqual([{ kind: 'deleteExpired' }])
    expect(h.stored).toEqual([])
  })

  it('prepares a fresh token after the running confirm and deletes once more', async () => {
    const h = harness({
      answers: [true, true],
      running: [true],
      remove: (_token, running) => (running ? null : fail({ kind: 'gameRunning' })),
    })
    await deletePermanently(SUBJECT, h.deps)
    expect(h.calls).toEqual(['prepare:t1', 'remove:t1:false', 'prepare:t2', 'remove:t2:true'])
    expect(h.dialogs).toEqual(['deleteFirst', 'deleteFinal', 'running'])
  })

  it('makes no further call when the running confirm is cancelled', async () => {
    const h = harness({
      answers: [true, true],
      running: [false],
      remove: () => fail({ kind: 'gameRunning' }),
    })
    await deletePermanently(SUBJECT, h.deps)
    expect(h.calls).toEqual(['prepare:t1', 'remove:t1:false'])
    expect(h.failures).toEqual([])
  })
})
