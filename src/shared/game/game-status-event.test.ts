import { describe, expect, it } from 'vitest'
import type { GameStatus } from '../../lib/ipc/types'
import { applyGameStatusEvent } from './game-status-event'

const STATUS: GameStatus = {
  location: {
    kind: 'ready',
    install: {
      root: '/games/steamapps/common/TEKKEN 8',
      paks: '/games/steamapps/common/TEKKEN 8/Polaris/Content/Paks',
      source: 'steam',
      steamKind: 'native',
      buildId: '25221600',
      updateTracking: 'available',
    },
  },
  running: 'unknown',
  session: 0,
  otherManager: false,
  update: null,
  candidates: [],
  notes: [],
}

describe('applyGameStatusEvent', () => {
  it('replaces the running state, session, other-manager flag and update notice', () => {
    const next = applyGameStatusEvent(STATUS, {
      running: 'running',
      session: 2,
      otherManager: true,
      update: { from: '1', to: '25221600' },
    })
    expect(next).toEqual({
      ...STATUS,
      running: 'running',
      session: 2,
      otherManager: true,
      update: { from: '1', to: '25221600' },
    })
  })

  it('keeps the location and candidates as they are', () => {
    const next = applyGameStatusEvent(STATUS, {
      running: 'notRunning',
      session: 0,
      otherManager: false,
      update: null,
    })
    expect(next?.location).toBe(STATUS.location)
    expect(next?.candidates).toBe(STATUS.candidates)
  })

  it('does nothing without a cached status', () => {
    const next = applyGameStatusEvent(undefined, {
      running: 'running',
      session: 1,
      otherManager: false,
      update: null,
    })
    expect(next).toBeUndefined()
  })
})
