import { createTheme, type MantineThemeOverride } from '@mantine/core'
import { buildComponentOverrides } from './components'
import {
  QUIET_CANVAS_ACCENTS,
  QUIET_CANVAS_DARK,
  QUIET_CANVAS_FONTS,
  QUIET_CANVAS_LINE_HEIGHTS,
  QUIET_CANVAS_MOTION,
  QUIET_CANVAS_RADIUS,
  QUIET_CANVAS_SHADOWS,
  QUIET_CANVAS_SPACING,
  QUIET_CANVAS_TEXT_SIZES,
} from './quiet-canvas'
import type { ThemeAppearance } from './types'

export const DEFAULT_THEME_APPEARANCE: ThemeAppearance = {
  accent: 'violet',
  radius: 'md',
  density: 'comfortable',
}

// The colour scheme is not part of the theme (the scheme manager mirrors settings.json), and the
// UI scale is webview zoom, so neither appears here.
export function buildTheme(appearance: Partial<ThemeAppearance> = {}): MantineThemeOverride {
  const { accent, radius, density } = { ...DEFAULT_THEME_APPEARANCE, ...appearance }
  return createTheme({
    primaryColor: accent,
    primaryShade: { light: 6, dark: 5 },
    colors: {
      [accent]: QUIET_CANVAS_ACCENTS[accent],
      dark: QUIET_CANVAS_DARK,
    },
    autoContrast: true,
    cursorType: 'pointer',
    respectReducedMotion: true,
    defaultRadius: radius,
    radius: QUIET_CANVAS_RADIUS,
    shadows: QUIET_CANVAS_SHADOWS,
    spacing: QUIET_CANVAS_SPACING[density],
    fontFamily: QUIET_CANVAS_FONTS.sans,
    fontFamilyMonospace: QUIET_CANVAS_FONTS.mono,
    headings: {
      fontFamily: QUIET_CANVAS_FONTS.sans,
      fontWeight: '650',
    },
    fontSizes: QUIET_CANVAS_TEXT_SIZES,
    lineHeights: QUIET_CANVAS_LINE_HEIGHTS,
    other: {
      motion: QUIET_CANVAS_MOTION,
    },
    components: buildComponentOverrides(),
  })
}
