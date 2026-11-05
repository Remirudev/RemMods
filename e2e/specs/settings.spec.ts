import { callsOf, expect, openApp, openPanel, test } from './app'
import { t, tDe } from './i18n'

test('colour scheme and UI scale are saved through settings_update', async ({ page }) => {
  await openApp(page)
  await openPanel(page, 'settings')

  await page.getByText(t('settings.appearance.scheme.light'), { exact: true }).click()
  await expect(page.locator('html')).toHaveAttribute('data-mantine-color-scheme', 'light')

  await page.getByRole('slider', { name: t('settings.appearance.uiScale.label') }).focus()
  await page.keyboard.press('ArrowRight')
  await expect
    .poll(async () => (await callsOf(page, 'set_ui_scale')).map((call) => call.args))
    .toEqual([{ scale: 1.05 }])

  const updates = await callsOf(page, 'settings_update')
  expect(updates.map((call) => call.args)).toEqual([
    { update: { appearance: { scheme: 'light' } } },
    { update: { appearance: { uiScale: 1.05 } } },
  ])
})

test('the item pictures card shows what is stored and checks on request', async ({ page }) => {
  await openApp(page)
  await openPanel(page, 'settings')

  await expect(page.getByText('12 pictures (', { exact: false })).toBeVisible()
  await page.getByRole('button', { name: t('settings.itemPictures.refresh') }).click()
  await expect.poll(async () => (await callsOf(page, 'assets_refresh')).length).toBe(1)
})

// RemMods has no language setting: it follows the system language, which the browser locale is
// here.
test.describe('in German', () => {
  test.use({ locale: 'de-DE' })

  test('the UI speaks German when the system does', async ({ page }) => {
    await openApp(page)
    await expect(page.locator('html')).toHaveAttribute('lang', 'de')
    const rail = page.getByRole('navigation', { name: tDe('shell.rail.label') })
    await rail.getByRole('button', { name: tDe('shell.panels.settings.title') }).click()
    await expect(
      page.getByRole('heading', { name: tDe('settings.appearance.title'), exact: true }),
    ).toBeVisible()
  })
})
