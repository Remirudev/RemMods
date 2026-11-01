import { Button, Group, Paper, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { modActions, selectedRows } from './mod-actions'
import { useModsUiStore } from './mods-ui-store'
import type { ActionBarProps, ModActionId } from './types'

export function ActionBar({ rows, onToggle }: ActionBarProps) {
  const { t } = useTranslation('mods')
  const selection = useModsUiStore((state) => state.selection)
  const clearSelection = useModsUiStore((state) => state.clearSelection)
  if (selection.length < 2) return null

  const actions = modActions(rows, selection)
  const run = (id: ModActionId) => onToggle(selectedRows(rows, selection), id === 'enable')

  return (
    <Paper withBorder px="md" py="xs" role="toolbar" aria-label={t(($) => $.actionBar.label)}>
      <Group justify="space-between" gap="md" wrap="nowrap">
        <Text size="sm" fw={600}>
          {t(($) => $.actionBar.selected, { count: selection.length })}
        </Text>
        <Group gap="sm" wrap="nowrap">
          {actions.map((action) => (
            <Button
              key={action.id}
              variant="default"
              size="compact-md"
              disabled={action.disabled}
              onClick={() => run(action.id)}
            >
              {action.id === 'enable'
                ? t(($) => $.actionBar.enable)
                : t(($) => $.actionBar.disable)}
            </Button>
          ))}
          <Button variant="subtle" color="gray" size="compact-md" onClick={clearSelection}>
            {t(($) => $.actionBar.clear)}
          </Button>
        </Group>
      </Group>
    </Paper>
  )
}
