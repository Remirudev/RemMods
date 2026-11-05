import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { useInstallQueue } from './install-queue'
import type { QueuedSource } from './types'
import { useUpdateRun } from './update-run-store'

const A: QueuedSource = { kind: 'token', id: 'src-a', name: 'a.zip', sourceKind: 'zip' }
const B: QueuedSource = { kind: 'archive', row: 'm:b', name: 'b.7z' }

describe('install queue', () => {
  beforeEach(() => useInstallQueue.getState().clear())
  afterEach(() => useUpdateRun.setState({ updating: false }))

  it('queues nothing while a self-update runs', () => {
    useUpdateRun.getState().setUpdating(true)
    useInstallQueue.getState().push([A, B])
    expect(useInstallQueue.getState().queue).toEqual([])
    expect(useInstallQueue.getState().current).toBeNull()
  })

  it('starts empty', () => {
    const state = useInstallQueue.getState()
    expect(state.queue).toEqual([])
    expect(state.current).toBeNull()
  })

  it('serves sources in the order they were pushed', () => {
    const queue = useInstallQueue.getState()
    queue.push([A])
    queue.push([B])
    expect(useInstallQueue.getState().current).toEqual(A)
    useInstallQueue.getState().shift()
    expect(useInstallQueue.getState().current).toEqual(B)
    useInstallQueue.getState().shift()
    expect(useInstallQueue.getState().current).toBeNull()
  })

  it('keeps duplicates', () => {
    useInstallQueue.getState().push([A, A])
    expect(useInstallQueue.getState().queue).toEqual([A, A])
  })

  it('counts every change of the head, so the same source twice is a new turn', () => {
    useInstallQueue.getState().push([A, A])
    const first = useInstallQueue.getState().turn
    useInstallQueue.getState().shift()
    expect(useInstallQueue.getState().turn).toBe(first + 1)
  })

  it('does not change the turn when a source joins a running queue', () => {
    useInstallQueue.getState().push([A])
    const turn = useInstallQueue.getState().turn
    useInstallQueue.getState().push([B])
    expect(useInstallQueue.getState().turn).toBe(turn)
  })

  it('clears everything', () => {
    useInstallQueue.getState().push([A, B])
    useInstallQueue.getState().clear()
    expect(useInstallQueue.getState().queue).toEqual([])
    expect(useInstallQueue.getState().current).toBeNull()
  })
})
