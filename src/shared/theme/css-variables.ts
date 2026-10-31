import type { CSSVariablesResolver } from '@mantine/core'

// Text roles: primary, --qc-text-2 for secondary titles, dimmed for hints and counts. Each one
// clears 7:1 against both the canvas and a card. They live here rather than in tokens.css because
// Mantine writes its own variables at runtime, and the resolver output is what replaces them.
export const cssVariablesResolver: CSSVariablesResolver = () => ({
  variables: {},
  light: {
    '--mantine-color-body': '#faf9f7',
    '--mantine-color-dimmed': '#4a4f58',
    '--qc-text-2': '#33373f',
  },
  dark: {
    '--mantine-color-dimmed': '#a9a9b4',
    '--qc-text-2': '#c9c9d2',
  },
})
