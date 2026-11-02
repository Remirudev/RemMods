import {
  callsOf,
  clickSwitch,
  expect,
  modRow,
  modsGrid,
  openApp,
  selectRows,
  test,
  toggleSwitch,
} from './app'
import { t } from './i18n'

const ALPHA = { modRef: 'm:alpha', key: 'alpha' }

test('a row turns off with its switch and on again with Space', async ({ page }) => {
  await openApp(page)
  await expect(modsGrid(page)).toBeVisible()
  const alpha = modRow(page, 'Alpha')
  await expect(alpha.getByText(t('mods.states.enabled'), { exact: true })).toBeVisible()

  await clickSwitch(page, 'Alpha', true)
  await expect(alpha.getByText(t('mods.states.disabled'), { exact: true })).toBeVisible()
  await expect(toggleSwitch(page, 'Alpha', false)).not.toBeChecked()

  await selectRows(page, ['Alpha'])
  await modsGrid(page).focus()
  await page.keyboard.press('Space')
  await expect(alpha.getByText(t('mods.states.enabled'), { exact: true })).toBeVisible()
  await expect(toggleSwitch(page, 'Alpha', true)).toBeChecked()

  const toggles = await callsOf(page, 'mods_toggle')
  expect(toggles.map((call) => call.args)).toEqual([
    { request: { targets: [ALPHA], enable: false, applyWhileRunning: false } },
    { request: { targets: [ALPHA], enable: true, applyWhileRunning: false } },
  ])
})
