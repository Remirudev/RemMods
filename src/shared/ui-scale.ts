import type { UiScaleAction, UiScaleKeyEvent } from './types'

export const UI_SCALE_DEFAULT = 1

// Whole percent steps keep the arithmetic exact; 0.05 steps in floating point drift.
export const UI_SCALE_PERCENT = {
  min: 80,
  max: 150,
  step: 5,
}

export function clampUiScale(scale: number): number {
  const { min, max, step } = UI_SCALE_PERCENT
  const percent = Math.round((scale * 100) / step) * step
  return Math.min(max, Math.max(min, percent)) / 100
}

export function nextUiScale(current: number, action: UiScaleAction): number {
  if (action === 'reset') return UI_SCALE_DEFAULT
  const delta = action === 'increase' ? UI_SCALE_PERCENT.step : -UI_SCALE_PERCENT.step
  return clampUiScale(current + delta / 100)
}

// `+` and `=` both count: German layouts have a `+` key, US layouts put it on Shift + `=`.
export function uiScaleActionForKey(event: UiScaleKeyEvent): UiScaleAction | null {
  if (!event.ctrlKey || event.altKey || event.metaKey) return null
  switch (event.key) {
    case '+':
    case '=':
      return 'increase'
    case '-':
      return 'decrease'
    case '0':
      return 'reset'
    default:
      return null
  }
}
