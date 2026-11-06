import type { Locator, Page } from '@playwright/test'
import {
  callsOf,
  clickSwitch,
  dialog,
  expect,
  mockState,
  modsGrid,
  openApp,
  test,
  toggleSwitch,
} from './app'
import { t } from './i18n'

const GROUP = 'Set A'

function sidebar(page: Page): Locator {
  return page.getByRole('navigation', { name: t('profiles.sidebar.title') })
}

function groupSwitch(page: Page, on: boolean): Locator {
  const label = on
    ? t('profiles.sidebar.switchOff', { name: GROUP })
    : t('profiles.sidebar.switchOn', { name: GROUP })
  return sidebar(page).getByRole('switch', { name: label })
}

async function createGroup(page: Page, name = GROUP, mod = 'Alpha'): Promise<void> {
  // An empty sidebar offers the header's + and a button in its empty state; both open the dialog.
  await sidebar(page)
    .getByRole('button', { name: t('profiles.sidebar.newGroup') })
    .first()
    .click()
  const form = dialog(page, t('profiles.group.newTitle'))
  await form.getByRole('textbox', { name: t('profiles.group.name') }).fill(name)
  await form
    .getByRole('radiogroup', { name: mod })
    .getByText(t('profiles.group.pick.on'), { exact: true })
    .click()
  await form.getByRole('button', { name: t('profiles.group.create') }).click()
  await expect(form).toHaveCount(0)
}

async function applyShown(page: Page, title: string, change: string): Promise<void> {
  const diff = dialog(page, title)
  await expect(diff).toContainText(change)
  await expect(diff).toContainText('Alpha')
  await diff.getByRole('button', { name: t('profiles.diff.apply') }).click()
  await expect(diff).toHaveCount(0)
}

test('a group made in the Mods dashboard switches on, off and back with Undo', async ({ page }) => {
  await openApp(page)
  await expect(modsGrid(page)).toBeVisible()
  await createGroup(page)
  // Alpha is on, so the new group matches the disk and its switch is on.
  await expect(groupSwitch(page, true)).toBeChecked()

  await clickSwitch(page, 'Alpha', true)
  await expect(toggleSwitch(page, 'Alpha', false)).not.toBeChecked()
  await expect(groupSwitch(page, false)).not.toBeChecked()

  await groupSwitch(page, false).click({ force: true })
  await applyShown(
    page,
    t('profiles.diff.titleOn', { name: GROUP }),
    t('profiles.diff.enable', { count: 1 }),
  )
  await expect(toggleSwitch(page, 'Alpha', true)).toBeChecked()
  await expect(groupSwitch(page, true)).toBeChecked()

  await groupSwitch(page, true).click({ force: true })
  await applyShown(
    page,
    t('profiles.diff.titleOff', { name: GROUP }),
    t('profiles.diff.disable', { count: 1 }),
  )
  await expect(toggleSwitch(page, 'Alpha', false)).not.toBeChecked()
  await expect(page.getByText(t('profiles.apply.done', { count: 1 })).last()).toBeVisible()

  // Undo of the off switch applies the snapshot taken just before it.
  await page
    .getByRole('button', { name: t('profiles.apply.undo') })
    .last()
    .click()
  await expect(toggleSwitch(page, 'Alpha', true)).toBeChecked()

  const applies = await callsOf(page, 'profiles_apply')
  expect(applies.map((call) => (call.args as { request: unknown }).request)).toEqual([
    {
      profileIds: ['profile-1'],
      fingerprint: '+alpha\n',
      applyWhileRunning: false,
      direction: 'on',
    },
    {
      profileIds: ['profile-1'],
      fingerprint: '-alpha\n',
      applyWhileRunning: false,
      direction: 'off',
    },
    expect.objectContaining({ fingerprint: '+alpha\n', direction: 'on' }),
  ])
  const state = await mockState(page)
  expect(state.rows.find((row) => row.key === 'alpha')?.state).toBe('enabled')
})

test('a group’s name narrows the mod list to its mods', async ({ page }) => {
  await openApp(page)
  await expect(modsGrid(page)).toBeVisible()
  await createGroup(page)

  await sidebar(page)
    .getByRole('button', { name: t('profiles.sidebar.showMods', { name: GROUP }) })
    .click()
  await expect(page.getByTestId('mods-row')).toHaveCount(1)

  const groups = t('mods.filters.groups')
  await page.getByRole('button', { name: t('mods.filters.clearField', { name: groups }) }).click()
  await expect(page.getByTestId('mods-row')).toHaveCount(5)
})

test('the Groups filter and the sidebar pick the same groups', async ({ page }) => {
  await openApp(page)
  await expect(modsGrid(page)).toBeVisible()
  await createGroup(page)

  await page.getByRole('combobox', { name: t('mods.filters.groups') }).click()
  await page.getByRole('option', { name: GROUP }).click()

  await expect(page.getByTestId('mods-row')).toHaveCount(1)
  await expect(
    sidebar(page).getByRole('button', { name: t('profiles.sidebar.showMods', { name: GROUP }) }),
  ).toHaveAttribute('aria-pressed', 'true')
})

test('picked groups show together and switch together, with one Undo', async ({ page }) => {
  await openApp(page)
  await expect(modsGrid(page)).toBeVisible()
  await createGroup(page, 'Set A', 'Alpha')
  await createGroup(page, 'Set B', 'Bravo')
  await createGroup(page, 'Set C', 'Delta')
  const name = (group: string) =>
    sidebar(page).getByRole('button', { name: t('profiles.sidebar.showMods', { name: group }) })

  await name('Set A').click()
  await name('Set C').click({ modifiers: ['ControlOrMeta'] })
  await expect(page.getByTestId('mods-row')).toHaveCount(2)
  await name('Set C').click({ modifiers: ['Shift'] })
  await expect(page.getByTestId('group-selected')).toHaveCount(0)
  await name('Set A').click({ modifiers: ['Shift'] })
  await expect(page.getByTestId('group-selected')).toContainText('3')
  await expect(page.getByTestId('mods-row')).toHaveCount(3)

  await sidebar(page)
    .getByRole('switch', { name: t('profiles.sidebar.switchOff', { name: 'Set B' }) })
    .click({ force: true })
  const diff = dialog(page, t('profiles.diff.titleOffMany', { count: 3 }))
  await expect(diff).toContainText('Bravo')
  await diff.getByRole('button', { name: t('profiles.diff.apply') }).click()

  await expect(toggleSwitch(page, 'Alpha', false)).toBeVisible()
  await expect(toggleSwitch(page, 'Bravo', false)).toBeVisible()
  await expect(toggleSwitch(page, 'Delta', false)).toBeVisible()
  expect(await callsOf(page, 'profiles_apply')).toHaveLength(1)

  await page.getByRole('button', { name: t('profiles.apply.undo') }).click()
  await expect(toggleSwitch(page, 'Alpha', true)).toBeVisible()
  await expect(toggleSwitch(page, 'Delta', true)).toBeVisible()
})

test('All on and All off move every group', async ({ page }) => {
  await openApp(page)
  await expect(modsGrid(page)).toBeVisible()
  await createGroup(page, 'Set A', 'Alpha')
  await createGroup(page, 'Set B', 'Bravo')

  await sidebar(page)
    .getByRole('button', { name: t('profiles.sidebar.allOffHint') })
    .click()
  const off = dialog(page, t('profiles.diff.titleOffMany', { count: 2 }))
  await off.getByRole('button', { name: t('profiles.diff.apply') }).click()
  await expect(toggleSwitch(page, 'Alpha', false)).toBeVisible()
  await expect(toggleSwitch(page, 'Bravo', false)).toBeVisible()

  await sidebar(page)
    .getByRole('button', { name: t('profiles.sidebar.allOnHint') })
    .click()
  const on = dialog(page, t('profiles.diff.titleOnMany', { count: 2 }))
  await on.getByRole('button', { name: t('profiles.diff.apply') }).click()
  await expect(toggleSwitch(page, 'Alpha', true)).toBeVisible()
  await expect(toggleSwitch(page, 'Bravo', true)).toBeVisible()
})

test('switching without the change list applies at once', async ({ page }) => {
  await openApp(page)
  await expect(modsGrid(page)).toBeVisible()
  await createGroup(page)
  await sidebar(page)
    .getByRole('button', { name: t('profiles.sidebar.menu') })
    .click()
  await page.getByRole('menuitem', { name: t('profiles.menu.switchWithoutList') }).click()

  await groupSwitch(page, true).click({ force: true })

  await expect(toggleSwitch(page, 'Alpha', false)).not.toBeChecked()
  await expect(page.getByRole('dialog')).toHaveCount(0)
  const applies = await callsOf(page, 'profiles_apply')
  expect(applies).toHaveLength(1)
})
