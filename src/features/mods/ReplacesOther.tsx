import { Button, Stack, Text } from '@mantine/core'
import { useDisclosure } from '@mantine/hooks'
import { IconChevronDown, IconChevronRight } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { ItemPicture } from '../../shared/components/ItemPicture'
import { slotLabel } from './content-hints'
import classes from './mods.module.css'
import { internalName } from './replaces-model'
import type { ReplacesOtherProps } from './types'

const PICTURE = 40

export function ReplacesOther({ items }: ReplacesOtherProps) {
  const { t } = useTranslation('mods')
  const [opened, { toggle }] = useDisclosure(false)
  const Chevron = opened ? IconChevronDown : IconChevronRight

  return (
    <Stack gap={4}>
      <Button
        variant="subtle"
        color="gray"
        size="compact-sm"
        w="fit-content"
        leftSection={<Chevron size={16} stroke={1.8} />}
        aria-expanded={opened}
        onClick={toggle}
      >
        {t(($) => $.replaces.other, { count: items.length })}
      </Button>
      {/* Mounted only when open, so a closed list loads no pictures. */}
      {opened && (
        <Stack gap={6}>
          <Text size="xs" c="var(--qc-text-2)">
            {t(($) => $.replaces.otherHint)}
          </Text>
          {items.map((item) => {
            const name = item.name ?? internalName(item)
            return (
              <div key={item.key} className={classes.otherRow} data-testid="replaces-other">
                <ItemPicture
                  id={item.gamePicture}
                  label={t(($) => $.replaces.gamePicture, { name })}
                  width={PICTURE}
                  height={PICTURE}
                />
                <div className={classes.itemText}>
                  <Text size="sm" className={classes.itemName}>
                    <bdi>{name}</bdi>
                  </Text>
                  <Text size="xs" c="var(--qc-text-2)">
                    <bdi>{slotLabel(item.slot, t)}</bdi>
                  </Text>
                </div>
              </div>
            )
          })}
        </Stack>
      )}
    </Stack>
  )
}
