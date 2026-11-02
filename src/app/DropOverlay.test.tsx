import { Notifications, notifications } from '@mantine/notifications'
import { emit } from '@tauri-apps/api/event'
import { screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import type { InstallDrop } from '../lib/ipc/types'
import { useInstallQueue } from '../shared/install-queue'
import { useUpdateRun } from '../shared/update-run-store'
import { installIpcMock } from '../test/ipc-mock'
import { renderWithProviders } from '../test/render'
import { DropOverlay } from './DropOverlay'

const EVENT = 'install-drop'
const EMPTY = { count: 0, sources: [], rejected: [] }
const ENTER: InstallDrop = { phase: 'enter', ...EMPTY, count: 2 }
const LEAVE: InstallDrop = { phase: 'leave', ...EMPTY }

function mountOverlay() {
  installIpcMock()
  renderWithProviders(
    <>
      <Notifications />
      <DropOverlay />
    </>,
  )
}

// The listener registers asynchronously; the first enter that shows the overlay proves it is on.
async function waitUntilListening() {
  await waitFor(async () => {
    await emit(EVENT, ENTER)
    expect(screen.getByTestId('drop-overlay')).toBeInTheDocument()
  })
  await emit(EVENT, LEAVE)
  await waitFor(() => expect(screen.queryByTestId('drop-overlay')).not.toBeInTheDocument())
}

describe('DropOverlay', () => {
  beforeEach(() => {
    useInstallQueue.getState().clear()
    notifications.clean()
  })
  afterEach(() => useUpdateRun.setState({ updating: false }))

  it('shows the hint with the count while files are dragged over and hides on leave', async () => {
    mountOverlay()
    expect(screen.queryByTestId('drop-overlay')).not.toBeInTheDocument()
    await waitFor(async () => {
      await emit(EVENT, ENTER)
      expect(screen.getByTestId('drop-overlay')).toBeInTheDocument()
    })
    expect(screen.getByText('Drop archives or folders to install')).toBeVisible()
    expect(screen.getByText('2 items')).toBeVisible()
    await emit(EVENT, LEAVE)
    await waitFor(() => expect(screen.queryByTestId('drop-overlay')).not.toBeInTheDocument())
  })

  it('hides on drop and queues the dropped sources in order', async () => {
    mountOverlay()
    await waitUntilListening()
    await emit(EVENT, ENTER)
    await emit(EVENT, {
      phase: 'dropped',
      count: 2,
      sources: [
        { id: 'src-1', name: 'one.zip', kind: 'zip' },
        { id: 'src-2', name: 'Two', kind: 'folder' },
      ],
      rejected: [],
    } satisfies InstallDrop)
    await waitFor(() => expect(useInstallQueue.getState().queue).toHaveLength(2))
    expect(screen.queryByTestId('drop-overlay')).not.toBeInTheDocument()
    expect(useInstallQueue.getState().queue).toEqual([
      { kind: 'token', id: 'src-1', name: 'one.zip', sourceKind: 'zip' },
      { kind: 'token', id: 'src-2', name: 'Two', sourceKind: 'folder' },
    ])
  })

  it('lists every rejected item with its reason in one toast', async () => {
    mountOverlay()
    await waitUntilListening()
    await emit(EVENT, {
      phase: 'dropped',
      count: 2,
      sources: [],
      rejected: [
        { name: 'notes.txt', reason: 'unsupportedType' },
        { name: 'remote.zip', reason: 'missing' },
      ],
    } satisfies InstallDrop)
    expect(await screen.findByText('Some items cannot be installed')).toBeVisible()
    expect(screen.getAllByText(/notes\.txt: Not a zip/)).toHaveLength(1)
    expect(screen.getByText(/remote\.zip: .*copy it to a local folder first/s)).toBeVisible()
    expect(useInstallQueue.getState().queue).toEqual([])
  })

  it('ignores drags and drops while a self-update runs', async () => {
    mountOverlay()
    await waitUntilListening()
    useUpdateRun.getState().setUpdating(true)
    await emit(EVENT, ENTER)
    expect(screen.queryByTestId('drop-overlay')).not.toBeInTheDocument()
    await emit(EVENT, {
      phase: 'dropped',
      count: 1,
      sources: [{ id: 'src-1', name: 'one.zip', kind: 'zip' }],
      rejected: [{ name: 'notes.txt', reason: 'unsupportedType' }],
    } satisfies InstallDrop)
    // The mocked emit calls the listener before it resolves; the listener is still on.
    useUpdateRun.getState().setUpdating(false)
    await emit(EVENT, ENTER)
    await waitFor(() => expect(screen.getByTestId('drop-overlay')).toBeInTheDocument())
    expect(useInstallQueue.getState().queue).toEqual([])
    expect(screen.queryByText('Some items cannot be installed')).not.toBeInTheDocument()
  })
})
