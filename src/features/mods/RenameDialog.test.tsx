import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { makeSnapshot } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import type { IpcHandler } from '../../test/types'
import { foldName } from './name-check'
import { RenameDialog } from './RenameDialog'

const TARGET = { modRef: 'm:alpha', key: 'alpha' }

const OUTCOME = {
  snapshot: makeSnapshot([], { revision: 2 }),
  oldKey: 'alpha',
  newKey: 'beta',
  newRef: null,
  rekey: { profiles: 0, meta: false, skippedAmbiguous: false, failed: false },
}

function reject(error: object): IpcHandler {
  return () => {
    throw error
  }
}

function renderDialog(handler: IpcHandler = () => OUTCOME) {
  const onClose = vi.fn()
  const ipc = installIpcMock(undefined, { handlers: { mods_rename: handler } })
  renderWithProviders(
    <RenameDialog
      target={TARGET}
      currentName="Alpha"
      initialName="Alpha"
      siblings={new Set([foldName('Taken')])}
      otherManager={false}
      onClose={onClose}
    />,
  )
  return { ipc, onClose }
}

function submitButton() {
  return screen.getByRole('button', { name: 'Rename' })
}

describe('RenameDialog', () => {
  it('disables Submit for the unchanged name and for invalid ones', async () => {
    const user = userEvent.setup()
    renderDialog()
    const input = screen.getByRole('textbox', { name: 'New name' })

    expect(submitButton()).toBeDisabled()
    await user.clear(input)
    expect(submitButton()).toBeDisabled()
    await user.type(input, 'Bad:Name')
    expect(submitButton()).toBeDisabled()
    expect(screen.getByText(/cannot contain/)).toBeVisible()
    await user.clear(input)
    await user.type(input, 'taken')
    expect(screen.getByText('Another mod or folder already has this name.')).toBeVisible()
    expect(submitButton()).toBeDisabled()
    await user.clear(input)
    await user.type(input, 'Beta')
    expect(submitButton()).toBeEnabled()
  })

  it('shows an InvalidName from the core inline and keeps the dialog open', async () => {
    const user = userEvent.setup()
    const { ipc, onClose } = renderDialog(reject({ kind: 'invalidName', issue: 'temp_name' }))
    const input = screen.getByRole('textbox', { name: 'New name' })

    await user.clear(input)
    await user.type(input, 'Beta{Enter}')

    expect(
      await screen.findByText('This name looks like an unfinished install folder.'),
    ).toBeVisible()
    expect(ipc.callsOf('mods_rename')).toHaveLength(1)
    expect(onClose).not.toHaveBeenCalled()
  })

  it('shows TargetExists inline', async () => {
    const user = userEvent.setup()
    renderDialog(reject({ kind: 'targetExists' }))
    await user.clear(screen.getByRole('textbox', { name: 'New name' }))
    await user.type(screen.getByRole('textbox', { name: 'New name' }), 'Beta{Enter}')

    expect(await screen.findByText('Another mod or folder already has this name.')).toBeVisible()
  })

  it('submits once on Enter and closes on success', async () => {
    const user = userEvent.setup()
    const { ipc, onClose } = renderDialog()
    const input = screen.getByRole('textbox', { name: 'New name' })

    await user.clear(input)
    await user.type(input, 'Beta{Enter}{Enter}')

    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1))
    expect(ipc.callsOf('mods_rename')).toHaveLength(1)
    expect(ipc.callsOf('mods_rename')[0]?.payload).toEqual({
      request: { target: TARGET, newName: 'Beta', applyWhileRunning: false },
    })
  })
})
