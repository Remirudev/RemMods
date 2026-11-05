import { modals } from '@mantine/modals'
import { Notifications, notifications } from '@mantine/notifications'
import { emit } from '@tauri-apps/api/event'
import { act, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it } from 'vitest'
import { RenameDialog } from '../features/mods/RenameDialog'
import type { FolderBusy } from '../lib/ipc/types'
import { makeFolderBusy } from '../test/fixtures'
import { installIpcMock } from '../test/ipc-mock'
import { renderWithProviders } from '../test/render'
import { useFolderBusy } from './use-folder-busy'

const EVENT = 'folder-busy'
const TITLE = 'Waiting for a folder in use'

function Host() {
  useFolderBusy()
  return (
    <>
      <button type="button">Grid</button>
      <Notifications />
    </>
  )
}

function mount(handlers = {}) {
  const ipc = installIpcMock(undefined, { handlers })
  renderWithProviders(<Host />)
  return ipc
}

// The listener registers asynchronously; the first event that shows the toast proves it is on.
async function emitUntilShown(payload: FolderBusy) {
  await waitFor(async () => {
    await emit(EVENT, payload)
    expect(screen.getByTestId('folder-busy')).toBeInTheDocument()
  })
}

describe('the folder-busy toast', () => {
  beforeEach(() => {
    notifications.clean()
  })

  it('shows Cancel with the focus and cancels the wait by its id', async () => {
    const user = userEvent.setup()
    const ipc = mount({ mods_busy_cancel: () => true })
    await emitUntilShown(makeFolderBusy({ waitId: 7 }))

    expect(screen.getAllByText(TITLE).length).toBeGreaterThan(0)
    expect(screen.getByText(/“Mods\/Alpha” is in use by another program/)).toBeInTheDocument()
    const cancel = screen.getByRole('button', { name: 'Cancel' })
    expect(document.activeElement).toBe(cancel)

    await user.keyboard('{Enter}')
    await waitFor(() => expect(ipc.callsOf('mods_busy_cancel')).toHaveLength(1))
    expect(ipc.callsOf('mods_busy_cancel')[0]?.payload).toEqual({ waitId: 7 })
    expect(await screen.findByRole('button', { name: 'Cancelling…' })).toBeDisabled()

    await act(() => emit(EVENT, makeFolderBusy({ waitId: 7, phase: 'cancelled' })))
    await waitFor(() => expect(screen.queryByTestId('folder-busy')).toBeNull())
  })

  it('shows a rollback wait without a button and hides it when done', async () => {
    mount()
    await emitUntilShown(makeFolderBusy({ waitId: 8, cancellable: false }))

    expect(screen.getByText(/RemMods is putting “Mods\/Alpha” back/)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Cancel' })).toBeNull()

    await act(() => emit(EVENT, makeFolderBusy({ waitId: 8, phase: 'done', cancellable: false })))
    await waitFor(() => expect(screen.queryByTestId('folder-busy')).toBeNull())
  })

  it('keeps the focus where it is on later updates', async () => {
    mount()
    await emitUntilShown(makeFolderBusy({ waitId: 9 }))
    const cancel = screen.getByRole('button', { name: 'Cancel' })
    cancel.blur()

    await act(() => emit(EVENT, makeFolderBusy({ waitId: 9, elapsedMs: 3200 })))
    const elapsed = await screen.findByText('Waiting for 3 s')
    expect(elapsed).toHaveAttribute('aria-hidden', 'true')
    expect(document.activeElement).not.toBe(cancel)
  })

  it('gives the focus back when the wait ends', async () => {
    mount()
    const grid = screen.getByRole('button', { name: 'Grid' })
    act(() => grid.focus())
    await emitUntilShown(makeFolderBusy({ waitId: 10 }))
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Cancel' }))

    await act(() => emit(EVENT, makeFolderBusy({ waitId: 10, phase: 'done' })))
    await waitFor(() => expect(screen.queryByTestId('folder-busy')).toBeNull())
    expect(document.activeElement).toBe(grid)
  })
})

describe('a dialog while its file change runs', () => {
  beforeEach(() => {
    notifications.clean()
  })

  it('disables Cancel and ignores Escape', async () => {
    const user = userEvent.setup()
    installIpcMock(undefined, { handlers: { mods_rename: () => new Promise(() => undefined) } })
    renderWithProviders(<Notifications />)
    const modalId = 'rename-test'
    act(() => {
      modals.open({
        modalId,
        title: 'Rename mod',
        children: (
          <RenameDialog
            target={{ modRef: 'm:alpha', key: 'alpha' }}
            currentName="Alpha"
            initialName="Alpha"
            siblings={new Set()}
            onClose={() => modals.close(modalId)}
            modalId={modalId}
          />
        ),
      })
    })

    const input = await screen.findByRole('textbox', { name: 'New name' })
    await user.clear(input)
    await user.type(input, 'Bravo{Enter}')

    await waitFor(() => expect(screen.getByRole('button', { name: 'Cancel' })).toBeDisabled())
    await user.keyboard('{Escape}')
    expect(screen.getByRole('dialog')).toBeInTheDocument()
    act(() => modals.closeAll())
  })
})
