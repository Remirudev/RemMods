import type { Page } from '@playwright/test'
import {
  actionBar,
  callsOf,
  dialog,
  expect,
  mockState,
  modRow,
  modsGrid,
  openApp,
  selectRows,
  test,
} from './app'
import { t } from './i18n'

const FIRST = t('mods.bulk.deleteFirst.title')
const FINAL = t('mods.bulk.deleteFinal.title')
const CANCEL = t('mods.delete.cancel')

async function startBulkDelete(page: Page): Promise<void> {
  await actionBar(page)
    .getByRole('button', { name: t('mods.actionBar.delete', { count: 2 }) })
    .click()
  await dialog(page, FIRST)
    .getByRole('button', { name: t('mods.bulk.deleteFirst.confirm') })
    .click()
}

test('bulk delete asks twice and the final button waits before it can be pressed', async ({
  page,
}) => {
  await openApp(page)
  await expect(modsGrid(page)).toBeVisible()
  await selectRows(page, ['Alpha', 'Charlie'])

  await startBulkDelete(page)
  const final = dialog(page, FINAL)
  const deleteButton = final.getByRole('button', {
    name: t('mods.bulk.deleteFinal.confirm', { count: 2 }),
  })
  await expect(deleteButton).toBeDisabled()
  await expect(deleteButton).toBeEnabled({ timeout: 5_000 })
  await final.getByRole('button', { name: CANCEL }).click()
  await expect(final).toHaveCount(0)
  expect(await callsOf(page, 'mods_delete_many')).toEqual([])
  expect((await mockState(page)).deleted).toEqual([])

  await startBulkDelete(page)
  await dialog(page, FINAL)
    .getByRole('button', { name: t('mods.bulk.deleteFinal.confirm', { count: 2 }) })
    .click()
  await expect(modRow(page, 'Alpha')).toHaveCount(0)
  await expect(modRow(page, 'Charlie')).toHaveCount(0)
  await expect(page.getByText(t('mods.bulk.done.delete', { count: 2 }))).toBeVisible()

  const deletes = await callsOf(page, 'mods_delete_many')
  expect(deletes).toHaveLength(1)
  const state = await mockState(page)
  expect(state.deleted).toEqual(['alpha', 'charlie'])
  expect(deletes[0]?.args).toEqual({
    request: {
      targets: [
        { modRef: 'm:alpha', key: 'alpha' },
        { modRef: 'm:charlie', key: 'charlie' },
      ],
      token: state.issuedTokens[1],
      applyWhileRunning: false,
    },
  })
})

test('Shift+Delete on a row of the selection opens the bulk delete', async ({ page }) => {
  await openApp(page)
  const grid = modsGrid(page)
  await expect(grid).toBeVisible()
  await grid.focus()
  await page.keyboard.press('ArrowDown')
  await page.keyboard.press('Shift+ArrowDown')
  await expect(actionBar(page)).toContainText(t('mods.actionBar.selected', { count: 2 }))

  await page.keyboard.press('Shift+Delete')
  const first = dialog(page, FIRST)
  await expect(first).toContainText(
    t('mods.bulk.deleteFirst.body', { count: 2, place: t('mods.trash.place.trash') }),
  )
  await first.getByRole('button', { name: CANCEL }).click()
  await expect(first).toHaveCount(0)
  expect(await callsOf(page, 'mods_delete_prepare_many', 'mods_delete_many')).toEqual([])
})
