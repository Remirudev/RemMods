import { cleanup, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { SkinsView } from '../../lib/ipc/types'
import { usePictureDropStore } from '../../shared/picture-drop-store'
import { makeSkinSet, TEST_SKINS_VIEW } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { NewSkinSetDialog } from './NewSkinSetDialog'
import { SkinSetDialog } from './SkinSetDialog'

const VIEW: SkinsView = {
  ...TEST_SKINS_VIEW,
  sets: [
    makeSkinSet('First', { notes: 'Reina 3', screenshots: ['own-a.png'] }),
    makeSkinSet('Second'),
  ],
  active: 'second',
}

function renderDialog(setId = 'first', handlers = {}) {
  const onClose = vi.fn()
  const ipc = installIpcMock(undefined, {
    handlers: {
      skins_view: () => VIEW,
      assets_picture: () => 'data:image/png;base64,AAAA',
      ...handlers,
    },
  })
  renderWithProviders(<SkinSetDialog setId={setId} modalId="m" onClose={onClose} />)
  return { ipc, onClose }
}

describe('SkinSetDialog', () => {
  afterEach(() => {
    usePictureDropStore.getState().clearTarget()
  })

  it('edits the name and the notes', async () => {
    const { ipc } = renderDialog('first', { skins_edit: () => VIEW })
    const notes = await screen.findByRole('textbox', { name: 'Notes' })
    expect(notes).toHaveValue('Reina 3')

    await userEvent.type(notes, ' is the maid')
    await userEvent.click(screen.getByRole('button', { name: 'Save changes' }))

    await waitFor(() => expect(ipc.callsOf('skins_edit')).toHaveLength(1))
    expect(ipc.callsOf('skins_edit')[0]?.payload).toEqual({
      id: 'first',
      name: 'First',
      notes: 'Reina 3 is the maid',
    })
  })

  it('shows a taken name at the field', async () => {
    renderDialog('first', {
      skins_edit: () => {
        throw { kind: 'skinSetNameTaken' }
      },
    })
    const name = await screen.findByRole('textbox', { name: 'Name' })

    await userEvent.clear(name)
    await userEvent.type(name, 'Second')
    await userEvent.click(screen.getByRole('button', { name: 'Save changes' }))

    expect(await screen.findByText('A set with this name exists.')).toBeVisible()
  })

  it('adds screenshots through the picker', async () => {
    const { ipc } = renderDialog('first', {
      pick_screenshots: () => [
        { id: 'src-1', name: 'a.png', kind: 'image' },
        { id: 'src-2', name: 'b.png', kind: 'image' },
      ],
      skins_add_screenshot: () => VIEW,
    })

    await userEvent.click(await screen.findByRole('button', { name: 'Add screenshots…' }))

    await waitFor(() => expect(ipc.callsOf('skins_add_screenshot')).toHaveLength(2))
    expect(ipc.callsOf('skins_add_screenshot')[1]?.payload).toEqual({
      id: 'first',
      source: 'src-2',
    })
  })

  it('removes a screenshot and shows one larger', async () => {
    const { ipc } = renderDialog('first', { skins_remove_screenshot: () => VIEW })
    const shot = await screen.findByTestId('skin-shot')

    await userEvent.click(within(shot).getByRole('button', { name: 'Show larger' }))
    expect(screen.getByRole('button', { name: 'Show smaller' })).toBeVisible()

    await userEvent.click(within(shot).getByRole('button', { name: 'Remove screenshot' }))
    await waitFor(() => expect(ipc.callsOf('skins_remove_screenshot')).toHaveLength(1))
    expect(ipc.callsOf('skins_remove_screenshot')[0]?.payload).toEqual({
      id: 'first',
      shot: 'own-a.png',
    })
  })

  it('takes every picture dropped while it is open', async () => {
    const { ipc } = renderDialog('first', { skins_add_screenshot: () => VIEW })
    await waitFor(() => expect(usePictureDropStore.getState().target).not.toBeNull())
    const target = usePictureDropStore.getState().target
    expect(target?.many).toBe(true)

    await target?.apply('src-9')

    expect(ipc.callsOf('skins_add_screenshot')[0]?.payload).toEqual({
      id: 'first',
      source: 'src-9',
    })
    cleanup()
    expect(usePictureDropStore.getState().target).toBeNull()
  })

  it('does not delete the set in the game', async () => {
    renderDialog('second')
    expect(await screen.findByRole('button', { name: 'Delete set' })).toBeDisabled()
  })

  it('offers to put back only the set in the game', async () => {
    renderDialog('first')
    await screen.findByRole('button', { name: 'Delete set' })
    expect(screen.queryByRole('button', { name: 'Put back in the game' })).not.toBeInTheDocument()
  })

  it('puts the set back after asking', async () => {
    const { ipc } = renderDialog('second', { skins_put_back: () => VIEW })

    await userEvent.click(await screen.findByRole('button', { name: 'Put back in the game' }))
    expect(ipc.callsOf('skins_put_back')).toEqual([])
    await userEvent.click(await screen.findByRole('button', { name: 'Put back' }))

    await waitFor(() => expect(ipc.callsOf('skins_put_back')).toHaveLength(1))
    expect(ipc.callsOf('skins_put_back')[0]?.payload).toEqual({ id: 'second' })
  })

  it('leaves putting back alone during an unfinished switch', async () => {
    renderDialog('second', { skins_view: () => ({ ...VIEW, interrupted: 'first' }) })
    expect(await screen.findByRole('button', { name: 'Put back in the game' })).toBeDisabled()
  })
})

describe('NewSkinSetDialog', () => {
  function renderNew(handlers = {}, copy?: string) {
    const onClose = vi.fn()
    const ipc = installIpcMock(undefined, { handlers })
    renderWithProviders(<NewSkinSetDialog modalId="m" onClose={onClose} {...{ copy }} />)
    return { ipc, onClose }
  }

  it('keeps an automatic copy instead of the game’s skins', async () => {
    const { ipc, onClose } = renderNew({ skins_keep_copy: () => VIEW }, 'game-1')
    expect(screen.getByText(/Makes a new set from this copy/)).toBeVisible()

    await userEvent.type(screen.getByRole('textbox', { name: 'Name' }), 'Rescued')
    await userEvent.click(screen.getByRole('button', { name: 'Save set' }))

    await waitFor(() => expect(onClose).toHaveBeenCalled())
    expect(ipc.callsOf('skins_keep_copy')[0]?.payload).toEqual({ copy: 'game-1', name: 'Rescued' })
    expect(ipc.callsOf('skins_save_new')).toEqual([])
  })

  it('needs a name and saves', async () => {
    const { ipc, onClose } = renderNew({ skins_save_new: () => VIEW })
    const save = screen.getByRole('button', { name: 'Save set' })
    expect(save).toBeDisabled()

    await userEvent.type(screen.getByRole('textbox', { name: 'Name' }), '  Fine mods ')
    await userEvent.click(save)

    await waitFor(() => expect(onClose).toHaveBeenCalled())
    expect(ipc.callsOf('skins_save_new')[0]?.payload).toEqual({ name: 'Fine mods' })
  })

  it('shows a taken name at the field and stays open', async () => {
    const { onClose } = renderNew({
      skins_save_new: () => {
        throw { kind: 'skinSetNameTaken' }
      },
    })

    await userEvent.type(screen.getByRole('textbox', { name: 'Name' }), 'First')
    await userEvent.click(screen.getByRole('button', { name: 'Save set' }))

    expect(await screen.findByText('A set with this name exists.')).toBeVisible()
    expect(onClose).not.toHaveBeenCalled()
  })
})
