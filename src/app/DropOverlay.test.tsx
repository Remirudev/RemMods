import { Notifications, notifications } from '@mantine/notifications'
import { emit } from '@tauri-apps/api/event'
import { screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import type { InstallDrop } from '../lib/ipc/types'
import { useInstallQueue } from '../shared/install-queue'
import { usePictureDropStore } from '../shared/picture-drop-store'
import { useUpdateRun } from '../shared/update-run-store'
import { installIpcMock } from '../test/ipc-mock'
import { renderWithProviders } from '../test/render'
import { DropOverlay } from './DropOverlay'

const EVENT = 'install-drop'
const EMPTY = { count: 0, images: 0, sources: [], rejected: [] }
const ENTER: InstallDrop = { phase: 'enter', ...EMPTY, count: 2 }
const LEAVE: InstallDrop = { phase: 'leave', ...EMPTY }
const TARGET = { modRef: 'm:alpha', key: 'alpha' }

function mountOverlay() {
  const ipc = installIpcMock()
  renderWithProviders(
    <>
      <Notifications />
      <DropOverlay />
    </>,
  )
  return ipc
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
    usePictureDropStore.getState().clearTarget()
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
      images: 0,
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
      images: 0,
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

  it('offers a dragged picture to the open mod', async () => {
    mountOverlay()
    await waitUntilListening()
    usePictureDropStore.getState().setTarget(TARGET, 'Alpha')
    await emit(EVENT, { ...ENTER, count: 1, images: 1 })
    expect(await screen.findByText('Drop to use as the picture of Alpha')).toBeVisible()
    expect(screen.queryByText('1 item')).not.toBeInTheDocument()
  })

  it('keeps the install text when the drag is not only pictures', async () => {
    mountOverlay()
    await waitUntilListening()
    usePictureDropStore.getState().setTarget(TARGET, 'Alpha')
    await emit(EVENT, { ...ENTER, count: 2, images: 1 })
    expect(await screen.findByText('Drop archives or folders to install')).toBeVisible()
  })

  it('makes a dropped picture the open mod’s picture and installs the rest', async () => {
    const ipc = mountOverlay()
    await waitUntilListening()
    usePictureDropStore.getState().setTarget(TARGET, 'Alpha')
    await emit(EVENT, {
      phase: 'dropped',
      count: 2,
      images: 0,
      sources: [
        { id: 'src-1', name: 'look.png', kind: 'image' },
        { id: 'src-2', name: 'one.zip', kind: 'zip' },
      ],
      rejected: [],
    } satisfies InstallDrop)
    await waitFor(() => expect(ipc.callsOf('mods_set_picture')).toHaveLength(1))
    expect(ipc.callsOf('mods_set_picture')[0]?.payload).toEqual({
      target: TARGET,
      source: 'src-1',
    })
    expect(useInstallQueue.getState().queue).toEqual([
      { kind: 'token', id: 'src-2', name: 'one.zip', sourceKind: 'zip' },
    ])
  })

  it('asks for a mod when a picture is dropped with no details open', async () => {
    const ipc = mountOverlay()
    await waitUntilListening()
    await emit(EVENT, {
      phase: 'dropped',
      count: 1,
      images: 0,
      sources: [{ id: 'src-1', name: 'look.png', kind: 'image' }],
      rejected: [],
    } satisfies InstallDrop)
    expect(await screen.findByText('Which mod is this picture for?')).toBeVisible()
    expect(ipc.callsOf('mods_set_picture')).toEqual([])
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
      images: 0,
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
