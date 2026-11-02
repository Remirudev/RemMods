import type { FolderBusy } from '../../src/lib/ipc/types'
import {
  callsOf,
  clickSwitch,
  emitEvent,
  expect,
  mockState,
  modsGrid,
  openApp,
  test,
  toggleSwitch,
} from './app'
import { t } from './i18n'

const TITLE = t('mods.folderBusy.title')

test('Cancel in the folder-busy toast stops the wait and changes nothing', async ({ page }) => {
  await openApp(page)
  await expect(modsGrid(page)).toBeVisible()

  await clickSwitch(page, 'Delta', true)
  const toast = page.getByTestId('folder-busy')
  await expect(toast).toBeVisible()
  await expect(toast).toContainText(t('mods.folderBusy.body', { folder: 'Mods/Delta', limit: 30 }))
  await expect(page.getByRole('alert').filter({ has: toast })).toContainText(TITLE)
  const cancel = toast.getByRole('button', { name: t('mods.folderBusy.cancel') })
  await expect(cancel).toBeFocused()

  await page.keyboard.press('Enter')
  await expect(toast).toHaveCount(0)
  await expect(page.getByText(t('errors.messages.busyCancelled'))).toBeVisible()
  await expect(toggleSwitch(page, 'Delta', true)).toBeChecked()

  const state = await mockState(page)
  expect(state.rows.find((row) => row.key === 'delta')?.state).toBe('enabled')
  expect(state.waits).toEqual([])
  const cancels = await callsOf(page, 'mods_busy_cancel')
  expect(cancels.map((call) => call.args)).toEqual([{ waitId: expect.any(Number) }])
})

test('a rollback wait cannot be cancelled and the toast ends with it', async ({ page }) => {
  await openApp(page)
  await expect(modsGrid(page)).toBeVisible()
  const folder = 'Mods/Delta'
  const waiting: FolderBusy = {
    waitId: 900,
    phase: 'waiting',
    folder,
    holders: [],
    elapsedMs: 2000,
    limitMs: 30_000,
    cancellable: false,
  }

  await emitEvent(page, 'folder-busy', waiting)
  const toast = page.getByTestId('folder-busy')
  await expect(toast).toContainText(t('mods.folderBusy.bodyUndo', { folder }))
  await expect(toast.getByRole('button', { name: t('mods.folderBusy.cancel') })).toHaveCount(0)

  await emitEvent(page, 'folder-busy', { ...waiting, phase: 'done', elapsedMs: 2500 })
  await expect(toast).toHaveCount(0)
  expect(await callsOf(page, 'mods_busy_cancel')).toEqual([])
})
