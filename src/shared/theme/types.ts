import type { MantineColorScheme, MantineColorSchemeManager } from '@mantine/core'

export type AccentName = 'violet'

export type Density = 'comfortable' | 'compact'

export type SizeStep = 'xs' | 'sm' | 'md' | 'lg' | 'xl'

export interface ThemeAppearance {
  accent: AccentName
  radius: SizeStep
  density: Density
}

export interface ColorSchemeMirror {
  manager: MantineColorSchemeManager
  sync: (scheme: MantineColorScheme) => void
}
