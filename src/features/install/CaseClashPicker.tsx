import { Code, Radio, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { keepClashPath } from './install-model'
import type { CaseClashPickerProps } from './types'

export function CaseClashPicker({ inspection, choices, onChoices }: CaseClashPickerProps) {
  const { t } = useTranslation('install')
  if (inspection.clashes.length === 0) return null

  return (
    <Stack gap="xs" data-testid="case-clashes">
      <Text fw={600} size="sm">
        {t(($) => $.clashes.title)}
      </Text>
      <Text size="xs" c="var(--qc-text-2)">
        {t(($) => $.clashes.hint)}
      </Text>
      {inspection.clashes.map((clash) => (
        <Radio.Group
          key={clash.id}
          value={String(choices.clashKeeps[clash.id] ?? '')}
          onChange={(value) => onChoices(keepClashPath(choices, clash.id, Number(value)))}
        >
          <Stack gap={4}>
            {clash.paths.map((path, index) => (
              <Radio
                key={path}
                value={String(index)}
                label={
                  <Code>
                    <bdi>{path}</bdi>
                  </Code>
                }
              />
            ))}
          </Stack>
        </Radio.Group>
      ))}
    </Stack>
  )
}
