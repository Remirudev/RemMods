import { IconPackages, IconSettings } from '@tabler/icons-react'
import { lazy } from 'react'
import type { PanelDefinition, PanelId } from './types'

const ModsGate = lazy(async () => ({
  default: (await import('./ModsGate')).ModsGate,
}))

const SettingsPanel = lazy(async () => ({
  default: (await import('../features/settings/SettingsPanel')).SettingsPanel,
}))

export const PANEL_IDS: readonly PanelId[] = ['mods', 'settings']

export const DEFAULT_PANEL: PanelId = 'mods'

export const panelRegistry: Record<PanelId, PanelDefinition> = {
  mods: {
    icon: IconPackages,
    component: ModsGate,
    layout: 'fill',
  },
  settings: {
    icon: IconSettings,
    component: SettingsPanel,
    layout: 'scroll',
  },
}

export function isPanelId(value: unknown): value is PanelId {
  return typeof value === 'string' && (PANEL_IDS as readonly string[]).includes(value)
}
