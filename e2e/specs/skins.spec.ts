import type { Page } from '@playwright/test'
import { callsOf, expect, mockState, openApp, test } from './app'
import { t } from './i18n'

async function saveSet(page: Page, name: string) {
  await page
    .getByRole('button', { name: t('skins.saveCurrent') })
    .first()
    .click()
  await page.getByRole('textbox', { name: t('skins.newSet.name') }).fill(name)
  await page.getByRole('button', { name: t('skins.newSet.create') }).click()
  await expect(page.getByTestId('skin-set-row').filter({ hasText: name })).toBeVisible()
}

test('skin sets are saved and switched from the Mods dashboard', async ({ page }) => {
  await openApp(page)
  await saveSet(page, 'Fine mods')
  await saveSet(page, 'BlenderSphere')

  const fine = page.getByTestId('skin-set-row').filter({ hasText: 'Fine mods' })
  const blender = page.getByTestId('skin-set-row').filter({ hasText: 'BlenderSphere' })
  await expect(blender).toContainText(t('skins.inGame'))

  await fine.getByRole('button').last().click()

  await expect(fine).toContainText(t('skins.inGame'))
  await expect(blender).not.toContainText(t('skins.inGame'))
  await expect(page.getByText(/Skins switched to Fine mods/)).toBeVisible()
})

test('a set takes notes and screenshots', async ({ page }) => {
  await openApp(page)
  await saveSet(page, 'Fine mods')

  await page.getByTestId('skin-set-row').getByText('Fine mods').click()
  const dialog = page.getByTestId('skin-set-dialog')
  await dialog.getByRole('textbox', { name: t('skins.dialog.notes') }).fill('Reina 3: the maid')
  await dialog.getByRole('button', { name: t('skins.dialog.save') }).click()
  await dialog.getByRole('button', { name: t('skins.shots.add') }).click()

  await expect(dialog.getByTestId('skin-shot')).toHaveCount(1)
  await expect
    .poll(async () => (await mockState(page)).skinSets[0]?.notes)
    .toBe('Reina 3: the maid')
})

test('a group brings its skin set along', async ({ page }) => {
  await openApp(page)
  await saveSet(page, 'Fine mods')
  await saveSet(page, 'BlenderSphere')

  await page
    .getByRole('button', { name: t('profiles.sidebar.newGroup') })
    .first()
    .click()
  const dialog = page.getByTestId('group-dialog')
  await dialog.getByRole('textbox', { name: t('profiles.group.name') }).fill('Fine')
  await dialog
    .getByRole('radiogroup', { name: 'Charlie' })
    .getByText(t('profiles.group.pick.on'), { exact: true })
    .click()
  await dialog.getByRole('combobox', { name: t('profiles.group.skinSet') }).click()
  await page.getByRole('option', { name: 'Fine mods' }).click()
  await dialog.getByRole('button', { name: t('profiles.group.create') }).click()
  await expect(dialog).toHaveCount(0)

  await page
    .getByRole('switch', { name: t('profiles.sidebar.switchOn', { name: 'Fine' }) })
    .click({ force: true })
  await page.getByRole('button', { name: t('profiles.diff.apply') }).click()

  await expect(page.getByText(/Skins switched to Fine mods/)).toBeVisible()
  await expect(page.getByTestId('skin-set-row').filter({ hasText: 'Fine mods' })).toContainText(
    t('skins.inGame'),
  )
  expect(await callsOf(page, 'profiles_apply')).toHaveLength(1)
})

test('the set in the game can be put back', async ({ page }) => {
  await openApp(page)
  await saveSet(page, 'Fine mods')

  await page.getByTestId('skin-set-row').getByText('Fine mods').click()
  await page.getByRole('button', { name: t('skins.dialog.putBack') }).click()
  await page.getByRole('button', { name: t('skins.putBack.confirm'), exact: true }).click()

  await expect(page.getByText('Fine mods is back in the game.')).toBeVisible()
  await expect.poll(async () => (await callsOf(page, 'skins_put_back')).length).toBe(1)
})

test('an automatic copy can be kept as a set', async ({ page }) => {
  await openApp(page)
  await saveSet(page, 'Fine mods')
  await saveSet(page, 'BlenderSphere')
  const fine = page.getByTestId('skin-set-row').filter({ hasText: 'Fine mods' })
  await fine.getByRole('button').last().click()
  await expect(fine).toContainText(t('skins.inGame'))

  await page.getByRole('button', { name: t('skins.copies.title') }).click()
  await page
    .getByTestId('skin-copy')
    .getByRole('button', { name: t('skins.copies.keep') })
    .click()
  await page.getByRole('textbox', { name: t('skins.newSet.name') }).fill('Rescued')
  await page.getByRole('button', { name: t('skins.newSet.create') }).click()

  const rescued = page.getByTestId('skin-set-row').filter({ hasText: 'Rescued' })
  await expect(rescued).toBeVisible()
  await expect(rescued).not.toContainText(t('skins.inGame'))
  await expect(fine).toContainText(t('skins.inGame'))
})

test('deleting a set closes its dialog', async ({ page }) => {
  await openApp(page)
  await saveSet(page, 'Fine mods')
  await saveSet(page, 'BlenderSphere')

  await page.getByTestId('skin-set-row').getByText('Fine mods').click()
  await page.getByRole('button', { name: t('skins.dialog.delete') }).click()
  await page.getByRole('button', { name: t('skins.delete.confirm'), exact: true }).click()

  await expect(page.getByTestId('skin-set-row')).toHaveCount(1)
  await expect(page.getByRole('dialog')).toHaveCount(0)
})
