import { act, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { ProfilesView } from '../../lib/ipc/types'
import { registerProfileApplyOpener } from '../../shared/profile-quick-apply'
import { queryKeys } from '../../shared/query-keys'
import { installIpcMock } from '../../test/ipc-mock'
import { makeProfile, makeSummary, makeView } from '../../test/profile-fixtures'
import { renderWithProviders } from '../../test/render'
import { QuickApplyMenu } from './QuickApplyMenu'

const VIEW = makeView(
  [
    makeSummary(makeProfile('b', { name: 'Set B', mode: 'exclusive' })),
    makeSummary(makeProfile('a', { name: 'Set A' })),
    makeSummary(makeProfile('s', { name: '', source: { kind: 'snapshot', of: 'a' } })),
  ],
  { lastApplied: { profileId: 'a', at: '2026-11-01T11:00:00.000Z' } },
)

function renderMenu(view: ProfilesView) {
  const ipc = installIpcMock(undefined, { handlers: { profiles_list: () => view } })
  renderWithProviders(<QuickApplyMenu />)
  return ipc
}

describe('QuickApplyMenu', () => {
  let unregister: () => void = () => undefined

  afterEach(() => unregister())

  it('lists the profiles and opens the apply for the clicked one', async () => {
    const user = userEvent.setup()
    const open = vi.fn()
    unregister = registerProfileApplyOpener(open)
    renderMenu(VIEW)

    await user.click(screen.getByRole('button', { name: 'Apply profile' }))
    const items = await screen.findAllByRole('menuitem')
    expect(items.map((item) => item.textContent)).toEqual(['Set AAdditive', 'Set BExclusive'])
    expect(screen.getByLabelText('Last applied')).toBeInTheDocument()

    await user.click(screen.getByRole('menuitem', { name: /Set B/ }))
    expect(open).toHaveBeenCalledWith('b')
  })

  it('disables the items while profiles are read-only', async () => {
    const user = userEvent.setup()
    const open = vi.fn()
    unregister = registerProfileApplyOpener(open)
    renderMenu({ ...VIEW, readOnly: true })

    await user.click(screen.getByRole('button', { name: 'Apply profile' }))
    expect(await screen.findByText(/Profiles cannot be saved right now/)).toBeInTheDocument()
    for (const item of screen.getAllByRole('menuitem')) expect(item).toBeDisabled()
  })

  it('says so when there is nothing to apply', async () => {
    const user = userEvent.setup()
    renderMenu(makeView([]))

    await user.click(screen.getByRole('button', { name: 'Apply profile' }))
    expect(await screen.findByRole('menuitem', { name: 'No saved profiles yet' })).toBeDisabled()
  })

  it('says the profiles could not be loaded when the list fails', async () => {
    const user = userEvent.setup()
    installIpcMock(undefined, {
      handlers: {
        profiles_list: () => {
          throw { kind: 'internal' }
        },
      },
    })
    renderWithProviders(<QuickApplyMenu />)

    await user.click(screen.getByRole('button', { name: 'Apply profile' }))
    expect(
      await screen.findByRole('menuitem', { name: 'Profiles could not be loaded.' }),
    ).toBeDisabled()
    expect(screen.queryByText('No saved profiles yet')).toBeNull()
  })

  it('keeps the cached profiles when a later refetch fails', async () => {
    const user = userEvent.setup()
    unregister = registerProfileApplyOpener(vi.fn())
    let calls = 0
    installIpcMock(undefined, {
      handlers: {
        profiles_list: () => {
          calls += 1
          if (calls > 1) throw { kind: 'internal' }
          return VIEW
        },
      },
    })
    const { queryClient } = renderWithProviders(<QuickApplyMenu />)

    await user.click(screen.getByRole('button', { name: 'Apply profile' }))
    expect(await screen.findAllByRole('menuitem')).toHaveLength(2)
    await act(() => queryClient.refetchQueries({ queryKey: queryKeys.profiles }))
    await waitFor(() => expect(queryClient.getQueryState(queryKeys.profiles)?.status).toBe('error'))
    expect(screen.getAllByRole('menuitem').map((item) => item.textContent)).toEqual([
      'Set AAdditive',
      'Set BExclusive',
    ])
    expect(screen.queryByText('Profiles could not be loaded.')).toBeNull()
  })
})
