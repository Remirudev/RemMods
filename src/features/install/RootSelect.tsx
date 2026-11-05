import { Select, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import type { RootKind } from '../../lib/ipc/types'
import type { RootSelectProps } from './types'

const ROOTS: readonly RootKind[] = ['standard', 'legacy', 'logic']

export function RootSelect({ inspection, value, disabled, onChange }: RootSelectProps) {
  const { t } = useTranslation('install')
  const data = ROOTS.map((root) => ({ value: root, label: t(($) => $.root.options[root]) }))

  return (
    <Stack gap={4}>
      <Select
        label={t(($) => $.root.label)}
        data={data}
        value={value}
        allowDeselect={false}
        disabled={disabled}
        onChange={(next) => {
          const root = ROOTS.find((candidate) => candidate === next)
          if (root && root !== value) onChange(root)
        }}
      />
      <Text size="xs" c="var(--qc-text-2)">
        {t(($) => $.root.reasons[inspection.rootReason])}
      </Text>
    </Stack>
  )
}
