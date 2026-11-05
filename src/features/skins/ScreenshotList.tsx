import { Button, CloseButton, Group, Stack, Text, UnstyledButton } from '@mantine/core'
import { IconPhotoPlus } from '@tabler/icons-react'
import { useQueryClient } from '@tanstack/react-query'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ipc } from '../../lib/ipc'
import { ItemPicture } from '../../shared/components/ItemPicture'
import { usePictureDropStore } from '../../shared/picture-drop-store'
import { queryKeys } from '../../shared/query-keys'
import { useAddScreenshots, useRemoveScreenshot } from './queries'
import classes from './skins.module.css'
import type { ScreenshotListProps } from './types'

const SHOT_HEIGHT = 110
const ZOOM_HEIGHT = 420

// Screenshots of the game's slot pages show what a set holds: the saves themselves can't be read.
// While this is open, images dropped on the window land in the set.
export function ScreenshotList({ set }: ScreenshotListProps) {
  const { t } = useTranslation('skins')
  const client = useQueryClient()
  const add = useAddScreenshots(set.id)
  const remove = useRemoveScreenshot(set.id)
  const [zoomed, setZoomed] = useState<string | null>(null)
  const shown = zoomed !== null && set.screenshots.includes(zoomed) ? zoomed : null
  const label = t(($) => $.shots.dropLabel, { name: set.name })

  useEffect(() => {
    const { setTarget, clearTarget } = usePictureDropStore.getState()
    setTarget({
      label,
      many: true,
      apply: async (sourceId) => {
        client.setQueryData(queryKeys.skins, await ipc.skinsAddScreenshot(set.id, sourceId))
      },
    })
    return clearTarget
  }, [client, set.id, label])

  return (
    <Stack gap="xs">
      <Group justify="space-between">
        <Text size="sm" fw={500}>
          {t(($) => $.shots.title)}
        </Text>
        <Button
          size="compact-sm"
          variant="default"
          leftSection={<IconPhotoPlus size={16} stroke={1.8} />}
          loading={add.isPending}
          onClick={() => add.mutate(t(($) => $.shots.pickTitle))}
        >
          {t(($) => $.shots.add)}
        </Button>
      </Group>
      <Text size="xs" c="var(--qc-text-2)">
        {t(($) => $.shots.hint)}
      </Text>
      {shown !== null && (
        <UnstyledButton
          className={classes.zoomed}
          aria-label={t(($) => $.shots.zoomOut)}
          onClick={() => setZoomed(null)}
        >
          <ItemPicture id={shown} label={t(($) => $.shots.alt)} width="100%" height={ZOOM_HEIGHT} />
        </UnstyledButton>
      )}
      <div className={classes.shots}>
        {set.screenshots.map((shot) => (
          <div key={shot} className={classes.shot} data-testid="skin-shot">
            <UnstyledButton
              w="100%"
              aria-label={t(($) => $.shots.zoomIn)}
              onClick={() => setZoomed(shot)}
            >
              <ItemPicture
                id={shot}
                label={t(($) => $.shots.alt)}
                width="100%"
                height={SHOT_HEIGHT}
              />
            </UnstyledButton>
            <CloseButton
              size="sm"
              className={classes.shotRemove}
              aria-label={t(($) => $.shots.remove)}
              onClick={() => remove.mutate(shot)}
            />
          </div>
        ))}
      </div>
    </Stack>
  )
}
