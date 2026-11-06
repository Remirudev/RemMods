import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it } from 'vitest'
import type { ProfilesView } from '../../lib/ipc/types'
import { useGroupFilterStore } from '../../shared/group-filter-store'
import { makeModRow, makeSnapshot } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import {
  makeChange,
  makeDiff,
  makeProfile,
  makeSummary,
  makeView,
} from '../../test/profile-fixtures'
import { renderWithProviders } from '../../test/render'
import type { IpcHandler } from '../../test/types'
import { GroupsSidebar } from './GroupsSidebar'
import { useGroupPrefs } from './group-prefs-store'

const ROWS = [makeModRow('Apron'), makeModRow('Mavis', { state: 'disabled' })]
const FINE = makeProfile('p1', {
  name: 'FineMods',
  mods: [
    { key: 'apron', state: 'enabled' },
    { key: 'mavis', state: 'disabled' },
  ],
})
const BLENDER = makeProfile('p2', {
  name: 'BlenderSphere',
  mods: [
    { key: 'mavis', state: 'enabled' },
    { key: 'apron', state: 'disabled' },
  ],
})
const VIEW = makeView([
  makeSummary(FINE, { enabled: 1, disabled: 1 }),
  makeSummary(BLENDER, { enabled: 1, disabled: 1, matchesCurrent: false }),
])
const OUTCOME = {
  snapshot: makeSnapshot(ROWS, { revision: 2 }),
  changed: 2,
  snapshotId: 's1',
  skipped: [],
}

function renderSidebar(
  handlers: Record<string, IpcHandler> = {},
  view: ProfilesView = VIEW,
  rows = ROWS,
) {
  const ipc = installIpcMock(undefined, { handlers: { profiles_list: () => view, ...handlers } })
  renderWithProviders(<GroupsSidebar rows={rows} />)
  return ipc
}

const THREE = makeView(
  ['First', 'Second', 'Third'].map((name, index) =>
    makeSummary(
      makeProfile(`g${index + 1}`, { name, mods: [{ key: 'apron', state: 'enabled' }] }),
      { enabled: 1, matchesCurrent: false },
    ),
  ),
)

function renderThree(handlers: Record<string, IpcHandler> = {}) {
  return renderSidebar(handlers, THREE)
}

async function threeNames() {
  const name = (title: string) =>
    screen.findByRole('button', { name: `Show only the mods of “${title}”` })
  return [await name('First'), await name('Second'), await name('Third')] as const
}

beforeEach(() => {
  useGroupPrefs.setState({ askBeforeSwitch: true, collapsed: false })
  useGroupFilterStore.setState({ groups: [], picked: [], anchor: null })
})

describe('GroupsSidebar', () => {
  it('lists the groups by name with a switch that is on while the mods match', async () => {
    renderSidebar()

    expect(await screen.findByRole('switch', { name: 'Switch off “FineMods”' })).toBeChecked()
    expect(screen.getByRole('switch', { name: 'Switch on “BlenderSphere”' })).not.toBeChecked()
    expect(screen.getAllByTestId('group-row').map((row) => row.textContent)).toEqual([
      expect.stringContaining('BlenderSphere'),
      expect.stringContaining('FineMods'),
    ])
  })

  it('shows the change list first, in the direction of the switch', async () => {
    const user = userEvent.setup()
    const ipc = renderSidebar({
      profiles_diff: () =>
        makeDiff({ changes: [makeChange('apron', false)], fingerprint: '-apron\n' }),
    })

    await user.click(await screen.findByRole('switch', { name: 'Switch off “FineMods”' }))

    expect(await screen.findByRole('dialog', { name: 'Switch off “FineMods”' })).toBeVisible()
    expect(ipc.callsOf('profiles_diff')[0]?.payload).toEqual({ ids: ['p1'], direction: 'off' })
    expect(ipc.callsOf('profiles_apply')).toHaveLength(0)
  })

  it('switches at once with the diff’s fingerprint when asking first is off', async () => {
    useGroupPrefs.setState({ askBeforeSwitch: false })
    const user = userEvent.setup()
    const ipc = renderSidebar({
      profiles_diff: () =>
        makeDiff({
          changes: [makeChange('mavis', true), makeChange('apron', false)],
          fingerprint: '+mavis\n-apron\n',
        }),
      profiles_apply: () => OUTCOME,
    })

    await user.click(await screen.findByRole('switch', { name: 'Switch on “BlenderSphere”' }))

    await waitFor(() => expect(ipc.callsOf('profiles_apply')).toHaveLength(1))
    expect(ipc.callsOf('profiles_apply')[0]?.payload).toEqual({
      request: {
        profileIds: ['p2'],
        fingerprint: '+mavis\n-apron\n',
        applyWhileRunning: false,
        direction: 'on',
      },
    })
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('shows the change list instead when the disk changed before a direct switch', async () => {
    useGroupPrefs.setState({ askBeforeSwitch: false })
    const user = userEvent.setup()
    renderSidebar({
      profiles_diff: () => makeDiff(),
      profiles_apply: () => {
        throw { kind: 'stale' }
      },
    })

    await user.click(await screen.findByRole('switch', { name: 'Switch on “BlenderSphere”' }))

    expect(await screen.findByRole('dialog', { name: 'Switch on “BlenderSphere”' })).toBeVisible()
  })

  it('shows only a group’s mods from its name and all mods again on a second click', async () => {
    const user = userEvent.setup()
    renderSidebar()
    const name = await screen.findByRole('button', { name: 'Show only the mods of “FineMods”' })

    await user.click(name)
    expect(useGroupFilterStore.getState().picked).toEqual(['p1'])
    expect(useGroupFilterStore.getState().groups).toContainEqual({
      id: 'p1',
      name: 'FineMods',
      keys: ['apron'],
    })
    expect(name).toHaveAttribute('aria-pressed', 'true')

    await user.click(name)
    expect(useGroupFilterStore.getState().picked).toEqual([])
  })

  it('drops a group that is gone from the list filter', async () => {
    useGroupFilterStore.setState({ picked: ['gone', 'p1'], anchor: 'gone' })
    renderSidebar()

    await screen.findByRole('switch', { name: 'Switch off “FineMods”' })
    await waitFor(() => expect(useGroupFilterStore.getState().picked).toEqual(['p1']))
    expect(useGroupFilterStore.getState().anchor).toBeNull()
  })

  it('picks several with Ctrl and a range with Shift, like a file list', async () => {
    const user = userEvent.setup()
    renderThree()
    const [first, second, third] = await threeNames()

    await user.click(first)
    await user.keyboard('{Control>}')
    await user.click(third)
    await user.keyboard('{/Control}')
    expect(useGroupFilterStore.getState().picked).toEqual(['g1', 'g3'])
    expect(second).toHaveAttribute('aria-pressed', 'false')

    await user.keyboard('{Control>}')
    await user.click(first)
    await user.keyboard('{/Control}')
    expect(useGroupFilterStore.getState().picked).toEqual(['g3'])

    await user.keyboard('{Shift>}')
    await user.click(third)
    await user.keyboard('{/Shift}')
    expect(useGroupFilterStore.getState().picked).toEqual(['g1', 'g2', 'g3'])
    expect(screen.getByTestId('group-selected')).toHaveTextContent('3 selected')
  })

  it('marks everything between two Shift clicks', async () => {
    const user = userEvent.setup()
    renderThree()
    const [first, , third] = await threeNames()

    await user.keyboard('{Shift>}')
    await user.click(first)
    await user.click(third)
    await user.keyboard('{/Shift}')

    expect(useGroupFilterStore.getState().picked).toEqual(['g1', 'g2', 'g3'])
  })

  it('goes back to one group on a plain click', async () => {
    const user = userEvent.setup()
    renderThree()
    const [first, second, third] = await threeNames()

    await user.click(first)
    await user.keyboard('{Shift>}')
    await user.click(third)
    await user.keyboard('{/Shift}')
    await user.click(second)

    expect(useGroupFilterStore.getState().picked).toEqual(['g2'])
    expect(screen.queryByTestId('group-selected')).not.toBeInTheDocument()
  })

  it('moves all picked groups with one switch, in its direction', async () => {
    const user = userEvent.setup()
    const ipc = renderThree({ profiles_diff: () => makeDiff() })
    const [first, second] = await threeNames()

    await user.click(first)
    await user.keyboard('{Control>}')
    await user.click(second)
    await user.keyboard('{/Control}')
    await user.click(screen.getByRole('switch', { name: 'Switch on “Second”' }))

    expect(await screen.findByRole('dialog', { name: 'Switch on 2 groups' })).toBeVisible()
    expect(ipc.callsOf('profiles_diff')[0]?.payload).toEqual({
      ids: ['g1', 'g2'],
      direction: 'on',
    })
  })

  it('moves only its own group from a switch outside the selection', async () => {
    const user = userEvent.setup()
    const ipc = renderThree({ profiles_diff: () => makeDiff() })
    const [first, second] = await threeNames()

    await user.click(first)
    await user.keyboard('{Control>}')
    await user.click(second)
    await user.keyboard('{/Control}')
    await user.click(screen.getByRole('switch', { name: 'Switch on “Third”' }))

    expect(await screen.findByRole('dialog', { name: 'Switch on “Third”' })).toBeVisible()
    expect(ipc.callsOf('profiles_diff')[0]?.payload).toEqual({ ids: ['g3'], direction: 'on' })
  })

  it('switches every group on or off at once', async () => {
    const user = userEvent.setup()
    const ipc = renderThree({ profiles_diff: () => makeDiff() })

    await user.click(await screen.findByRole('button', { name: 'Switch off every group' }))

    expect(await screen.findByRole('dialog', { name: 'Switch off 3 groups' })).toBeVisible()
    expect(ipc.callsOf('profiles_diff')[0]?.payload).toEqual({
      ids: ['g1', 'g2', 'g3'],
      direction: 'off',
    })
  })

  it('still means every group while one group filters the list', async () => {
    const user = userEvent.setup()
    const ipc = renderThree({ profiles_diff: () => makeDiff() })
    const [first] = await threeNames()

    await user.click(first)
    await user.click(screen.getByRole('button', { name: 'Switch on every group' }))

    expect(await screen.findByRole('dialog', { name: 'Switch on 3 groups' })).toBeVisible()
    expect(ipc.callsOf('profiles_diff')[0]?.payload).toEqual({
      ids: ['g1', 'g2', 'g3'],
      direction: 'on',
    })
  })

  it('switches only the selection while several groups are picked, and forgets it on request', async () => {
    const user = userEvent.setup()
    const ipc = renderThree({ profiles_diff: () => makeDiff() })
    const [first, , third] = await threeNames()

    await user.click(first)
    await user.keyboard('{Control>}')
    await user.click(third)
    await user.keyboard('{/Control}')
    await user.click(screen.getByRole('button', { name: 'Switch on the 2 selected groups' }))

    expect(await screen.findByRole('dialog', { name: 'Switch on 2 groups' })).toBeVisible()
    expect(ipc.callsOf('profiles_diff')[0]?.payload).toEqual({
      ids: ['g1', 'g3'],
      direction: 'on',
    })
    await user.click(screen.getByRole('button', { name: 'Cancel' }))

    await user.click(screen.getByRole('button', { name: 'Clear the selection' }))
    expect(useGroupFilterStore.getState().picked).toEqual([])
    expect(screen.getByRole('button', { name: 'Switch on every group' })).toBeVisible()
  })

  it('applies several groups at once in one batch when asking first is off', async () => {
    useGroupPrefs.setState({ askBeforeSwitch: false })
    const user = userEvent.setup()
    const ipc = renderThree({
      profiles_diff: () => makeDiff({ fingerprint: '+apron\n' }),
      profiles_apply: () => OUTCOME,
    })

    await user.click(await screen.findByRole('button', { name: 'Switch on every group' }))

    await waitFor(() => expect(ipc.callsOf('profiles_apply')).toHaveLength(1))
    expect(ipc.callsOf('profiles_apply')[0]?.payload).toEqual({
      request: {
        profileIds: ['g1', 'g2', 'g3'],
        fingerprint: '+apron\n',
        applyWhileRunning: false,
        direction: 'on',
      },
    })
  })

  it('needs no All on or All off for a single group', async () => {
    renderSidebar({}, makeView([makeSummary(FINE)]))

    await screen.findByRole('switch', { name: 'Switch off “FineMods”' })
    expect(screen.queryByTestId('group-bulk')).not.toBeInTheDocument()
  })

  it('locks All on and All off for a read-only store', async () => {
    renderSidebar({}, makeView(VIEW.profiles, { readOnly: true }))

    expect(await screen.findByRole('button', { name: 'Switch on every group' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Switch off every group' })).toBeDisabled()
  })

  it('warns when a group switches on two mods that replace the same files', async () => {
    const rows = [
      makeModRow('Apron', {
        conflictsWith: [{ modRef: 'm:mavis', key: 'mavis', name: 'Mavis', paks: ['x.pak'] }],
      }),
      makeModRow('Mavis'),
    ]
    const both = makeProfile('p3', {
      name: 'Both',
      mods: [
        { key: 'apron', state: 'enabled' },
        { key: 'mavis', state: 'enabled' },
      ],
    })
    renderSidebar({}, makeView([makeSummary(both)]), rows)

    expect(await screen.findByRole('img', { name: /1 pair of mods in this group/ })).toBeVisible()
  })

  it('offers a first group when there is none', async () => {
    renderSidebar({}, makeView([]))

    expect(await screen.findByText(/No groups yet/)).toBeVisible()
  })

  it('locks switching and editing for a read-only store', async () => {
    renderSidebar({}, makeView(VIEW.profiles, { readOnly: true }))

    expect(await screen.findByRole('switch', { name: 'Switch off “FineMods”' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'New group' })).toBeDisabled()
    expect(screen.getByText(/groups are read-only here/)).toBeVisible()
  })

  it('folds to round buttons that still switch', async () => {
    const user = userEvent.setup()
    renderSidebar({ profiles_diff: () => makeDiff() })

    await user.click(await screen.findByRole('button', { name: 'Hide groups' }))

    expect(useGroupPrefs.getState().collapsed).toBe(true)
    const chips = await screen.findAllByTestId('group-chip')
    expect(chips.map((chip) => chip.textContent)).toEqual(['BL', 'FI'])
    await user.click(screen.getByRole('button', { name: 'Switch on “BlenderSphere”' }))
    expect(await screen.findByRole('dialog', { name: 'Switch on “BlenderSphere”' })).toBeVisible()
  })
})
