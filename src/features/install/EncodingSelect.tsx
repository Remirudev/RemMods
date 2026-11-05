import { Select, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import type { NameEncoding } from '../../lib/ipc/types'
import type { EncodingSelectProps } from './types'

const ENCODINGS: readonly NameEncoding[] = [
  'auto',
  'shiftJis',
  'gbk',
  'big5',
  'eucKr',
  'ibm866',
  'windows1251',
]

export function EncodingSelect({ value, disabled, onChange }: EncodingSelectProps) {
  const { t } = useTranslation('install')
  const data = ENCODINGS.map((encoding) => ({
    value: encoding,
    label: t(($) => $.encoding.options[encoding]),
  }))

  return (
    <Stack gap={4}>
      <Select
        label={t(($) => $.encoding.label)}
        data={data}
        value={value}
        allowDeselect={false}
        disabled={disabled}
        onChange={(next) => {
          const encoding = ENCODINGS.find((candidate) => candidate === next)
          if (encoding && encoding !== value) onChange(encoding)
        }}
      />
      <Text size="xs" c="var(--qc-text-2)">
        {t(($) => $.encoding.hint)}
      </Text>
    </Stack>
  )
}
