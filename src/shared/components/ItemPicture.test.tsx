import { screen, waitFor } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { TEST_SETTINGS } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { ItemPicture } from './ItemPicture'

const PICTURE = 'data:image/png;base64,AAAA'

function renderPicture(id: string | null, hidePictures = false) {
  const ipc = installIpcMock(undefined, { handlers: { assets_picture: () => PICTURE } })
  const settings = {
    ...TEST_SETTINGS,
    appearance: { ...TEST_SETTINGS.appearance, hidePictures },
  }
  renderWithProviders(<ItemPicture id={id} label="Blouse" width={40} height={40} />, { settings })
  return ipc
}

describe('ItemPicture', () => {
  it('shows the stored picture', async () => {
    renderPicture('192-a')
    expect(await screen.findByAltText('Blouse')).toHaveAttribute('src', PICTURE)
  })

  it('shows an empty tile without a picture', async () => {
    const ipc = renderPicture(null)
    expect(screen.getByRole('img', { name: 'Blouse' })).toBeVisible()
    await waitFor(() => expect(ipc.callsOf('assets_picture')).toEqual([]))
  })

  it('loads nothing while pictures are hidden', async () => {
    const ipc = renderPicture('192-a', true)
    expect(screen.getByRole('img', { name: 'Blouse' })).toBeVisible()
    await waitFor(() => expect(ipc.callsOf('assets_picture')).toEqual([]))
    expect(screen.queryByAltText('Blouse')).not.toBeInTheDocument()
  })
})
