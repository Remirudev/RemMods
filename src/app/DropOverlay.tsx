import { Stack, Text } from '@mantine/core'
import { IconDownload, IconPhoto } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { usePictureDropStore } from '../shared/picture-drop-store'
import classes from './drop-overlay.module.css'
import { useInstallDrop } from './use-install-drop'

// Shown while files are dragged over the window. The drop itself arrives from Rust, so the
// overlay takes no pointer events.
export function DropOverlay() {
  const { t } = useTranslation('install')
  const { active, count, images } = useInstallDrop()
  const pictureLabel = usePictureDropStore((state) => state.target?.label ?? null)
  if (!active) return null

  // Only when the whole drag is pictures and something open takes them: the rest installs.
  const forPicture = pictureLabel !== null && count > 0 && images === count

  return (
    <div className={classes.overlay} data-testid="drop-overlay" role="status">
      <div className={classes.frame} />
      <Stack align="center" gap="xs">
        {forPicture ? (
          <IconPhoto size={40} stroke={1.5} />
        ) : (
          <IconDownload size={40} stroke={1.5} />
        )}
        <Text fw={600} fz="lg">
          {forPicture ? pictureLabel : t(($) => $.overlay.title)}
        </Text>
        {count > 0 && !forPicture && (
          <Text size="sm" c="var(--qc-text-2)">
            {t(($) => $.overlay.count, { count })}
          </Text>
        )}
      </Stack>
    </div>
  )
}
