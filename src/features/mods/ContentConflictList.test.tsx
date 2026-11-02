import { screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { renderWithProviders } from '../../test/render'
import { ContentConflictList } from './ContentConflictList'

describe('ContentConflictList', () => {
  it('shows the shared count, the sample paths and the winner note', () => {
    renderWithProviders(
      <ContentConflictList
        conflicts={[
          {
            modRef: 'm:beta',
            key: 'beta',
            name: 'Beta',
            shared: 3,
            outfit: 0,
            sample: ['Smoke/Shared.uasset', 'Smoke/Other.uasset'],
          },
          { modRef: 'm:gamma', key: 'gamma', name: 'Gamma', shared: 1, outfit: 0, sample: [] },
        ]}
      />,
    )

    expect(screen.getByText('3 files in a different version than Beta')).toBeVisible()
    expect(screen.getByText('1 file in a different version than Gamma')).toBeVisible()
    expect(screen.getByText('Smoke/Shared.uasset')).toBeVisible()
    expect(screen.getByText('Smoke/Other.uasset')).toBeVisible()
    expect(screen.queryByText(/^Upper-body or full-body clothing/)).not.toBeInTheDocument()
    expect(
      screen.getByText('Which one the game shows is not known yet; it is checked in-game.'),
    ).toBeVisible()
  })

  it('lists every link and marks the clothing the mod list counts', () => {
    renderWithProviders(
      <ContentConflictList
        conflicts={[
          {
            modRef: 'm:top',
            key: 'top',
            name: 'Top',
            shared: 4,
            outfit: 2,
            sample: ['Character/Item/Base_Item/ker/bdu/BI_ker_bdu_a.uasset'],
          },
          { modRef: 'm:shoes', key: 'shoes', name: 'Shoes', shared: 6, outfit: 0, sample: [] },
        ]}
      />,
    )

    expect(screen.getByText('4 files in a different version than Top')).toBeVisible()
    expect(screen.getByText('6 files in a different version than Shoes')).toBeVisible()
    expect(screen.getAllByText('Upper-body or full-body clothing: 2')).toHaveLength(1)
    expect(
      screen.getByText(
        'In the mod list, “Same files” counts only mods that replace the same upper-body or full-body clothing.',
      ),
    ).toBeVisible()
  })
})
