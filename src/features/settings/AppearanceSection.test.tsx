import { act, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { AppearanceSection } from './AppearanceSection'

describe('AppearanceSection', () => {
  it('saves a new colour scheme with settings_update', async () => {
    const ipc = installIpcMock()
    const user = userEvent.setup()
    renderWithProviders(<AppearanceSection />)

    await user.click(screen.getByRole('radio', { name: 'Light' }))

    await waitFor(() => {
      expect(ipc.calls).toContainEqual({
        command: 'settings_update',
        payload: { update: { appearance: { scheme: 'light' } } },
      })
    })
  })

  it('saves a new UI scale and applies it with set_ui_scale', async () => {
    const ipc = installIpcMock()
    const user = userEvent.setup()
    renderWithProviders(<AppearanceSection />)

    const slider = screen.getByRole('slider', { name: 'UI scale' })
    act(() => slider.focus())
    await user.keyboard('{ArrowRight}')

    await waitFor(() => {
      expect(ipc.calls).toContainEqual({ command: 'set_ui_scale', payload: { scale: 1.05 } })
    })
    expect(ipc.calls).toContainEqual({
      command: 'settings_update',
      payload: { update: { appearance: { uiScale: 1.05 } } },
    })
  })
})
