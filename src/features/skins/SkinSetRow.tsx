import { Button, Text, UnstyledButton } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useTranslation } from 'react-i18next'
import { useSwitchSkinSet } from './queries'
import { openSkinSet } from './skin-modals'
import classes from './skins.module.css'
import type { SkinSetRowProps } from './types'

// The name opens the set; the button puts it into the game.
export function SkinSetRow({ set, active, disabled }: SkinSetRowProps) {
  const { t } = useTranslation('skins')
  const switchSet = useSwitchSkinSet()

  return (
    <div className={classes.row} data-active={active || undefined} data-testid="skin-set-row">
      <UnstyledButton className={classes.name} onClick={() => openSkinSet(set)}>
        <Text size="sm" truncate="end" title={set.name}>
          <bdi>{set.name}</bdi>
        </Text>
        {active && (
          <Text size="xs" c="var(--qc-text-2)">
            {t(($) => $.inGame)}
          </Text>
        )}
      </UnstyledButton>
      {!active && (
        <Button
          size="compact-xs"
          variant="light"
          loading={switchSet.isPending}
          disabled={disabled}
          aria-label={t(($) => $.switchTo, { name: set.name })}
          onClick={() =>
            switchSet.mutate(set.id, {
              onSuccess: () =>
                notifications.show({ message: t(($) => $.switched, { name: set.name }) }),
            })
          }
        >
          {t(($) => $.switch)}
        </Button>
      )}
    </div>
  )
}
