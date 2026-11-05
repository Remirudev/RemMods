import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import type { GraphicsMode, GraphicsState } from '../../lib/ipc/types'
import { makeGraphicsState, WINDOWS_PLATFORM_INFO } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { GraphicsSection } from './GraphicsSection'
import { GraphicsStepBanner } from './GraphicsStepBanner'

const STEPPED = makeGraphicsState({
  mode: 'forceShm',
  effective: 'forceShm',
  step: { from: 'auto', to: 'forceShm' },
})

function renderSection(initial: GraphicsState) {
  let state = initial
  const ipc = installIpcMock(undefined, {
    handlers: {
      graphics_get: () => state,
      graphics_set_mode: (payload) => {
        state = { ...state, mode: (payload as { mode: GraphicsMode }).mode }
        return state
      },
      graphics_ack_step: (payload) => {
        const { revert } = payload as { revert: boolean }
        state = { ...state, step: null, mode: revert ? 'auto' : state.mode }
        return state
      },
    },
  })
  renderWithProviders(<GraphicsSection />)
  return ipc
}

describe('GraphicsSection', () => {
  it('is not rendered and asks nothing when the platform has no graphics fallback', async () => {
    const ipc = installIpcMock()
    renderWithProviders(
      <>
        <GraphicsSection />
        <GraphicsStepBanner />
      </>,
      { platformInfo: WINDOWS_PLATFORM_INFO },
    )

    await new Promise((resolve) => setTimeout(resolve, 20))
    expect(screen.queryByTestId('graphics-section')).not.toBeInTheDocument()
    expect(screen.queryByTestId('graphics-step-banner')).not.toBeInTheDocument()
    expect(ipc.callsOf('graphics_get')).toHaveLength(0)
  })

  it('stores a new mode through graphics_set_mode and says when it applies', async () => {
    const user = userEvent.setup()
    const ipc = renderSection(makeGraphicsState())

    expect(await screen.findByText(/Applies after RemMods restarts/)).toBeVisible()
    await waitFor(() => expect(screen.getByRole('radio', { name: 'Automatic' })).toBeChecked())
    await user.click(screen.getByText('Software'))

    await waitFor(() => expect(ipc.callsOf('graphics_set_mode')).toHaveLength(1))
    expect(ipc.callsOf('graphics_set_mode')[0]?.payload).toEqual({ mode: 'software' })
    await waitFor(() => expect(screen.getByRole('radio', { name: 'Software' })).toBeChecked())
  })

  it('disables the control while graphics.json is read-only', async () => {
    renderSection(makeGraphicsState({ readOnly: true }))

    expect(await screen.findByText(/cannot be changed here/)).toBeVisible()
    for (const name of ['Automatic', 'GPU, shared memory', 'Software']) {
      expect(screen.getByRole('radio', { name })).toBeDisabled()
    }
  })

  it('keeps the automatic step through graphics_ack_step(false)', async () => {
    const user = userEvent.setup()
    const ipc = renderSection(STEPPED)

    expect(await screen.findByText('Graphics mode changed')).toBeVisible()
    expect(
      screen.getByText(
        'RemMods did not start correctly last time, so it switched from Automatic to GPU, shared memory.',
      ),
    ).toBeVisible()
    await user.click(screen.getByRole('button', { name: 'Keep' }))

    await waitFor(() => expect(ipc.callsOf('graphics_ack_step')).toHaveLength(1))
    expect(ipc.callsOf('graphics_ack_step')[0]?.payload).toEqual({ revert: false })
    await waitFor(() =>
      expect(screen.queryByTestId('graphics-step-banner')).not.toBeInTheDocument(),
    )
  })

  it('undoes the automatic step through graphics_ack_step(true)', async () => {
    const user = userEvent.setup()
    const ipc = renderSection(STEPPED)

    await user.click(await screen.findByRole('button', { name: 'Switch back' }))

    await waitFor(() => expect(ipc.callsOf('graphics_ack_step')).toHaveLength(1))
    expect(ipc.callsOf('graphics_ack_step')[0]?.payload).toEqual({ revert: true })
    await waitFor(() => expect(screen.getByRole('radio', { name: 'Automatic' })).toBeChecked())
  })

  it('offers software rendering for good in a safe-graphics session', async () => {
    const user = userEvent.setup()
    const ipc = renderSection(makeGraphicsState({ safeFlag: true, effective: 'software' }))

    expect(
      await screen.findByText(
        'This session runs in safe graphics mode (started with --safe-graphics).',
      ),
    ).toBeVisible()
    await user.click(screen.getByRole('button', { name: 'Always use software rendering' }))

    await waitFor(() => expect(ipc.callsOf('graphics_set_mode')).toHaveLength(1))
    expect(ipc.callsOf('graphics_set_mode')[0]?.payload).toEqual({ mode: 'software' })
    await waitFor(() =>
      expect(
        screen.queryByRole('button', { name: 'Always use software rendering' }),
      ).not.toBeInTheDocument(),
    )
  })

  it('says that the user environment wins', async () => {
    renderSection(makeGraphicsState({ userEnv: true }))

    expect(await screen.findByText(/Your environment sets WebKit graphics variables/)).toBeVisible()
  })
})
