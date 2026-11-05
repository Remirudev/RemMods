import { cleanup, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it } from 'vitest'
import type { ModItem } from '../../lib/ipc/types'
import { usePictureDropStore } from '../../shared/picture-drop-store'
import { makeModItem, makeModRow } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { ModCover } from './ModCover'

const ROW = makeModRow('Alpha')
const TARGET = { modRef: ROW.modRef, key: ROW.key }
const PICKED = { id: 'src-1', name: 'look.png', kind: 'image' }

function renderCover(ownPicture: string | null, items: ModItem[] = [], picked: unknown = PICKED) {
  const ipc = installIpcMock(undefined, {
    handlers: {
      mods_items: () => ({ modRef: ROW.modRef, ownPicture, items }),
      assets_picture: (payload) => `data:image/png;base64,${(payload as { id: string }).id}`,
      pick_picture: () => picked,
    },
  })
  renderWithProviders(<ModCover row={ROW} />)
  return ipc
}

describe('ModCover', () => {
  afterEach(() => {
    usePictureDropStore.getState().clearTarget()
  })

  it('shows the owner’s picture before the mod’s', async () => {
    renderCover('own-1.png', [makeModItem('ker_bdf_a')])
    expect(await screen.findByAltText('Picture of Alpha')).toHaveAttribute(
      'src',
      'data:image/png;base64,own-1.png',
    )
    expect(screen.getByRole('button', { name: 'Change picture…' })).toBeVisible()
    expect(screen.getByRole('button', { name: 'Remove picture' })).toBeVisible()
  })

  it('falls back to the picture a mod ships', async () => {
    renderCover(null, [makeModItem('ker_bdf_a')])
    expect(await screen.findByAltText('Picture of Alpha')).toHaveAttribute(
      'src',
      'data:image/png;base64,mod-ker_bdf_a',
    )
    expect(screen.queryByRole('button', { name: 'Remove picture' })).not.toBeInTheDocument()
  })

  it('offers an empty tile when there is no picture at all', async () => {
    renderCover(null)
    expect(await screen.findByRole('img', { name: 'Picture of Alpha' })).toBeVisible()
    expect(screen.getByRole('button', { name: 'Choose a picture…' })).toBeVisible()
  })

  it('uses a picked picture and asks for a fresh list', async () => {
    const ipc = renderCover(null)
    await userEvent.click(await screen.findByRole('button', { name: 'Choose a picture…' }))
    await waitFor(() => expect(ipc.callsOf('mods_set_picture')).toHaveLength(1))
    expect(ipc.callsOf('pick_picture')[0]?.payload).toEqual({
      title: 'Choose a picture for this mod',
    })
    expect(ipc.callsOf('mods_set_picture')[0]?.payload).toEqual({
      target: TARGET,
      source: 'src-1',
    })
    await waitFor(() => expect(ipc.callsOf('mods_items').length).toBeGreaterThan(1))
  })

  it('does nothing when the file picker is cancelled', async () => {
    const ipc = renderCover(null, [], null)
    await userEvent.click(await screen.findByRole('button', { name: 'Choose a picture…' }))
    await waitFor(() => expect(ipc.callsOf('pick_picture')).toHaveLength(1))
    expect(ipc.callsOf('mods_set_picture')).toEqual([])
  })

  it('removes the picture', async () => {
    const ipc = renderCover('own-1.png')
    await userEvent.click(await screen.findByRole('button', { name: 'Remove picture' }))
    await waitFor(() => expect(ipc.callsOf('mods_clear_picture')).toHaveLength(1))
    expect(ipc.callsOf('mods_clear_picture')[0]?.payload).toEqual({ target: TARGET })
  })

  it('takes dropped pictures while it is open', async () => {
    renderCover(null)
    await waitFor(() => expect(usePictureDropStore.getState().target).not.toBeNull())
    expect(usePictureDropStore.getState().target?.label).toBe('Drop to use as the picture of Alpha')
    cleanup()
    expect(usePictureDropStore.getState().target).toBeNull()
  })
})
