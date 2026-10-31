import { Badge, Button, type MantineThemeComponents, Paper, Tooltip } from '@mantine/core'

export function buildComponentOverrides(): MantineThemeComponents {
  return {
    Button: Button.extend({
      defaultProps: { variant: 'filled' },
    }),
    // Badges carry short statuses in table cells, where the default ellipsis cut words in half.
    Badge: Badge.extend({
      styles: { label: { overflow: 'visible' } },
    }),
    Paper: Paper.extend({
      defaultProps: { radius: 'lg' },
    }),
    Tooltip: Tooltip.extend({
      defaultProps: { withArrow: true, openDelay: 250 },
    }),
  }
}
