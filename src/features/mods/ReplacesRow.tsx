import { Group, Text } from '@mantine/core'
import { IconArrowRight, IconSparkles } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { ItemPicture } from '../../shared/components/ItemPicture'
import { slotLabel } from './content-hints'
import classes from './mods.module.css'
import { internalName } from './replaces-model'
import type { ReplacesRowProps } from './types'

const PICTURE = 76

export function ReplacesRow({ item }: ReplacesRowProps) {
  const { t } = useTranslation('mods')
  const internal = internalName(item)
  const name = item.name ?? internal
  const isNew = item.origin === 'adds'

  return (
    <div className={classes.itemRow} data-testid="replaces-row">
      {isNew ? (
        <div className={classes.newItem} title={t(($) => $.replaces.newItemHint)}>
          <IconSparkles size={18} stroke={1.5} />
          <Text size="xs">{t(($) => $.replaces.newItem)}</Text>
        </div>
      ) : (
        <ItemPicture
          id={item.gamePicture}
          label={t(($) => $.replaces.gamePicture, { name })}
          width={PICTURE}
          height={PICTURE}
        />
      )}
      <IconArrowRight size={16} stroke={1.5} aria-hidden="true" />
      <ItemPicture
        id={item.modPicture}
        label={t(($) => $.replaces.modPicture, { name })}
        width={PICTURE}
        height={PICTURE}
      />
      <div className={classes.itemText}>
        <Text size="sm" fw={500} className={classes.itemName}>
          <bdi>{name}</bdi>
        </Text>
        <Group gap={6}>
          <Text size="xs" c="var(--qc-text-2)">
            <bdi>{slotLabel(item.slot, t)}</bdi>
          </Text>
          {!item.live && (
            <Text size="xs" c="var(--qc-text-2)">
              {t(($) => $.drawer.off)}
            </Text>
          )}
        </Group>
        {item.name === null && (
          <Text size="xs" c="var(--qc-text-2)">
            {t(($) => $.replaces.internalName)}
          </Text>
        )}
      </div>
    </div>
  )
}
