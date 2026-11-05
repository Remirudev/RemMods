import { actionBar, dialog, expect, mockState, modsGrid, openApp, selectRows, test } from './app'
import { t } from './i18n'

const BIN = t('mods.trash.place.recycleBin')

test('on Windows the bulk trash names the Recycle Bin and reports a refusal', async ({ page }) => {
  await openApp(page, 'windows')
  await expect(modsGrid(page)).toBeVisible()
  await selectRows(page, ['Alpha', 'Bravo'])

  await actionBar(page)
    .getByRole('button', { name: t('mods.actionBar.trash', { count: 2, place: BIN }) })
    .click()
  const confirm = dialog(page, t('mods.bulk.trashConfirm.title', { place: BIN }))
  await expect(confirm).toContainText(BIN)
  await confirm
    .getByRole('button', { name: t('mods.bulk.trashConfirm.confirm', { place: BIN }) })
    .click()

  const results = page.getByTestId('bulk-result-row')
  await expect(results.filter({ hasText: 'Alpha' })).toContainText(t('mods.bulk.results.recycled'))
  await expect(results.filter({ hasText: 'Bravo' })).toContainText(
    t('mods.trash.unavailable.recycleRefused'),
  )

  const resultsDialog = dialog(page, t('mods.bulk.results.title'))
  await resultsDialog.getByRole('button', { name: t('mods.bulk.results.close') }).click()
  await expect(resultsDialog).toHaveCount(0)
  const state = await mockState(page)
  expect(state.trashed).toEqual(['alpha'])
  expect(state.deleted).toEqual([])
})
