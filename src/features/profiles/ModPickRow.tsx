import { SegmentedControl, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import classes from './groups.module.css'
import type { ModPick, ModPickRowProps } from './types'

const PICKS: readonly ModPick[] = ['on', 'off', 'skip']

// On and Off stand out from "leave alone" at a glance in a long list.
const PICK_COLOR: Record<ModPick, string | undefined> = {
  on: 'var(--mantine-primary-color-filled)',
  off: 'gray',
  skip: undefined,
}

function isPick(value: string): value is ModPick {
  return (PICKS as readonly string[]).includes(value)
}

export function ModPickRow({ item, pick, onPick }: ModPickRowProps) {
  const { t } = useTranslation('profiles')
  const now =
    item.on === null
      ? t(($) => $.group.notInstalled)
      : item.on
        ? t(($) => $.group.nowOn)
        : t(($) => $.group.nowOff)

  return (
    <div className={classes.pickRow} data-testid="pick-row">
      <div className={classes.pickName}>
        <Text size="sm" truncate="end">
          <bdi>{item.name}</bdi>
        </Text>
        <Text size="xs" c="var(--qc-text-2)">
          {now}
        </Text>
      </div>
      <SegmentedControl
        size="xs"
        color={PICK_COLOR[pick]}
        value={pick}
        aria-label={item.name}
        data={PICKS.map((value) => ({ value, label: t(($) => $.group.pick[value]) }))}
        onChange={(value) => {
          if (isPick(value)) onPick([item.key], value)
        }}
      />
    </div>
  )
}
