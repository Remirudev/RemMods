import { Button, CloseButton, Group, Text, Tooltip } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import type { GroupBulkBarProps } from './types'

// Switches every group at once, or the selection while several groups are picked.
export function GroupBulkBar({ selected, disabled, onSwitch, onClear }: GroupBulkBarProps) {
  const { t } = useTranslation('profiles')
  const several = selected > 1
  const onHint = several
    ? t(($) => $.sidebar.selectedOnHint, { amount: selected })
    : t(($) => $.sidebar.allOnHint)
  const offHint = several
    ? t(($) => $.sidebar.selectedOffHint, { amount: selected })
    : t(($) => $.sidebar.allOffHint)

  return (
    <Group gap={4} wrap="nowrap" mb={6} data-testid="group-bulk">
      <Tooltip label={onHint} withinPortal>
        <Button
          size="compact-xs"
          variant="default"
          aria-label={onHint}
          disabled={disabled}
          onClick={() => onSwitch('on')}
        >
          {several ? t(($) => $.sidebar.selectedOn) : t(($) => $.sidebar.allOn)}
        </Button>
      </Tooltip>
      <Tooltip label={offHint} withinPortal>
        <Button
          size="compact-xs"
          variant="default"
          aria-label={offHint}
          disabled={disabled}
          onClick={() => onSwitch('off')}
        >
          {several ? t(($) => $.sidebar.selectedOff) : t(($) => $.sidebar.allOff)}
        </Button>
      </Tooltip>
      {several && (
        <>
          <Text size="xs" c="var(--qc-text-2)" ml={4} data-testid="group-selected">
            {t(($) => $.sidebar.selected, { amount: selected })}
          </Text>
          <CloseButton
            size="xs"
            aria-label={t(($) => $.sidebar.clearSelection)}
            onClick={onClear}
          />
        </>
      )}
    </Group>
  )
}
