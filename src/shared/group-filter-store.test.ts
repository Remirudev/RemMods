import { beforeEach, describe, expect, it } from 'vitest'
import { pickedKeys, useGroupFilterStore } from './group-filter-store'

const GROUPS = [
  { id: 'a', name: 'A', keys: ['one', 'two'] },
  { id: 'b', name: 'B', keys: ['two', 'three'] },
  { id: 'c', name: 'C', keys: [] },
]

describe('pickedKeys', () => {
  it('means every mod while no group is picked', () => {
    expect(pickedKeys(GROUPS, [])).toBeNull()
  })

  it('takes the mods of all picked groups, each once', () => {
    expect(pickedKeys(GROUPS, ['a', 'b'])).toEqual(['one', 'two', 'three'])
  })

  it('shows nothing for a picked group that switches nothing on', () => {
    expect(pickedKeys(GROUPS, ['c'])).toEqual([])
  })
})

describe('useGroupFilterStore', () => {
  beforeEach(() => {
    useGroupFilterStore.setState({ groups: [], picked: [], anchor: null })
  })

  it('keeps the picks of groups that stay', () => {
    useGroupFilterStore.setState({ picked: ['a', 'b'], anchor: 'b' })
    useGroupFilterStore.getState().setGroups(GROUPS)

    expect(useGroupFilterStore.getState().picked).toEqual(['a', 'b'])
    expect(useGroupFilterStore.getState().anchor).toBe('b')
  })

  it('drops the picks and the anchor of groups that are gone', () => {
    useGroupFilterStore.setState({ picked: ['a', 'gone'], anchor: 'gone' })
    useGroupFilterStore.getState().setGroups(GROUPS)

    expect(useGroupFilterStore.getState().picked).toEqual(['a'])
    expect(useGroupFilterStore.getState().anchor).toBeNull()
  })
})
