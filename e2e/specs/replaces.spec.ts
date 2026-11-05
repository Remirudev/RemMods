import type { Page } from '@playwright/test'
import { callsOf, expect, modsGrid, openApp, test } from './app'
import { t } from './i18n'

async function openAlpha(page: Page) {
  await openApp(page)
  await modsGrid(page).getByText('Alpha', { exact: true }).click()
  await expect(page.getByTestId('mod-details')).toBeVisible()
}

test('the details show what a mod replaces, per character', async ({ page }) => {
  await openAlpha(page)
  const details = page.getByTestId('mod-details')

  await expect(details.getByText(t('mods.replaces.title'))).toBeVisible()
  await expect(details.getByTestId('replaces-row')).toHaveCount(2)
  await expect(details.getByText('Frilled blouse')).toBeVisible()
  await expect(details.getByText('Yor Style')).toBeVisible()
  await expect(details.getByText('New', { exact: true })).toBeVisible()
  await expect(details.getByAltText('In the game: Frilled blouse')).toBeVisible()
  await expect(details.getByAltText('With this mod: Frilled blouse')).toBeVisible()

  await details.getByText('Reina', { exact: true }).click()
  await expect(details.getByText('Rüschenbluse (Reina)')).toBeVisible()
  await expect(details.getByText('Yor Style')).toHaveCount(0)
})

test('hidden pieces wait behind a fold', async ({ page }) => {
  await openAlpha(page)
  const details = page.getByTestId('mod-details')

  await expect(details.getByText('Barefoot')).toHaveCount(0)
  await details.getByRole('button', { name: '1 more piece without its own picture' }).click()
  await expect(details.getByText('Barefoot')).toBeVisible()
})

test('a mod takes a picture of its own and gives it back', async ({ page }) => {
  await openAlpha(page)
  const details = page.getByTestId('mod-details')

  await details.getByRole('button', { name: t('mods.cover.choose') }).click()
  await expect(details.getByRole('button', { name: t('mods.cover.remove') })).toBeVisible()
  await expect
    .poll(async () => (await callsOf(page, 'mods_set_picture')).map((call) => call.args))
    .toEqual([{ target: { modRef: 'm:alpha', key: 'alpha' }, source: 'src-picture' }])

  await details.getByRole('button', { name: t('mods.cover.remove') }).click()
  await expect(details.getByRole('button', { name: t('mods.cover.choose') })).toBeVisible()
})
