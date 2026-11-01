import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { makeSnapshot } from '../../test/fixtures'
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
import { ProfileDiffModal } from './ProfileDiffModal'

const PROFILE = makeProfile('p1', { name: 'Main' })
const OUTCOME = {
  snapshot: makeSnapshot([], { revision: 2 }),
  changed: 2,
  snapshotId: 's1',
  skipped: [],
}

function renderModal(handlers: Record<string, IpcHandler>) {
  const onClose = vi.fn()
  const ipc = installIpcMock(undefined, {
    handlers: { profiles_list: () => makeView([makeSummary(PROFILE)]), ...handlers },
  })
  renderWithProviders(<ProfileDiffModal profile={PROFILE} onClose={onClose} />)
  return { ipc, onClose }
}

describe('ProfileDiffModal', () => {
  it('disables Apply and says so when nothing changes', async () => {
    renderModal({ profiles_diff: () => makeDiff({ changes: [], fingerprint: '' }) })

    expect(await screen.findByText('Nothing to change.')).toBeVisible()
    expect(screen.getByRole('button', { name: 'Apply' })).toBeDisabled()
  })

  it('sends the fingerprint it shows', async () => {
    const user = userEvent.setup()
    const { ipc, onClose } = renderModal({
      profiles_diff: () => makeDiff({ fingerprint: '+alpha\n-beta\n' }),
      profiles_apply: () => OUTCOME,
    })

    await user.click(await screen.findByRole('button', { name: 'Apply' }))

    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1))
    expect(ipc.callsOf('profiles_apply')[0]?.payload).toEqual({
      request: { profileId: 'p1', fingerprint: '+alpha\n-beta\n', applyWhileRunning: false },
    })
  })

  it('reloads the diff on Stale and does not apply again by itself', async () => {
    const user = userEvent.setup()
    let diffs = 0
    const { ipc, onClose } = renderModal({
      profiles_diff: () => {
        diffs += 1
        return diffs === 1
          ? makeDiff({ fingerprint: 'old' })
          : makeDiff({ changes: [makeChange('gamma', true)], fingerprint: '+gamma\n' })
      },
      profiles_apply: () => {
        throw { kind: 'stale' }
      },
    })

    await user.click(await screen.findByRole('button', { name: 'Apply' }))

    expect(await screen.findByText(/The mods changed on disk/)).toBeVisible()
    expect(await screen.findByText('gamma')).toBeVisible()
    expect(ipc.callsOf('profiles_apply')).toHaveLength(1)
    expect(ipc.callsOf('profiles_diff').length).toBeGreaterThanOrEqual(2)
    expect(onClose).not.toHaveBeenCalled()
  })

  it('disables Apply for a read-only store', async () => {
    renderModal({
      profiles_diff: () => makeDiff(),
      profiles_list: () => makeView([makeSummary(PROFILE)], { readOnly: true }),
    })

    await screen.findByText('alpha')
    await waitFor(() => expect(screen.getByRole('button', { name: 'Apply' })).toBeDisabled())
  })
})
