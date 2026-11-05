import { ARCHIVE } from '../mock/state'
import { callsOf, dialog, expect, modRow, modsGrid, modsTab, openApp, test } from './app'
import { t } from './i18n'

test('an archive in Paks installs through preview and progress', async ({ page }) => {
  await openApp(page)
  await expect(modsGrid(page)).toBeVisible()
  await expect(modRow(page, 'Echo')).toHaveCount(0)

  await modsTab(page, 'archives').click()
  await page
    .getByRole('button', { name: t('install.row.installLabel', { name: ARCHIVE.relPath }) })
    .click()

  const modal = dialog(page, t('install.modal.titleFor', { name: ARCHIVE.relPath }))
  await expect(modal).toContainText(t('install.candidates.title'))
  await expect(modal).toContainText('Echo')
  await modal.getByRole('button', { name: t('install.actions.install'), exact: true }).click()

  const progress = modal.getByTestId('install-progress')
  await expect(progress).toBeVisible()
  await expect(progress).toContainText(t('install.progress.phase.committing'))
  await page.evaluate(() => window.__REMMODS_E2E__.releaseInstall())
  await expect(modal.getByTestId('install-result')).toContainText(
    t('install.result.installed', { count: 1 }),
  )
  await modal.getByRole('button', { name: t('install.actions.close') }).click()
  await expect(modal).toHaveCount(0)

  await modsTab(page, 'mods').click()
  await expect(modRow(page, 'Echo')).toHaveCount(1)

  // StrictMode runs the modal's mount effect twice on the dev server, so the inspect may go out
  // twice; exactly one install starts, after the inspect of the archive row.
  const calls = await callsOf(page, 'install_inspect', 'install_start')
  expect(calls.filter((call) => call.cmd === 'install_start')).toHaveLength(1)
  expect(calls[calls.length - 1]?.cmd).toBe('install_start')
  expect(calls[0]?.cmd).toBe('install_inspect')
  expect(calls[0]?.args).toMatchObject({
    request: { source: { kind: 'archive', row: ARCHIVE.modRef } },
  })
})
