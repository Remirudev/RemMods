import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it } from 'vitest'
import type { ModItem } from '../../lib/ipc/types'
import { makeModItem } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { useModsUiStore } from './mods-ui-store'
import { ReplacesSection } from './ReplacesSection'

const reina = { code: 'zbr', name: 'Reina', common: false }

function renderSection(items: ModItem[]) {
  const ipc = installIpcMock(undefined, {
    handlers: {
      mods_items: () => ({ modRef: 'm:alpha', ownPicture: null, items }),
      assets_picture: (payload) => `data:image/png;base64,${(payload as { id: string }).id}`,
    },
  })
  renderWithProviders(<ReplacesSection modRef="m:alpha" />)
  return ipc
}

async function firstRow() {
  const [row] = await screen.findAllByTestId('replaces-row')
  if (!row) throw new Error('no row')
  return within(row)
}

describe('ReplacesSection', () => {
  afterEach(() => {
    useModsUiStore.setState({ detailsCharacter: null })
  })

  it('stays away for a mod without items', async () => {
    const ipc = renderSection([])
    await expect.poll(() => ipc.callsOf('mods_items').length).toBe(1)
    expect(screen.queryByText('Replaces')).not.toBeInTheDocument()
  })

  it('shows the game’s picture next to the mod’s, with the item’s name and slot', async () => {
    renderSection([makeModItem('ker_bdu_blouse', { name: 'Frilled blouse' })])
    const row = await firstRow()
    expect(row.getByText('Frilled blouse')).toBeVisible()
    expect(row.getByText('Upper body')).toBeVisible()
    expect(await row.findByAltText('In the game: Frilled blouse')).toHaveAttribute(
      'src',
      'data:image/png;base64,game-ker_bdu_blouse',
    )
    expect(await row.findByAltText('With this mod: Frilled blouse')).toHaveAttribute(
      'src',
      'data:image/png;base64,mod-ker_bdu_blouse',
    )
  })

  it('names a single character without offering a choice', async () => {
    renderSection([makeModItem('ker_bdu_blouse')])
    await firstRow()
    expect(screen.getByText('Kunimitsu')).toBeVisible()
    expect(screen.queryByRole('radio')).not.toBeInTheDocument()
  })

  it('switches every row to the character clicked, and remembers it', async () => {
    renderSection([
      makeModItem('ker_bdu_blouse', { name: 'Kuni blouse' }),
      makeModItem('zbr_bdu_blouse', { name: 'Reina blouse', character: reina }),
    ])
    expect(await screen.findByText('Kuni blouse')).toBeVisible()
    expect(screen.queryByText('Reina blouse')).not.toBeInTheDocument()

    await userEvent.click(screen.getByRole('radio', { name: 'Reina' }))

    expect(await screen.findByText('Reina blouse')).toBeVisible()
    expect(screen.queryByText('Kuni blouse')).not.toBeInTheDocument()
    expect(useModsUiStore.getState().detailsCharacter).toBe('zbr')
  })

  it('marks an item the mod adds instead of showing an empty game picture', async () => {
    renderSection([
      makeModItem('ker_bdf_yor', { name: 'Yor Style', gamePicture: null, origin: 'adds' }),
    ])
    const row = await firstRow()
    expect(row.getByText('New')).toBeVisible()
    expect(row.queryByLabelText('In the game: Yor Style')).not.toBeInTheDocument()
  })

  it('says when only the internal name is known', async () => {
    renderSection([makeModItem('ker_bdf_diamondgirl')])
    const row = await firstRow()
    expect(row.getByText('bdf_diamondgirl')).toBeVisible()
    expect(row.getByText('Internal name: there is no text for it.')).toBeVisible()
  })

  it('marks an item of a switched-off container', async () => {
    renderSection([makeModItem('ker_bdu_blouse', { live: false })])
    const row = await firstRow()
    expect(row.getByText('off')).toBeVisible()
  })

  it('folds pieces without their own picture away until asked', async () => {
    renderSection([
      makeModItem('ker_bdf_outfit', { name: 'Outfit' }),
      makeModItem('ker_sho_hidden', { name: 'Barefoot', modPicture: null }),
    ])
    await firstRow()
    expect(screen.queryByText('Barefoot')).not.toBeInTheDocument()

    await userEvent.click(
      screen.getByRole('button', { name: '1 more piece without its own picture' }),
    )

    expect(await screen.findByText('Barefoot')).toBeVisible()
  })
})
