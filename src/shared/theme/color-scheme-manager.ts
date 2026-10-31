import type { MantineColorScheme, MantineColorSchemeManager } from '@mantine/core'
import type { ColorScheme } from '../../lib/ipc/types'
import type { ColorSchemeMirror } from './types'

const MANTINE_SCHEMES: Record<ColorScheme, MantineColorScheme> = {
  dark: 'dark',
  light: 'light',
  system: 'auto',
}

export function toMantineScheme(scheme: ColorScheme): MantineColorScheme {
  return MANTINE_SCHEMES[scheme]
}

// settings.json (Rust) is the source of truth. This manager only mirrors it, so Mantine never
// touches localStorage; changes are saved with `settings_update` and arrive here through `sync`.
export function createColorSchemeMirror(initial: MantineColorScheme): ColorSchemeMirror {
  let current = initial
  let listener: ((scheme: MantineColorScheme) => void) | null = null

  const manager: MantineColorSchemeManager = {
    get: () => current,
    set: (scheme) => {
      current = scheme
    },
    subscribe: (onUpdate) => {
      listener = onUpdate
    },
    unsubscribe: () => {
      listener = null
    },
    clear: () => {
      current = initial
    },
  }

  return {
    manager,
    sync: (scheme) => {
      if (scheme === current) return
      current = scheme
      listener?.(scheme)
    },
  }
}
