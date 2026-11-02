import { callsOf, expect, modsGrid, openApp, test } from './app'
import { t } from './i18n'

test('onboarding picks the detected game folder and shows the mods', async ({ page }) => {
  await openApp(page, 'onboarding')
  const onboarding = page.getByTestId('onboarding')
  await expect(onboarding.getByRole('heading', { name: t('onboarding.title') })).toBeVisible()
  await expect(onboarding).toContainText(t('onboarding.found.none'))

  await onboarding.getByRole('button', { name: t('onboarding.detectAgain') }).click()
  const candidate = onboarding.getByTestId('game-candidate')
  await expect(candidate).toHaveCount(1)
  await candidate.getByRole('button', { name: t('game.candidate.use') }).click()

  await expect(modsGrid(page)).toBeVisible()
  await expect(onboarding).toHaveCount(0)
  const choices = await callsOf(page, 'game_set_location')
  expect(choices.map((call) => call.args)).toEqual([
    { choice: { kind: 'candidate', id: 'candidate-1' } },
  ])
})
