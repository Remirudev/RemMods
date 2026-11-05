import { Button, Checkbox, Popover, Stack, Text } from '@mantine/core'
import { IconLayoutColumns, IconRestore } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { HIDEABLE_COLUMN_IDS } from './column-layout'
import { useColumnLayoutStore } from './column-layout-store'

// The dropdown is a dialog named by its button; focus moves into it and Escape returns it.
export function ColumnsMenu() {
  const { t } = useTranslation('mods')
  const visibility = useColumnLayoutStore((state) => state.visibility)
  const setColumnVisible = useColumnLayoutStore((state) => state.setColumnVisible)
  const resetLayout = useColumnLayoutStore((state) => state.resetLayout)

  return (
    <Popover position="bottom-end" shadow="md" trapFocus withinPortal>
      <Popover.Target>
        <Button
          variant="default"
          size="compact-sm"
          leftSection={<IconLayoutColumns size={16} stroke={1.8} />}
        >
          {t(($) => $.columnsMenu.button)}
        </Button>
      </Popover.Target>
      <Popover.Dropdown>
        <Stack gap="xs">
          <Text size="xs" fw={600} c="var(--qc-text-2)">
            {t(($) => $.columnsMenu.title)}
          </Text>
          <Checkbox label={t(($) => $.columns.toggle)} checked disabled readOnly />
          <Checkbox label={t(($) => $.columns.name)} checked disabled readOnly />
          {HIDEABLE_COLUMN_IDS.map((id) => (
            <Checkbox
              key={id}
              label={t(($) => $.columns[id])}
              checked={visibility[id] !== false}
              onChange={(event) => setColumnVisible(id, event.currentTarget.checked)}
            />
          ))}
          <Button
            variant="subtle"
            size="compact-sm"
            leftSection={<IconRestore size={16} stroke={1.8} />}
            onClick={resetLayout}
          >
            {t(($) => $.columnsMenu.reset)}
          </Button>
        </Stack>
      </Popover.Dropdown>
    </Popover>
  )
}
