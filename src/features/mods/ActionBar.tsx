import { Button, Group, Paper, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { trashPlaceWord } from '../../shared/trash-place'
import { modActions, selectedRows } from './mod-actions'
import { useModsUiStore } from './mods-ui-store'
import type { ActionBarProps, ModAction } from './types'

export function ActionBar({ rows, onToggle, onTrash, onDelete }: ActionBarProps) {
  const { t } = useTranslation('mods')
  const selection = useModsUiStore((state) => state.selection)
  const clearSelection = useModsUiStore((state) => state.clearSelection)
  if (selection.length < 2) return null

  const actions = modActions(rows, selection)
  const place = trashPlaceWord()
  const run = (action: ModAction) => {
    const selected = selectedRows(rows, selection)
    if (action.id === 'trash') onTrash(selected)
    else if (action.id === 'delete') onDelete(selected)
    else onToggle(selected, action.id === 'enable')
  }
  const label = (action: ModAction) => {
    switch (action.id) {
      case 'enable':
        return t(($) => $.actionBar.enable)
      case 'disable':
        return t(($) => $.actionBar.disable)
      case 'trash':
        return t(($) => $.actionBar.trash, { count: action.count, place })
      case 'delete':
        return t(($) => $.actionBar.delete, { count: action.count })
    }
  }

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
              variant={action.id === 'delete' ? 'subtle' : 'default'}
              color={action.id === 'delete' ? 'red' : undefined}
              size="compact-md"
              disabled={action.disabled}
              onClick={() => run(action)}
            >
              {label(action)}
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
