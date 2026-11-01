import type { MantineColorSchemeManager } from '@mantine/core'
import type { IconSettings } from '@tabler/icons-react'
import type { QueryClient } from '@tanstack/react-query'
import type { ComponentType, LazyExoticComponent, ReactNode } from 'react'
import type { GameStatus, PlatformInfo, Settings } from '../lib/ipc/types'

export type PanelId = 'mods' | 'profiles' | 'settings'

// 'scroll' panels scroll as a whole; 'fill' panels get the full height and scroll inside (the mod
// table keeps its header and filters in view).
export type PanelLayout = 'scroll' | 'fill'

export interface PanelDefinition {
  icon: typeof IconSettings
  component: LazyExoticComponent<ComponentType>
  layout: PanelLayout
}

export interface ActivePanelState {
  activePanel: PanelId
  visited: readonly PanelId[]
  setActivePanel: (panel: PanelId) => void
}

export interface PreloadedAppData {
  settings: Settings | null
  platformInfo: PlatformInfo | null
  gameStatus: GameStatus | null
}

export interface ProvidersProps {
  queryClient: QueryClient
  colorSchemeManager: MantineColorSchemeManager
  children: ReactNode
}

export interface ErrorBoundaryProps {
  area?: string
  children: ReactNode
}

export interface ErrorBoundaryState {
  error: Error | null
}

export interface ErrorFallbackProps {
  area?: string
  error: Error
  onReset: () => void
}

export interface ColorSchemeToggle {
  isDark: boolean
  toggle: () => void
}

export interface InputGuardOptions {
  blockBrowserKeys: boolean
}

export type BrowserKeyEvent = Pick<KeyboardEvent, 'key' | 'code' | 'ctrlKey' | 'altKey' | 'metaKey'>
