import { Stack, Text } from '@mantine/core'
import { IconDownload } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import classes from './drop-overlay.module.css'
import { useInstallDrop } from './use-install-drop'

// Shown while files are dragged over the window. The drop itself arrives from Rust, so the
// overlay takes no pointer events.
export function DropOverlay() {
  const { t } = useTranslation('install')
  const { active, count } = useInstallDrop()
  if (!active) return null

  return (
    <div className={classes.overlay} data-testid="drop-overlay" role="status">
      <div className={classes.frame} />
      <Stack align="center" gap="xs">
        <IconDownload size={40} stroke={1.5} />
        <Text fw={600} fz="lg">
          {t(($) => $.overlay.title)}
        </Text>
        {count > 0 && (
          <Text size="sm" c="var(--qc-text-2)">
            {t(($) => $.overlay.count, { count })}
          </Text>
        )}
      </Stack>
    </div>
  )
}
