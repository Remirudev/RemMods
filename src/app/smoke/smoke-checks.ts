import type { PlatformInfo, Settings, SmokeCheck } from '../../lib/ipc/types'
import { QUIET_CANVAS_DARK } from '../../shared/theme/quiet-canvas'
import type { Loaded } from './types'

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

export function checkCollectedErrors(errors: readonly string[]): SmokeCheck {
  if (errors.length === 0) return passedCheck('errors')
  return failedCheck('errors', errors.slice(0, MAX_REPORTED_ERRORS).join('\n'))
}
