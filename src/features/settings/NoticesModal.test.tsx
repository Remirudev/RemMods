import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { TEST_NOTICES } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { AboutSection } from './AboutSection'

describe('NoticesModal', () => {
  it('fetches the notices only once the dialog opens', async () => {
    const user = userEvent.setup()
    const ipc = installIpcMock()
    renderWithProviders(<AboutSection />)

    expect(screen.getByText('Source code')).toBeVisible()
    expect(ipc.callsOf('about_notices')).toHaveLength(0)

    await user.click(screen.getByRole('button', { name: 'Third-party notices' }))

    expect(await screen.findByRole('dialog', { name: 'Third-party notices' })).toBeVisible()
    await waitFor(() => expect(ipc.callsOf('about_notices')).toHaveLength(1))
  })

  it('shows the two texts in tabs, each inside a scroll area', async () => {
    const user = userEvent.setup()
    installIpcMock()
    renderWithProviders(<AboutSection />)

    await user.click(screen.getByRole('button', { name: 'Third-party notices' }))

    const tabs = await screen.findAllByRole('tab')
    expect(tabs.map((tab) => tab.textContent)).toEqual(['Rust crates', 'npm packages and fonts'])
    const rust = screen.getByText(/THIRD-PARTY NOTICES/)
    expect(rust).toBeVisible()
    expect(rust.tagName).toBe('PRE')

    const scrollAreas = screen.getAllByTestId('notice-scroll', { exact: true })
    expect(scrollAreas).toHaveLength(2)
    const texts = [TEST_NOTICES.rust, TEST_NOTICES.npm]
    texts.forEach((text, index) => {
      expect(scrollAreas[index]?.textContent).toBe(text)
    })

    await user.click(screen.getByRole('tab', { name: 'npm packages and fonts' }))
    expect(screen.getByText(/SIL OPEN FONT LICENSE/)).toBeVisible()
  })
})
