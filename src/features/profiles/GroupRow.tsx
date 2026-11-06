import { Loader, Switch, Text, Tooltip, UnstyledButton } from '@mantine/core'
import { IconAlertTriangle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { GroupRowMenu } from './GroupRowMenu'
import classes from './groups.module.css'
import type { GroupRowProps } from './types'

export function GroupRow({ item, rows, active, busy, readOnly, onSwitch, onShow }: GroupRowProps) {
  const { t } = useTranslation('profiles')
  const { summary, title, lit, clashes } = item
  const count =
    summary.missing > 0
      ? t(($) => $.sidebar.countMissing, { count: summary.enabled, missing: summary.missing })
      : t(($) => $.sidebar.count, { count: summary.enabled })

  return (
    <div className={classes.row} data-active={active || undefined} data-testid="group-row">
      <Switch
        size="sm"
        checked={lit}
        disabled={readOnly || busy}
        onChange={() => onSwitch(item)}
        aria-label={
          lit
            ? t(($) => $.sidebar.switchOff, { name: title })
            : t(($) => $.sidebar.switchOn, { name: title })
        }
      />
      <Tooltip label={count} openDelay={400} position="right" withinPortal>
        <UnstyledButton
          className={classes.name}
          onClick={(event) => onShow(item, event)}
          aria-pressed={active}
          aria-label={t(($) => $.sidebar.showMods, { name: title })}
        >
          <Text size="sm" truncate="end" fw={lit ? 600 : 400}>
            <bdi>{title}</bdi>
          </Text>
        </UnstyledButton>
      </Tooltip>
      {busy && <Loader size={14} />}
      {clashes.length > 0 && (
        <Tooltip
          label={t(($) => $.sidebar.clashes, { count: clashes.length })}
          multiline
          maw={260}
          withinPortal
        >
          <IconAlertTriangle
            size={16}
            stroke={1.8}
            color="var(--mantine-color-yellow-filled)"
            aria-label={t(($) => $.sidebar.clashes, { count: clashes.length })}
            role="img"
          />
        </Tooltip>
      )}
      <Text size="xs" c="var(--qc-text-2)" className={classes.count}>
        {summary.enabled}
      </Text>
      <GroupRowMenu item={item} rows={rows} readOnly={readOnly} />
    </div>
  )
}
