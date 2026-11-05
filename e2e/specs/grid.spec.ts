import type { Page } from '@playwright/test'
import { callsOf, expect, modsGrid, openApp, openPanel, test } from './app'
import { t } from './i18n'

async function showGrid(page: Page) {
  await page
    .getByRole('radiogroup', { name: t('mods.view.label') })
    .locator('label', { hasText: t('mods.view.grid') })
    .click()
  await expect(page.getByTestId('mods-card').first()).toBeVisible()
}

async function activeCard(page: Page) {
  return modsGrid(page).getAttribute('aria-activedescendant')
}

test('the table shows a picture per mod and hides the column on request', async ({ page }) => {
  await openApp(page)
  const first = modsGrid(page).getByAltText('Picture of Alpha')
  await expect(first).toBeVisible()

  await page.getByRole('button', { name: t('mods.columnsMenu.button') }).click()
  await page.getByRole('checkbox', { name: t('mods.columns.picture') }).click()
  await expect(first).toHaveCount(0)
})

test('the grid shows the same mods and is remembered', async ({ page }) => {
  await openApp(page)
  await showGrid(page)

  await expect(page.getByTestId('mods-card')).toHaveCount(5)
  await expect(page.getByTestId('mods-card').first()).toContainText('Alpha')
  await expect(page.getByTestId('mods-card').first()).toContainText('Yor Style')

  await page.reload()
  await expect(page.getByTestId('mods-card').first()).toBeVisible()
})

test('a card opens the details, and sorting works without table headers', async ({ page }) => {
  await openApp(page)
  await showGrid(page)

  await page.getByRole('button', { name: t('mods.grid.ascending') }).click()
  await expect(page.getByTestId('mods-card').first()).toContainText('Foxtrot')

  await page.getByTestId('mods-card').first().click()
  await expect(page.getByTestId('mod-details')).toBeVisible()
})

test('150 mods stay light: only what is on screen exists and loads', async ({ page }) => {
  await openApp(page, 'many')
  await expect(modsGrid(page).getByTestId('mods-row').first()).toBeVisible()
  const rows = await modsGrid(page).getByTestId('mods-row').count()
  expect(rows).toBeLessThan(60)
  await expect
    .poll(async () => (await callsOf(page, 'mods_items')).length)
    .toBeGreaterThanOrEqual(rows)
  expect((await callsOf(page, 'mods_items')).length).toBeLessThan(150)

  await showGrid(page)
  expect(await page.getByTestId('mods-card').count()).toBeLessThan(60)
})

test('the keyboard walks the grid by card and by line', async ({ page }) => {
  await openApp(page, 'many')
  await showGrid(page)
  const grid = modsGrid(page)
  await grid.focus()

  await page.keyboard.press('ArrowDown')
  expect(await activeCard(page)).toBe('mods-card-0')
  await page.keyboard.press('ArrowRight')
  expect(await activeCard(page)).toBe('mods-card-1')

  const columns = Number(await grid.getAttribute('aria-colcount'))
  await page.keyboard.press('ArrowDown')
  expect(await activeCard(page)).toBe(`mods-card-${1 + columns}`)

  await page.keyboard.press('End')
  await expect(grid).toHaveAttribute('aria-activedescendant', 'mods-card-149')
  await expect(page.getByTestId('mods-card').last()).toContainText('Mod 149')

  await page.keyboard.press('Home')
  await expect(grid).toHaveAttribute('aria-activedescendant', 'mods-card-0')
  await page.keyboard.press('Enter')
  await expect(page.getByTestId('mod-details')).toBeVisible()
})

test('a character filter narrows the list', async ({ page }) => {
  await openApp(page, 'many')
  await expect(page.getByTestId('mods-count')).toContainText('150')

  await page.getByRole('combobox', { name: t('mods.filters.characters') }).click()
  await page.getByRole('option', { name: 'Reina' }).click()

  await expect(page.getByTestId('mods-count')).toContainText('50')
  await page.getByRole('button', { name: t('mods.filters.clear') }).click()
  await expect(page.getByTestId('mods-count')).toContainText('150')
})

test('Hide pictures empties the list and the details', async ({ page }) => {
  await openApp(page)
  await expect(modsGrid(page).getByAltText('Picture of Alpha')).toBeVisible()

  await openPanel(page, 'settings')
  await page.getByRole('switch', { name: t('settings.itemPictures.hide') }).click({ force: true })
  await openPanel(page, 'mods')

  await expect(modsGrid(page).getByAltText('Picture of Alpha')).toHaveCount(0)
  await showGrid(page)
  await page.getByTestId('mods-card').first().click()
  await expect(page.getByTestId('mod-details').getByRole('img').first()).toBeVisible()
  await expect(page.getByTestId('mod-details').locator('img')).toHaveCount(0)
})
