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

beforeEach(() => {
  useGroupPrefs.setState({ askBeforeSwitch: true, collapsed: false })
  useGroupFilterStore.setState({ group: null })
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
    expect(ipc.callsOf('profiles_diff')[0]?.payload).toEqual({ id: 'p1', direction: 'off' })
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
        profileId: 'p2',
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
    expect(useGroupFilterStore.getState().group).toEqual({
      id: 'p1',
      name: 'FineMods',
      keys: ['apron'],
    })
    expect(name).toHaveAttribute('aria-pressed', 'true')

    await user.click(name)
    expect(useGroupFilterStore.getState().group).toBeNull()
  })

  it('ends the list filter when its group is gone', async () => {
    useGroupFilterStore.setState({ group: { id: 'gone', name: 'Old', keys: ['x'] } })
    renderSidebar()

    await screen.findByRole('switch', { name: 'Switch off “FineMods”' })
    await waitFor(() => expect(useGroupFilterStore.getState().group).toBeNull())
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
