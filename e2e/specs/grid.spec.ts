import type { Page } from '@playwright/test'
import { callsOf, expect, modRow, modsGrid, openApp, openPanel, test } from './app'
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

test('the mods take the whole window and the pictures fill the cards', async ({ page }) => {
  await page.setViewportSize({ width: 2000, height: 1000 })
  await openApp(page)
  await showGrid(page)

  const grid = await modsGrid(page).boundingBox()
  expect(grid?.width).toBeGreaterThan(1500)
  const picture = await page
    .getByTestId('mods-card')
    .first()
    .getByAltText(/Picture/)
    .boundingBox()
  const card = await page.getByTestId('mods-card').first().boundingBox()
  expect(picture?.height).toBeGreaterThan((card?.width ?? 0) - 4)
  await expect(page.getByTestId('mods-card').first()).not.toContainText(t('mods.states.enabled'))
})

test('the checkboxes, the count and the view switch stay on one line', async ({ page }) => {
  await openApp(page)
  await page.getByRole('textbox', { name: t('mods.filters.search') }).fill('a')
  const middle = async (name: string) => {
    const box = await page.getByRole('button', { name }).boundingBox()
    return (box?.y ?? 0) + (box?.height ?? 0) / 2
  }
  const checkbox = page.getByRole('checkbox', { name: t('mods.filters.issuesOnly') })

  for (const view of ['table', 'grid']) {
    if (view === 'grid') await showGrid(page)
    for (const width of [900, 1200, 1600, 2250]) {
      await page.setViewportSize({ width, height: 760 })
      const box = await checkbox.boundingBox()
      const line = (box?.y ?? 0) + (box?.height ?? 0) / 2
      expect(Math.abs((await middle(t('mods.filters.clear'))) - line)).toBeLessThan(6)
    }
  }
})

test('a right-click opens the row’s menu in the grid and in the table', async ({ page }) => {
  await openApp(page)
  const rename = page.getByRole('menuitem', { name: t('mods.rowMenu.rename') })
  await modRow(page, 'Bravo').click({ button: 'right' })
  await expect(rename).toBeVisible()
  await page.keyboard.press('Escape')
  await page.getByRole('columnheader', { name: t('mods.columns.name') }).click({ button: 'right' })
  await expect(rename).toHaveCount(0)

  await showGrid(page)
  await page.getByTestId('mods-card').nth(1).click({ button: 'right' })
  await expect(rename).toBeVisible()
  await page.getByTestId('mods-card').nth(3).click({ button: 'right' })
  await rename.click()

  await expect(page.getByRole('dialog').getByRole('textbox')).toHaveValue('Delta')
  await expect(page.getByTestId('mod-details')).toHaveCount(0)
})

test('a picture shows large after half a second, unless that is turned off', async ({ page }) => {
  await openApp(page)
  await showGrid(page)
  const picture = page.getByTestId('mods-card').first().getByAltText('Picture of Alpha')
  const preview = page.getByTestId('picture-preview')

  await picture.hover()
  await expect(preview).toBeVisible()
  const box = await preview.boundingBox()
  expect(box?.width).toBeGreaterThan((await picture.boundingBox())?.width ?? 0)
  await page.mouse.move(5, 5)
  await expect(preview).toHaveCount(0)

  await picture.hover()
  await expect(preview).toBeVisible()
  await page.mouse.wheel(0, 100)
  await expect(preview).toHaveCount(0)
  await page.mouse.move(5, 5)

  await openPanel(page, 'settings')
  await page
    .getByRole('switch', { name: t('settings.itemPictures.hoverPreview') })
    .click({ force: true })
  await openPanel(page, 'mods')
  await picture.hover()
  await page.waitForTimeout(800)
  await expect(preview).toHaveCount(0)
})
