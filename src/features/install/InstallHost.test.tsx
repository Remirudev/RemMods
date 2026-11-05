import { act, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it } from 'vitest'
import type { InspectRequest } from '../../lib/ipc/types'
import { useInstallQueue } from '../../shared/install-queue'
import { makeCandidate, makeInspection } from '../../test/install-fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { InstallHost } from './InstallHost'

function nameOf(request: InspectRequest): string {
  return request.source.kind === 'token' ? request.source.id : request.source.row
}

describe('InstallHost', () => {
  beforeEach(() => useInstallQueue.getState().clear())

  it('opens the queue head and moves on to the next source when it closes', async () => {
    const user = userEvent.setup()
    const ipc = installIpcMock(undefined, {
      handlers: {
        install_inspect: (payload) => {
          const name = nameOf((payload as { request: InspectRequest }).request)
          return makeInspection([makeCandidate(0, name)], { sourceName: `${name}.zip` })
        },
      },
    })
    renderWithProviders(<InstallHost />)
    expect(screen.queryByTestId('install-preview')).not.toBeInTheDocument()

    act(() =>
      useInstallQueue.getState().push([
        { kind: 'token', id: 'first', name: 'first.zip', sourceKind: 'zip' },
        { kind: 'token', id: 'second', name: 'second.zip', sourceKind: 'zip' },
      ]),
    )
    expect(await screen.findByText('first.zip')).toBeVisible()
    expect(screen.getByText('1 more waiting')).toBeVisible()

    await user.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(await screen.findByText('second.zip')).toBeVisible()
    expect(screen.queryByText('first.zip')).not.toBeInTheDocument()
    expect(ipc.callsOf('install_inspect')).toHaveLength(2)

    await user.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(screen.queryByTestId('install-preview')).not.toBeInTheDocument()
  })

  it('opens a fresh modal for the same source queued twice', async () => {
    const user = userEvent.setup()
    const ipc = installIpcMock(undefined, {
      handlers: { install_inspect: () => makeInspection([makeCandidate(0, 'Cool')]) },
    })
    renderWithProviders(<InstallHost />)
    const source = { kind: 'archive', row: 'm:x', name: 'x.zip' } as const
    act(() => useInstallQueue.getState().push([source, source]))
    await screen.findByTestId('install-preview')
    await user.click(screen.getByRole('button', { name: 'Cancel' }))
    await screen.findByTestId('install-preview')
    expect(ipc.callsOf('install_inspect')).toHaveLength(2)
  })
})
