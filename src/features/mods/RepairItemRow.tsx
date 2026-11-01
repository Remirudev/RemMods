import { Badge, Button, Code, Group, Paper, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { ipc } from '../../lib/ipc'
import type { RepairAction } from '../../lib/ipc/types'
import { useRevealEntry } from './queries'
import { isActionAvailable, planRepairAction } from './repair-model'
import type { ManageSubject, RepairItemRowProps } from './types'
import { useManageActions } from './use-manage-actions'
import { useToggleMods } from './use-toggle-mods'

function subjectOf(item: RepairItemRowProps['item']): ManageSubject | null {
  if (item.target === null) return null
  return {
    target: item.target,
    name: item.label,
    sizeBytes: null,
    kind: item.kind === 'leftover' ? 'leftover' : 'folder',
  }
}

export function RepairItemRow({ item }: RepairItemRowProps) {
  const { t } = useTranslation('mods')
  const manage = useManageActions()
  const toggle = useToggleMods()
  const reveal = useRevealEntry()
  const subject = subjectOf(item)

  const run = (action: RepairAction) => {
    const op = planRepairAction(item, action)
    if (op.kind === 'logs') {
      void ipc.openLogs()
      return
    }
    if (subject === null) return
    switch (op.kind) {
      case 'rename':
        manage.rename(subject, op.suggestedName)
        return
      case 'trash':
        manage.trash(subject)
        return
      case 'delete':
        manage.remove(subject)
        return
      case 'reveal':
        reveal.mutate(subject.target.modRef)
        return
      case 'toggle':
        toggle.mutate({ targets: [subject.target], enable: op.enable, applyWhileRunning: false })
        return
    }
  }

  return (
    <Paper withBorder px="md" py="xs" data-testid="repair-item">
      <Group justify="space-between" align="flex-start" wrap="nowrap" gap="md">
        <Stack gap={4} flex={1} miw={0}>
          <Group gap="xs">
            <Badge variant="light" size="sm" color="gray">
              {t(($) => $.repair.kinds[item.kind])}
            </Badge>
          </Group>
          {item.kind !== 'stuckJournal' && (
            <Code style={{ overflowWrap: 'anywhere' }}>
              <bdi>{item.label}</bdi>
            </Code>
          )}
          <Text size="xs" c="var(--qc-text-2)">
            {t(($) => $.repair.hints[item.kind])}
          </Text>
        </Stack>
        <Group gap="xs" wrap="wrap" justify="flex-end">
          {item.actions
            .filter((action) => isActionAvailable(item, action))
            .map((action) => (
              <Button
                key={action}
                variant="default"
                size="compact-md"
                color={action === 'deletePermanently' ? 'red' : undefined}
                onClick={() => run(action)}
              >
                {t(($) => $.repair.actions[action])}
              </Button>
            ))}
        </Group>
      </Group>
    </Paper>
  )
}
