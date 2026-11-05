import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { makeModRow, makeSnapshot, TEST_INFO } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { MetadataEditor } from './MetadataEditor'

const ROW = makeModRow('Alpha', {
  info: {
    ...TEST_INFO,
    origin: 'file',
    author: 'Ann',
    defaulted: { version: true, category: true, description: true, author: false },
  },
})

function renderEditor() {
  const onDone = vi.fn()
  const ipc = installIpcMock(undefined, {
    handlers: { mods_set_metadata: () => makeSnapshot([ROW], { revision: 2 }) },
  })
  renderWithProviders(<MetadataEditor row={ROW} onDone={onDone} />)
  return { ipc, onDone }
}

describe('MetadataEditor', () => {
  it('shows defaulted fields empty with the default as placeholder', () => {
    renderEditor()

    expect(screen.getByRole('textbox', { name: 'Version' })).toHaveValue('')
    expect(screen.getByRole('textbox', { name: 'Version' })).toHaveAttribute('placeholder', '1.0')
    expect(screen.getByRole('textbox', { name: 'Category' })).toHaveAttribute('placeholder', 'Misc')
    expect(screen.getByRole('textbox', { name: 'Author' })).toHaveValue('Ann')
  })

  it('enables Save only when something changed, and sends blank fields as empty strings', async () => {
    const user = userEvent.setup()
    const { ipc, onDone } = renderEditor()
    const save = screen.getByRole('button', { name: 'Save' })

    expect(save).toBeDisabled()
    await user.clear(screen.getByRole('textbox', { name: 'Author' }))
    await user.type(screen.getByRole('textbox', { name: 'Version' }), '2.0')
    expect(save).toBeEnabled()
    await user.click(save)

    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1))
    expect(ipc.callsOf('mods_set_metadata')[0]?.payload).toEqual({
      request: {
        target: { modRef: ROW.modRef, key: ROW.key },
        info: { version: '2.0', category: '', description: '', author: '' },
      },
    })
  })

  it('leaves without a call on Cancel', async () => {
    const user = userEvent.setup()
    const { ipc, onDone } = renderEditor()

    await user.type(screen.getByRole('textbox', { name: 'Version' }), '2')
    await user.click(screen.getByRole('button', { name: 'Cancel' }))

    expect(onDone).toHaveBeenCalledTimes(1)
    expect(ipc.callsOf('mods_set_metadata')).toHaveLength(0)
  })
})
