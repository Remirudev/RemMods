import { screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { makeGraphicsState, WINDOWS_PLATFORM_INFO } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { DiagnosticsSection } from './DiagnosticsSection'

describe('DiagnosticsSection', () => {
  it('names the install kind and, on Linux, the graphics mode of this session', async () => {
    installIpcMock(undefined, {
      handlers: {
        update_channel: () => ({
          install: 'appImage',
          selfUpdate: true,
          configured: true,
          current: '0.1.0',
        }),
        graphics_get: () => makeGraphicsState({ mode: 'software', effective: 'forceShm' }),
      },
    })
    renderWithProviders(<DiagnosticsSection />)

    expect(screen.getByText('Installed as')).toBeVisible()
    expect(await screen.findByText('AppImage')).toBeVisible()
    expect(screen.getByText('Graphics mode')).toBeVisible()
    expect(await screen.findByText('GPU, shared memory')).toBeVisible()
  })

  it('has no graphics row on Windows', async () => {
    installIpcMock()
    renderWithProviders(<DiagnosticsSection />, { platformInfo: WINDOWS_PLATFORM_INFO })

    expect(await screen.findByText('Unknown')).toBeVisible()
    expect(screen.queryByText('Graphics mode')).not.toBeInTheDocument()
  })
})
