import { Channel } from '@tauri-apps/api/core'
import { ipc } from '../../lib/ipc'
import { describeForLog, isIpcError } from '../../lib/ipc/ipc-error'
import type {
  AppUpdateProgress,
  GameStatus,
  PlatformInfo,
  Settings,
  SmokeCheck,
} from '../../lib/ipc/types'
import { runInstallRoundTrip } from './install-round-trip'
import {
  checkCollectedErrors,
  checkCommand,
  checkEmptyProfiles,
  checkGameStatus,
  checkGraphics,
  checkInstallRoundTrip,
  checkMantineCss,
  checkModsListWithoutGame,
  checkModsTable,
  checkNotices,
  checkOnboarding,
  checkPlatformInfo,
  checkRail,
  checkSettings,
  checkTitleBar,
  checkToggleRoundTrip,
  checkUpdateChannel,
  checkUpdateRefused,
  failedCheck,
  passedCheck,
} from './smoke-checks'
import { runToggleRoundTrip } from './toggle-round-trip'
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

// The release commands without network: a smoke build has no updater config, so both update
// commands must refuse before they reach out; '0.0.0' is never a version the check found.
async function checkReleaseCommands(info: Loaded<PlatformInfo>): Promise<SmokeCheck[]> {
  const channel = await load(() => ipc.updateChannel())
  const check = await refusalOf(() => ipc.updateCheck())
  const install = await refusalOf(() =>
    ipc.updateInstall('0.0.0', new Channel<AppUpdateProgress>()),
  )
  const graphics = await load(() => ipc.graphicsGet())
  const notices = await load(() => ipc.aboutNotices())
  return [
    checkUpdateChannel(channel, info),
    checkUpdateRefused(check, install),
    checkGraphics(graphics, info),
    checkNotices(notices),
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
  const profiles = await load(() => ipc.profilesList())
  const repair = await load(() => ipc.repairList())
  const detect = await load(() => ipc.gameDetect())
  const ack = await load(() => ipc.gameAckUpdate())
  const roundTrip = snapshot.ok
    ? await load(() => runToggleRoundTrip(snapshot.value))
    : ({ ok: false, error: 'no snapshot to toggle in' } as const)
  const install = snapshot.ok
    ? await load(() => runInstallRoundTrip(snapshot.value))
    : ({ ok: false, error: 'no snapshot to install into' } as const)
  return [
    checkModsTable(document),
    checkCommand('modsRefresh', snapshot),
    checkCommand('modsDetails', details),
    checkEmptyProfiles(profiles),
    checkCommand('repairList', repair),
    checkCommand('gameDetect', detect),
    checkCommand('gameAckUpdate', ack),
    roundTrip.ok
      ? checkToggleRoundTrip(roundTrip.value)
      : failedCheck('toggleRoundTrip', roundTrip.error),
    install.ok
      ? checkInstallRoundTrip(install.value)
      : failedCheck('installRoundTrip', install.error),
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
    ...(await checkReleaseCommands(platformInfo)),
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
