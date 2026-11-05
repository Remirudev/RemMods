import type { SkinSet, SkinsView } from '../../src/lib/ipc/types'
import { fail } from './state'
import type { MockState } from './types'

const NOW = '2026-11-05T12:00:00.000Z'
const SET_BYTES = 42_000_000

export function skinsView(state: MockState): SkinsView {
  return {
    account: '76561198000000001',
    available: state.scenario !== 'onboarding',
    sets: state.skinSets,
    active: state.activeSkinSet,
    interrupted: null,
    readOnly: false,
  }
}

function setOf(state: MockState, id: string): SkinSet {
  return state.skinSets.find((set) => set.id === id) ?? fail({ kind: 'stale' })
}

function addSet(state: MockState, name: string): string {
  if (state.skinSets.some((set) => set.name === name)) fail({ kind: 'skinSetNameTaken' })
  state.nextId += 1
  const id = String(state.nextId).padStart(32, '0')
  state.skinSets.push({
    id,
    name,
    notes: '',
    created: NOW,
    updated: NOW,
    screenshots: [],
    bytes: SET_BYTES,
  })
  return id
}

function addCopy(state: MockState): void {
  state.nextId += 1
  state.skinCopies.unshift({ id: `game-${state.nextId}`, taken: NOW, setName: null })
}

export function saveSkinSet(state: MockState, name: string): SkinsView {
  state.activeSkinSet = addSet(state, name)
  return skinsView(state)
}

export function keepSkinCopy(state: MockState, name: string): SkinsView {
  addSet(state, name)
  return skinsView(state)
}

export function switchSkinSet(state: MockState, id: string): SkinsView {
  state.activeSkinSet = setOf(state, id).id
  addCopy(state)
  return skinsView(state)
}

export function putBackSkinSet(state: MockState): SkinsView {
  addCopy(state)
  return skinsView(state)
}

export function editSkinSet(state: MockState, id: string, name: string, notes: string): SkinsView {
  const set = setOf(state, id)
  set.name = name
  set.notes = notes
  return skinsView(state)
}

export function deleteSkinSet(state: MockState, id: string): SkinsView {
  if (state.activeSkinSet === id) fail({ kind: 'skinSetInUse' })
  state.skinSets = state.skinSets.filter((set) => set.id !== id)
  return skinsView(state)
}

export function addSkinShot(state: MockState, id: string): SkinsView {
  const set = setOf(state, id)
  set.screenshots.push(`own-${set.screenshots.length}`)
  return skinsView(state)
}

export function removeSkinShot(state: MockState, id: string, shot: string): SkinsView {
  const set = setOf(state, id)
  set.screenshots = set.screenshots.filter((known) => known !== shot)
  return skinsView(state)
}
