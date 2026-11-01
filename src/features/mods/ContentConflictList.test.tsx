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
            sample: ['Smoke/Shared.uasset', 'Smoke/Other.uasset'],
          },
          { modRef: 'm:gamma', key: 'gamma', name: 'Gamma', shared: 1, sample: [] },
        ]}
      />,
    )

    expect(screen.getByText('3 files in a different version than Beta')).toBeVisible()
    expect(screen.getByText('1 file in a different version than Gamma')).toBeVisible()
    expect(screen.getByText('Smoke/Shared.uasset')).toBeVisible()
    expect(screen.getByText('Smoke/Other.uasset')).toBeVisible()
    expect(
      screen.getByText('Which one the game shows is not known yet; it is checked in-game.'),
    ).toBeVisible()
  })
})
