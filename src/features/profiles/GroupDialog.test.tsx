import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Profile } from '../../lib/ipc/types'
import { makeModRow } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { makeProfile, makeView } from '../../test/profile-fixtures'
import { renderWithProviders } from '../../test/render'
import type { IpcHandler } from '../../test/types'
import { GroupDialog } from './GroupDialog'

const ROWS = [
  makeModRow('Apron'),
  makeModRow('Mavis', { state: 'disabled' }),
  makeModRow('Police', {
    conflictsWith: [{ modRef: 'm:apron', key: 'apron', name: 'Apron', paks: ['x.pak'] }],
  }),
]

function renderDialog(profile: Profile | null = null, handlers: Record<string, IpcHandler> = {}) {
  const onClose = vi.fn()
  const ipc = installIpcMock(undefined, {
    handlers: { profiles_save: () => makeView([]), ...handlers },
  })
  renderWithProviders(<GroupDialog profile={profile} rows={ROWS} onClose={onClose} />)
  return { ipc, onClose }
}

async function pick(user: ReturnType<typeof userEvent.setup>, mod: string, value: string) {
  const control = screen.getByRole('radiogroup', { name: mod })
  await user.click(within(control).getByText(value))
}

describe('GroupDialog', () => {
  it('creates a group with exactly the picked mods', async () => {
    const user = userEvent.setup()
    const { ipc, onClose } = renderDialog()

    await user.type(screen.getByRole('textbox', { name: 'Name' }), 'FineMods')
    await pick(user, 'Apron', 'On')
    await pick(user, 'Mavis', 'Off')
    await user.click(screen.getByRole('switch', { name: /Turn off every other mod/ }))
    await user.click(screen.getByRole('button', { name: 'Create group' }))

    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1))
    expect(ipc.callsOf('profiles_save')[0]?.payload).toEqual({
      save: {
        kind: 'picked',
        skins: null,
        id: null,
        name: 'FineMods',
        description: '',
        mode: 'exclusive',
        mods: [
          { key: 'apron', state: 'enabled' },
          { key: 'mavis', state: 'disabled' },
        ],
      },
    })
  })

  it('keeps listed mods that are not installed when editing', async () => {
    const user = userEvent.setup()
    const profile = makeProfile('p1', {
      name: 'BlenderSphere',
      description: 'kept',
      mods: [
        { key: 'Standard:Gone', state: 'enabled' },
        { key: 'apron', state: 'disabled' },
      ],
    })
    const { ipc } = renderDialog(profile)

    expect(screen.getByText('not installed')).toBeVisible()
    await user.click(screen.getByRole('button', { name: 'Save changes' }))

    await waitFor(() => expect(ipc.callsOf('profiles_save')).toHaveLength(1))
    expect(ipc.callsOf('profiles_save')[0]?.payload).toEqual({
      save: {
        kind: 'picked',
        skins: null,
        id: 'p1',
        name: 'BlenderSphere',
        description: 'kept',
        mode: 'additive',
        mods: [
          { key: 'Standard:Gone', state: 'enabled' },
          { key: 'apron', state: 'disabled' },
        ],
      },
    })
  })

  it('starts from the current state and sets every shown mod at once', async () => {
    const user = userEvent.setup()
    renderDialog()

    await user.click(screen.getByRole('button', { name: 'Start from current state' }))
    expect(screen.getByTestId('pick-counts')).toHaveTextContent('turns 2 on and 1 off')

    await user.type(screen.getByRole('textbox', { name: 'Mods' }), 'ma')
    expect(screen.getAllByTestId('pick-row')).toHaveLength(1)
    await user.click(screen.getByRole('button', { name: 'On' }))
    expect(screen.getByTestId('pick-counts')).toHaveTextContent('turns 3 on and 0 off')
  })

  it('warns when two mods switched on together replace the same files', async () => {
    const user = userEvent.setup()
    renderDialog()

    await pick(user, 'Apron', 'On')
    expect(screen.queryByTestId('clash-alert')).not.toBeInTheDocument()
    await pick(user, 'Police', 'On')

    expect(screen.getByTestId('clash-alert')).toHaveTextContent(
      'Apron and Police: only one of them shows in game.',
    )
  })

  it('shows a taken name next to the field and stays open', async () => {
    const user = userEvent.setup()
    const { onClose } = renderDialog(null, {
      profiles_save: () => {
        throw { kind: 'profileNameTaken' }
      },
    })

    await user.type(screen.getByRole('textbox', { name: 'Name' }), 'FineMods')
    await user.click(screen.getByRole('button', { name: 'Create group' }))

    expect(await screen.findByText('A group with this name already exists.')).toBeVisible()
    expect(onClose).not.toHaveBeenCalled()
  })
})
