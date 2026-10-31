import type { MantineColorsTuple } from '@mantine/core'
import type { AccentName, Density, SizeStep } from './types'

// Brighter, more saturated violet: #7c5cff reads as shade 5 on the dark canvas, and shade 6 stays
// legible on the light one.
const VIOLET: MantineColorsTuple = [
  '#f3efff',
  '#e2d9fb',
  '#c3b0f6',
  '#a585f1',
  '#8d63ed',
  '#7c5cff',
  '#6f4be8',
  '#5f3ccf',
  '#4f2fb4',
  '#3f2299',
]

export const QUIET_CANVAS_ACCENTS: Record<AccentName, MantineColorsTuple> = {
  violet: VIOLET,
}

// Neutral near-black scale in place of Mantine's blue-grey dark:
// 7 canvas · 6 cards · 5 hover · 4 borders · 2 dimmed text · 0 primary text.
export const QUIET_CANVAS_DARK: MantineColorsTuple = [
  '#ededf0',
  '#c7c7ce',
  '#8a8a93',
  '#5e5e68',
  '#2a2a31',
  '#1f1f25',
  '#16161a',
  '#0b0b0f',
  '#08080b',
  '#050507',
]

export const QUIET_CANVAS_RADIUS = {
  xs: '6px',
  sm: '8px',
  md: '12px',
  lg: '16px',
  xl: '20px',
}

export const QUIET_CANVAS_SHADOWS = {
  xs: '0 1px 2px rgba(0, 0, 0, 0.3)',
  sm: '0 4px 12px -4px rgba(0, 0, 0, 0.4)',
  md: '0 10px 24px -10px rgba(0, 0, 0, 0.55)',
  lg: '0 18px 40px -16px rgba(0, 0, 0, 0.7)',
  xl: '0 28px 60px -24px rgba(0, 0, 0, 0.8)',
}

// `xs` is the size of every secondary line, not a caption size: at 13px under a dimmed grey the UI
// read as hard work, so nothing goes below 14px.
export const QUIET_CANVAS_TEXT_SIZES = {
  xs: '14px',
  sm: '15px',
  md: '16px',
  lg: '18px',
  xl: '20px',
}

export const QUIET_CANVAS_LINE_HEIGHTS = {
  xs: '1.5',
  sm: '1.5',
  md: '1.55',
  lg: '1.55',
  xl: '1.55',
}

export const QUIET_CANVAS_SPACING: Record<Density, Record<SizeStep, string>> = {
  comfortable: {
    xs: '10px',
    sm: '12px',
    md: '16px',
    lg: '20px',
    xl: '32px',
  },
  compact: {
    xs: '8px',
    sm: '10px',
    md: '12px',
    lg: '16px',
    xl: '24px',
  },
}

export const QUIET_CANVAS_FONTS = {
  sans: "'Inter Variable', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
  mono: "'JetBrains Mono Variable', ui-monospace, SFMono-Regular, Menlo, monospace",
}

export const QUIET_CANVAS_MOTION = {
  fast: 120,
  base: 200,
  expressive: 320,
  ease: 'cubic-bezier(0.2, 0.7, 0.2, 1)',
}
