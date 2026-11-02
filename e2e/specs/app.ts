import { test as base, expect, type Locator, type Page } from '@playwright/test'
import type { MockState, RecordedCall, Scenario } from '../mock/types'
import { t } from './i18n'

// Every test ends by checking that the app called no command the mock backend does not know.
export const test = base.extend<{ knownCommandsOnly: undefined }>({
  knownCommandsOnly: [
    async ({ page }, use) => {
      await use(undefined)
      const unknown = await page.evaluate(() => window.__REMMODS_E2E__.unknown)
      expect(unknown, 'commands the e2e mock backend does not answer').toEqual([])
    },
    { auto: true },
  ],
})

export { expect }

export async function openApp(page: Page, scenario: Scenario = 'ready'): Promise<void> {
  await page.goto(`/?scenario=${scenario}`)
  await expect(page.getByTestId('title-bar')).toBeVisible()
  await page.waitForFunction(() => window.__REMMODS_E2E__ !== undefined)
}

export function mockState(page: Page): Promise<MockState> {
  return page.evaluate(() => window.__REMMODS_E2E__.state)
}

export async function callsOf(page: Page, ...commands: string[]): Promise<RecordedCall[]> {
  const calls = await page.evaluate(() => window.__REMMODS_E2E__.calls)
  return calls.filter((call) => commands.includes(call.cmd))
}

export function emitEvent(page: Page, event: string, payload: unknown): Promise<void> {
  return page.evaluate(([name, body]) => window.__REMMODS_E2E__.emit(name as string, body), [
    event,
    payload,
  ] as const)
}

export function openPanel(page: Page, panel: 'mods' | 'profiles' | 'settings'): Promise<void> {
  return page
    .getByRole('navigation', { name: t('shell.rail.label') })
    .getByRole('button', { name: t(`shell.panels.${panel}.title`) })
    .click()
}

export function modsGrid(page: Page): Locator {
  return page.getByRole('grid', { name: t('mods.table.label') })
}

export function modRow(page: Page, name: string): Locator {
  return page.getByTestId('mods-row').filter({ has: page.getByText(name, { exact: true }) })
}

// A plain click opens the modal details drawer, whose overlay would take the next click, so rows
// are selected with Ctrl+click (sets the anchor, opens nothing) and Shift+click for a range.
export async function selectRows(page: Page, names: readonly string[]): Promise<void> {
  for (const name of names) {
    await modRow(page, name)
      .getByText(name, { exact: true })
      .click({ modifiers: ['ControlOrMeta'] })
  }
}

export async function selectRange(page: Page, first: string, last: string): Promise<void> {
  await selectRows(page, [first])
  await modRow(page, last)
    .getByText(last, { exact: true })
    .click({ modifiers: ['Shift'] })
}

export function dialog(page: Page, title: string): Locator {
  return page.getByRole('dialog', { name: title })
}

export function toggleSwitch(page: Page, name: string, enabled: boolean): Locator {
  const label = enabled ? t('mods.toggle.disable', { name }) : t('mods.toggle.enable', { name })
  return page.getByRole('switch', { name: label })
}

// Mantine draws the switch's track over its input, so the click lands on the track as a user's
// does; force skips Playwright's check that the input itself receives the pointer.
export function clickSwitch(page: Page, name: string, enabled: boolean): Promise<void> {
  return toggleSwitch(page, name, enabled).click({ force: true })
}

export function actionBar(page: Page): Locator {
  return page.getByRole('toolbar', { name: t('mods.actionBar.label') })
}

function prefixPattern(text: string): RegExp {
  return new RegExp(`^${text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`)
}

// Tab names end with their count badge ("Archives 1"), so they match by prefix.
export function modsTab(page: Page, tab: 'mods' | 'archives'): Locator {
  return page.getByRole('tab', { name: prefixPattern(t(`mods.tabs.${tab}`)) })
}
