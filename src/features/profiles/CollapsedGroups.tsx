import { ActionIcon, ScrollArea, Stack, Tooltip } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { groupInitials } from './group-state'
import classes from './groups.module.css'
import type { CollapsedGroupsProps } from './types'

// The folded sidebar: one round button per group, filled while the group is on.
export function CollapsedGroups({ items, busyId, readOnly, onSwitch }: CollapsedGroupsProps) {
  const { t } = useTranslation('profiles')

  return (
    <ScrollArea className={classes.list} type="hover">
      <Stack gap={6} align="center" py={2}>
        {items.map((item) => (
          <Tooltip key={item.summary.profile.id} label={item.title} position="right" withinPortal>
            <ActionIcon
              radius="xl"
              size="lg"
              variant={item.lit ? 'filled' : 'default'}
              loading={busyId === item.summary.profile.id}
              disabled={readOnly}
              aria-pressed={item.lit}
              aria-label={
                item.lit
                  ? t(($) => $.sidebar.switchOff, { name: item.title })
                  : t(($) => $.sidebar.switchOn, { name: item.title })
              }
              onClick={() => onSwitch(item)}
              data-testid="group-chip"
            >
              <span className={classes.initials}>{groupInitials(item.title)}</span>
            </ActionIcon>
          </Tooltip>
        ))}
      </Stack>
    </ScrollArea>
  )
}
