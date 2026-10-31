import type { Appearance, Settings, SettingsUpdate } from '../../lib/ipc/types'
import type { AppearancePatch } from './types'

export const DEFAULT_APPEARANCE: Appearance = {
  scheme: 'dark',
  uiScale: 1,
  titleBar: 'custom',
}

export function appearanceUpdate(patch: AppearancePatch): SettingsUpdate {
  return { appearance: { ...patch } }
}

export function applySettingsUpdate(settings: Settings, update: SettingsUpdate): Settings {
  const patch = update.appearance
  if (!patch) return settings
  return {
    ...settings,
    appearance: {
      scheme: patch.scheme ?? settings.appearance.scheme,
      uiScale: patch.uiScale ?? settings.appearance.uiScale,
      titleBar: patch.titleBar ?? settings.appearance.titleBar,
    },
  }
}
