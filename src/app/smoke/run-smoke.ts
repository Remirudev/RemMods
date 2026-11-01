import { ipc } from '../../lib/ipc'
import { describeForLog, isIpcError } from '../../lib/ipc/ipc-error'
import type { GameStatus, Settings, SmokeCheck } from '../../lib/ipc/types'
import {
  checkCollectedErrors,
  checkCommand,
  checkGameStatus,
  checkMantineCss,
  checkModsListWithoutGame,
  checkModsTable,
  checkOnboarding,
  checkPlatformInfo,
  checkRail,
  checkSettings,
  checkTitleBar,
  failedCheck,
  passedCheck,
} from './smoke-checks'
import type { Loaded, Refusal } from './types'

const RENDER_TIMEOUT_MS = 10_000
const NAVIGATION_WAIT_MS = 1_000
const collectedErrors: string[] = []
let collecting = false

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => {
    window.setTimeout(resolve, ms)
  })
}

async function load<T>(read: () => Promise<T>): Promise<Loaded<T>> {
  try {
    return { ok: true, value: await read() }
  } catch (error) {
    return { ok: false, error: describeForLog(error) }
  }
}

async function refusalOf(read: () => Promise<unknown>): Promise<Refusal> {
  try {
    await read()
    return { rejected: false }
  } catch (error) {
    return { rejected: true, kind: isIpcError(error) ? error.appError.kind : null }
  }
}

async function waitForElement(selector: string): Promise<void> {
  const deadline = Date.now() + RENDER_TIMEOUT_MS
  while (document.querySelector(selector) === null && Date.now() < deadline) {
    await wait(100)
  }
}

// If the navigation filter let this through, the page would unload and no report would ever
// arrive; the Rust watchdog then fails the run, which is the intended outcome.
async function checkNavigationRefused(): Promise<SmokeCheck> {
  const origin = window.location.origin
  window.location.assign('https://example.com/')
  await wait(NAVIGATION_WAIT_MS)
  const current = window.location.origin
  return current === origin ? passedCheck('navigation') : failedCheck('navigation', current)
}

// The commands the other checks do not call, so a missing permission or broken wiring fails the
// run as well. The empty settings update takes the app lock and rewrites settings.json in the
// temp root.
async function checkRemainingCommands(settings: Loaded<Settings>): Promise<SmokeCheck[]> {
  const scale = settings.ok ? settings.value.appearance.uiScale : 1
  const appReady = await load(() => ipc.appReady())
  const settingsUpdate = await load(() => ipc.settingsUpdate({}))
  const setUiScale = await load(() => ipc.setUiScale(scale))
  return [
    checkCommand('appReady', appReady),
    checkCommand('settingsUpdate', settingsUpdate),
    checkCommand('setUiScale', setUiScale),
  ]
}

// Fixture run: the table renders and the read commands answer. Never launch, pick or reveal.
async function checkFixtureGame(): Promise<SmokeCheck[]> {
  await waitForElement('[data-testid="mods-row"]')
  const snapshot = await load(() => ipc.modsRefresh())
  const first = snapshot.ok ? snapshot.value.rows[0] : undefined
  const details = first
    ? await load(() => ipc.modsDetails(first.modRef))
    : ({ ok: false, error: 'no first row to read details of' } as const)
  const detect = await load(() => ipc.gameDetect())
  const ack = await load(() => ipc.gameAckUpdate())
  return [
    checkModsTable(document),
    checkCommand('modsRefresh', snapshot),
    checkCommand('modsDetails', details),
    checkCommand('gameDetect', detect),
    checkCommand('gameAckUpdate', ack),
  ]
}

async function checkGameChecks(status: Loaded<GameStatus>): Promise<SmokeCheck[]> {
  const statusCheck = checkGameStatus(status)
  if (!statusCheck.ok || !status.ok) return [statusCheck]
  if (status.value.location.kind === 'ready') return [statusCheck, ...(await checkFixtureGame())]
  await waitForElement('[data-testid="onboarding"]')
  const refusal = await refusalOf(() => ipc.modsList())
  return [statusCheck, checkOnboarding(document), checkModsListWithoutGame(refusal)]
}

export function startSmokeErrorCollection(): void {
  if (collecting) return
  collecting = true
  window.addEventListener('error', (event) => {
    collectedErrors.push(`error: ${event.message}`)
  })
  window.addEventListener('unhandledrejection', (event) => {
    collectedErrors.push(`unhandled rejection: ${describeForLog(event.reason)}`)
  })
  const consoleError = console.error.bind(console)
  console.error = (...args: unknown[]) => {
    collectedErrors.push(`console.error: ${args.map(describeForLog).join(' ')}`)
    consoleError(...args)
  }
}

async function collectChecks(): Promise<SmokeCheck[]> {
  await waitForElement('[data-testid="rail"]')
  const [settings, platformInfo, gameStatus] = await Promise.all([
    load(() => ipc.settingsGet()),
    load(() => ipc.platformInfo()),
    load(() => ipc.gameStatus()),
  ])
  const checks = [
    checkTitleBar(document, settings),
    checkRail(document),
    checkMantineCss(document),
    checkPlatformInfo(platformInfo),
    checkSettings(settings),
    ...(await checkRemainingCommands(settings)),
    ...(await checkGameChecks(gameStatus)),
    await checkNavigationRefused(),
  ]
  return [...checks, checkCollectedErrors(collectedErrors)]
}

export async function runSmoke(): Promise<void> {
  let checks: SmokeCheck[]
  try {
    checks = await collectChecks()
  } catch (error) {
    checks = [failedCheck('smoke', describeForLog(error))]
  }
  const ok = checks.length > 0 && checks.every((check) => check.ok)
  await ipc.smokeReport({ ok, checks })
}
