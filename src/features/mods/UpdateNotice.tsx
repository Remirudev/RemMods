import { Alert, Button, Group } from '@mantine/core'
import { IconInfoCircle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { useAckUpdate } from '../../shared/game/queries'
import { DisableAllButton } from './DisableAllButton'
import type { UpdateNoticeProps } from './types'

export function UpdateNotice({ update }: UpdateNoticeProps) {
  const { t } = useTranslation('game')
  const acknowledge = useAckUpdate()

  return (
    <Alert
      color="yellow"
      icon={<IconInfoCircle size={20} stroke={1.8} />}
      title={t(($) => $.update.title)}
    >
      <Group justify="space-between" align="center" wrap="nowrap" gap="md">
        {t(($) => $.update.body, { from: update.from, to: update.to })}
        <Group gap="sm" wrap="nowrap">
          <DisableAllButton />
          <Button
            variant="default"
            size="compact-md"
            loading={acknowledge.isPending}
            onClick={() => acknowledge.mutate()}
          >
            {t(($) => $.update.acknowledge)}
          </Button>
        </Group>
      </Group>
    </Alert>
  )
}
