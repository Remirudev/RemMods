import type {
  GameStatus,
  ModRow,
  ModState,
  ModsSnapshot,
  PlatformInfo,
  ProfilesView,
  Settings,
  SmokeCheck,
} from '../../lib/ipc/types'
import { QUIET_CANVAS_DARK } from '../../shared/theme/quiet-canvas'
import type { Loaded, Refusal } from './types'

const MAX_REPORTED_ERRORS = 10

export function passedCheck(name: string, detail?: string): SmokeCheck {
  return { name, ok: true, detail: detail ?? null }
}

export function failedCheck(name: string, detail: string): SmokeCheck {
  return { name, ok: false, detail }
}

export function checkTitleBar(doc: Document, settings: Loaded<Settings>): SmokeCheck {
  if (settings.ok && settings.value.appearance.titleBar === 'system') {
    return passedCheck('titleBar', 'system title bar in use')
  }
  const rendered = doc.querySelector('[data-testid="title-bar"]') !== null
  return rendered ? passedCheck('titleBar') : failedCheck('titleBar', 'title bar not rendered')
}

export function checkRail(doc: Document): SmokeCheck {
  const rendered = doc.querySelector('[data-testid="rail"]') !== null
  return rendered ? passedCheck('rail') : failedCheck('rail', 'rail not rendered')
}

// Mantine's static stylesheet defines --mantine-color-body as well (and dark-7 as #242424), so
// only values from its runtime <style> tag show that the CSP let the theme in: the Quiet Canvas
// dark-7 and the resolver's --qc-text-2.
export function checkMantineCss(doc: Document): SmokeCheck {
  const style = doc.defaultView?.getComputedStyle(doc.documentElement)
  const canvas = style?.getPropertyValue('--mantine-color-dark-7').trim().toLowerCase() ?? ''
  const secondaryText = style?.getPropertyValue('--qc-text-2').trim() ?? ''
  if (canvas !== QUIET_CANVAS_DARK[7]) return failedCheck('mantineCss', `dark-7 is '${canvas}'`)
  if (secondaryText === '') return failedCheck('mantineCss', '--qc-text-2 is not set')
  return passedCheck('mantineCss', `dark-7 ${canvas}, --qc-text-2 ${secondaryText}`)
}

export function checkPlatformInfo(info: Loaded<PlatformInfo>): SmokeCheck {
  if (!info.ok) return failedCheck('platformInfo', info.error)
  const { os, appVersion } = info.value
  if (os !== 'linux' && os !== 'windows') return failedCheck('platformInfo', `unexpected os ${os}`)
  if (appVersion.trim() === '') return failedCheck('platformInfo', 'empty appVersion')
  return passedCheck('platformInfo', `${os} ${appVersion}`)
}

export function checkSettings(settings: Loaded<Settings>): SmokeCheck {
  if (!settings.ok) return failedCheck('settings', settings.error)
  const { version } = settings.value
  return version === 1 ? passedCheck('settings') : failedCheck('settings', `version ${version}`)
}

export function checkCommand(name: string, result: Loaded<unknown>): SmokeCheck {
  return result.ok ? passedCheck(name) : failedCheck(name, result.error)
}

// The smoke data folder is fresh, so the profile list must come back empty and writable.
export function checkEmptyProfiles(view: Loaded<ProfilesView>): SmokeCheck {
  if (!view.ok) return failedCheck('profilesList', view.error)
  const { profiles, readOnly } = view.value
  if (profiles.length > 0 || readOnly) {
    return failedCheck('profilesList', `${profiles.length} profiles, readOnly ${readOnly}`)
  }
  return passedCheck('profilesList')
}

export function checkCollectedErrors(errors: readonly string[]): SmokeCheck {
  if (errors.length === 0) return passedCheck('errors')
  return failedCheck('errors', errors.slice(0, MAX_REPORTED_ERRORS).join('\n'))
}

// The smoke run points the app at a fixture (`--paks`) or at nothing: any other location would
// make the checks below depend on the machine.
export function checkGameStatus(status: Loaded<GameStatus>): SmokeCheck {
  if (!status.ok) return failedCheck('gameStatus', status.error)
  const { location } = status.value
  if (location.kind === 'notSet') return passedCheck('gameStatus', 'notSet')
  if (location.kind === 'ready' && location.install.source === 'fixture') {
    return passedCheck('gameStatus', 'ready (fixture)')
  }
  const detail = location.kind === 'ready' ? `ready (${location.install.source})` : location.kind
  return failedCheck('gameStatus', `unexpected location ${detail}`)
}

export function checkModsTable(doc: Document): SmokeCheck {
  const rows = doc.querySelectorAll('[data-testid="mods-row"]').length
  return rows >= 1 ? passedCheck('modsTable', `${rows} rows`) : failedCheck('modsTable', 'no rows')
}

export function checkOnboarding(doc: Document): SmokeCheck {
  const rendered = doc.querySelector('[data-testid="onboarding"]') !== null
  return rendered
    ? passedCheck('onboarding')
    : failedCheck('onboarding', 'onboarding panel not rendered')
}

export function checkModsListWithoutGame(refusal: Refusal): SmokeCheck {
  if (!refusal.rejected) return failedCheck('modsListWithoutGame', 'mods_list succeeded')
  return refusal.kind === 'gameNotSet'
    ? passedCheck('modsListWithoutGame')
    : failedCheck('modsListWithoutGame', `rejected with ${refusal.kind ?? 'an unknown error'}`)
}

// Only the generated smoke fixture carries these rows; a run without `--paks` has none.
export const TOGGLE_FIXTURE_KEYS = ['Standard:Alpha', 'Logic:Tool'] as const

export function findRowByKey(snapshot: ModsSnapshot, key: string): ModRow | undefined {
  return snapshot.rows.find((row) => row.key === key)
}

export function hasToggleFixture(snapshot: ModsSnapshot): boolean {
  return TOGGLE_FIXTURE_KEYS.every((key) => findRowByKey(snapshot, key) !== undefined)
}

// A problem text, or null when the row is in the expected state.
export function expectRowState(
  snapshot: ModsSnapshot,
  key: string,
  expected: ModState,
): string | null {
  const row = findRowByKey(snapshot, key)
  if (!row) return `${key}: row missing after the toggle`
  return row.state === expected ? null : `${key}: state ${row.state}, expected ${expected}`
}

export function checkToggleRoundTrip(failures: readonly string[] | null): SmokeCheck {
  if (failures === null) return passedCheck('toggleRoundTrip', 'no fixture')
  if (failures.length === 0) return passedCheck('toggleRoundTrip')
  return failedCheck('toggleRoundTrip', failures.slice(0, MAX_REPORTED_ERRORS).join('\n'))
}
