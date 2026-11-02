import type { Page } from '@playwright/test'
import {
  callsOf,
  clickSwitch,
  dialog,
  expect,
  mockState,
  modsGrid,
  openApp,
  openPanel,
  test,
  toggleSwitch,
} from './app'
import { t } from './i18n'

const PROFILE = 'Set A'

async function applyShownDiff(page: Page): Promise<void> {
  const diff = dialog(page, t('profiles.diff.title', { name: PROFILE }))
  await expect(diff).toContainText(t('profiles.diff.enable', { count: 1 }))
  await expect(diff).toContainText('Alpha')
  await diff.getByRole('button', { name: t('profiles.diff.apply') }).click()
  await expect(diff).toHaveCount(0)
}

async function disableAlpha(page: Page): Promise<void> {
  await openPanel(page, 'mods')
  await expect(modsGrid(page)).toBeVisible()
  await clickSwitch(page, 'Alpha', true)
  await expect(toggleSwitch(page, 'Alpha', false)).not.toBeChecked()
}

test('a saved profile applies from the Mods header and from the Profiles panel', async ({
  page,
}) => {
  await openApp(page)
  await openPanel(page, 'profiles')
  await page.getByRole('button', { name: t('profiles.panel.save') }).click()
  const save = dialog(page, t('profiles.form.saveTitle'))
  await save.getByRole('textbox', { name: t('profiles.form.name') }).fill(PROFILE)
  await save.getByRole('button', { name: t('profiles.form.save'), exact: true }).click()
  await expect(page.getByTestId('profile-card').filter({ hasText: PROFILE })).toBeVisible()

  await disableAlpha(page)
  await page.getByRole('button', { name: t('mods.quickApply.button') }).click()
  await page.getByRole('menuitem', { name: new RegExp(PROFILE) }).click()
  await applyShownDiff(page)
  await expect(toggleSwitch(page, 'Alpha', true)).toBeChecked()
  await expect(page.getByText(t('profiles.apply.done', { count: 1 }))).toBeVisible()

  await openPanel(page, 'profiles')
  await expect(page.getByText(t('profiles.snapshots.title', { count: 1 }))).toBeVisible()

  await disableAlpha(page)
  await openPanel(page, 'profiles')
  await page
    .getByTestId('profile-card')
    .filter({ hasText: PROFILE })
    .getByRole('button', { name: t('profiles.card.apply') })
    .click()
  await applyShownDiff(page)
  await expect(page.getByText(t('profiles.snapshots.title', { count: 2 }))).toBeVisible()

  const state = await mockState(page)
  expect(state.rows.find((row) => row.key === 'alpha')?.state).toBe('enabled')
  const applies = await callsOf(page, 'profiles_apply')
  expect(applies).toHaveLength(2)
  expect(applies[0]?.args).toEqual({
    request: { profileId: 'profile-1', fingerprint: '+alpha\n', applyWhileRunning: false },
  })
})
