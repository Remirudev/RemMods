import { Button, Group, Stack, Text } from '@mantine/core'
import { IconPhotoPlus, IconTrash } from '@tabler/icons-react'
import { useEffect, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { ItemPicture } from '../../shared/components/ItemPicture'
import { usePictureDropStore } from '../../shared/picture-drop-store'
import classes from './mods.module.css'
import { useModsUiStore } from './mods-ui-store'
import { useClearModPicture, useModItems, usePickModPicture } from './queries'
import { charactersOf, chosenCharacter, coverPicture } from './replaces-model'
import type { ModCoverProps } from './types'

const COVER_HEIGHT = 220

// The owner's own picture when there is one, else what the mod's items show. While this is
// mounted, an image dropped on the window lands here.
export function ModCover({ row }: ModCoverProps) {
  const { t, i18n } = useTranslation('mods')
  const items = useModItems(row.modRef, i18n.language).data
  const wanted = useModsUiStore((state) => state.detailsCharacter)
  const target = useMemo(() => ({ modRef: row.modRef, key: row.key }), [row.modRef, row.key])
  const pick = usePickModPicture(target)
  const clear = useClearModPicture(target)
  const list = items?.items
  const own = items?.ownPicture ?? null
  const shown = useMemo(() => {
    if (own !== null) return own
    const characters = charactersOf(list ?? [], i18n.language)
    return coverPicture(list ?? [], chosenCharacter(characters, wanted))
  }, [own, list, wanted, i18n.language])

  useEffect(() => {
    const { setTarget, clearTarget } = usePictureDropStore.getState()
    setTarget(target, row.name)
    return clearTarget
  }, [target, row.name])

  return (
    <Stack gap="xs">
      <div className={classes.cover} style={{ height: COVER_HEIGHT }}>
        <ItemPicture
          id={shown}
          label={t(($) => $.cover.alt, { name: row.name })}
          width="100%"
          height={COVER_HEIGHT}
        />
      </div>
      <Group gap="xs" justify="space-between">
        <Text size="xs" c="var(--qc-text-2)">
          {t(($) => $.cover.dropHint)}
        </Text>
        <Group gap="xs">
          {own !== null && (
            <Button
              size="compact-sm"
              variant="subtle"
              color="gray"
              leftSection={<IconTrash size={16} stroke={1.8} />}
              loading={clear.isPending}
              onClick={() => clear.mutate()}
            >
              {t(($) => $.cover.remove)}
            </Button>
          )}
          <Button
            size="compact-sm"
            variant="default"
            leftSection={<IconPhotoPlus size={16} stroke={1.8} />}
            loading={pick.isPending}
            onClick={() => pick.mutate(t(($) => $.cover.pickTitle))}
          >
            {own === null ? t(($) => $.cover.choose) : t(($) => $.cover.change)}
          </Button>
        </Group>
      </Group>
    </Stack>
  )
}
