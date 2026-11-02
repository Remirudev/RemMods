import { screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { ProfilesView } from '../../lib/ipc/types'
import { installIpcMock } from '../../test/ipc-mock'
import { makeProfile, makeSummary, makeView } from '../../test/profile-fixtures'
import { renderWithProviders } from '../../test/render'
import { ProfilesPanel } from './ProfilesPanel'

function renderPanel(view: ProfilesView) {
  installIpcMock(undefined, { handlers: { profiles_list: () => view } })
  renderWithProviders(<ProfilesPanel />)
}

const MAIN = makeProfile('p1', { name: 'Main', mode: 'exclusive' })
const OTHER = makeProfile('p2', {
  name: 'Old setup',
  source: { kind: 'otherManager', id: 'x', isDefault: false },
})
const SNAPSHOT = makeProfile('s1', { name: '', source: { kind: 'snapshot', of: 'p1' } })

describe('ProfilesPanel', () => {
  it('renders user profiles as cards with mode and source badges', async () => {
    renderPanel(makeView([makeSummary(MAIN, { missing: 2 }), makeSummary(OTHER)]))

    const cards = await screen.findAllByTestId('profile-card')
    expect(cards).toHaveLength(2)
    expect(within(cards[0] as HTMLElement).getByText('Main')).toBeVisible()
    expect(within(cards[0] as HTMLElement).getByText('Exclusive')).toBeVisible()
    expect(within(cards[0] as HTMLElement).getByText(/2 missing/)).toBeVisible()
    expect(within(cards[1] as HTMLElement).getByText('From another mod manager')).toBeVisible()
  })

  it('shows the unsaved-changes chip only for the last applied profile that drifted', async () => {
    renderPanel(
      makeView(
        [
          makeSummary(MAIN, { matchesCurrent: false }),
          makeSummary(OTHER, { matchesCurrent: false }),
        ],
        { lastApplied: { profileId: 'p1', at: '2026-11-01T11:00:00.000Z' } },
      ),
    )

    await screen.findAllByTestId('profile-card')
    expect(screen.getAllByText('Unsaved changes since last apply')).toHaveLength(1)
  })

  it('shows the read-only banner and disables the buttons', async () => {
    renderPanel(makeView([makeSummary(MAIN)], { readOnly: true }))

    expect(
      await screen.findByText(/comes from a newer RemMods, so profiles are read-only/),
    ).toBeVisible()
    expect(screen.getByRole('button', { name: /Save current state/ })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Apply…' })).toBeDisabled()
  })

  it('lists snapshots in a collapsed section, labelled by the profile they preceded', async () => {
    renderPanel(makeView([makeSummary(MAIN), makeSummary(SNAPSHOT)]))

    expect(await screen.findByText('Automatic snapshot (1)')).toBeVisible()
    expect(screen.getAllByTestId('profile-card')).toHaveLength(1)
  })

  it('shows the empty state without profiles', async () => {
    renderPanel(makeView([]))

    expect(await screen.findByText('No profiles yet')).toBeVisible()
  })
})
