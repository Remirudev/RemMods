import { formatSize } from '../../src/shared/format-size'
import {
  actionBar,
  callsOf,
  dialog,
  expect,
  mockState,
  modRow,
  modsGrid,
  openApp,
  selectRange,
  test,
} from './app'
import { t } from './i18n'

const TRASH = t('mods.trash.place.trash')
const BRAVO_SIZE = formatSize(5 * 1024 * 1024, 'en') ?? ''

test('bulk trash, then the unavailable row is deleted only after the second confirmation', async ({
  page,
}) => {
  await openApp(page)
  await expect(modsGrid(page)).toBeVisible()
  await selectRange(page, 'Alpha', 'Charlie')

  await actionBar(page)
    .getByRole('button', { name: t('mods.actionBar.trash', { count: 3, place: TRASH }) })
    .click()
  const confirm = dialog(page, t('mods.bulk.trashConfirm.title', { place: TRASH }))
  await confirm
    .getByRole('button', { name: t('mods.bulk.trashConfirm.confirm', { place: TRASH }) })
    .click()

  const results = dialog(page, t('mods.bulk.results.title'))
  await expect(results).toContainText('Alpha')
  await expect(results).toContainText('Charlie')
  await expect(results).toContainText(t('mods.bulk.results.trashed'))
  await expect(results).toContainText('Bravo')
  await expect(results).toContainText(t('mods.trash.unavailable.otherDevice'))
  await expect(results).toContainText(BRAVO_SIZE)
  await expect(results).toContainText(t('mods.delete.final.irreversible'))

  // Nothing is deleted before the second confirmation.
  const between = await mockState(page)
  expect(between.trashed).toEqual(['alpha', 'charlie'])
  expect(between.deleted).toEqual([])
  expect(between.rows.map((row) => row.key)).toContain('bravo')
  expect(await callsOf(page, 'mods_delete_prepare_many', 'mods_delete_many')).toEqual([])

  await results
    .getByRole('button', { name: t('mods.bulk.results.deleteUnavailable', { count: 1 }) })
    .click()
  await expect(modRow(page, 'Bravo')).toHaveCount(0)
  await expect(modRow(page, 'Alpha')).toHaveCount(0)
  await expect(modRow(page, 'Delta')).toHaveCount(1)

  const calls = await callsOf(
    page,
    'mods_trash_many',
    'mods_delete_prepare_many',
    'mods_delete_many',
  )
  expect(calls.map((call) => call.cmd)).toEqual([
    'mods_trash_many',
    'mods_delete_prepare_many',
    'mods_delete_many',
  ])
  const bravo = { modRef: 'm:bravo', key: 'bravo' }
  expect(calls[0]?.args).toEqual({
    request: {
      targets: [
        { modRef: 'm:alpha', key: 'alpha' },
        bravo,
        { modRef: 'm:charlie', key: 'charlie' },
      ],
      applyWhileRunning: false,
    },
  })
  expect(calls[1]?.args).toEqual({ targets: [bravo] })
  const after = await mockState(page)
  expect(calls[2]?.args).toEqual({
    request: { targets: [bravo], token: after.issuedTokens[0], applyWhileRunning: false },
  })
  expect(after.deleted).toEqual(['bravo'])
})
