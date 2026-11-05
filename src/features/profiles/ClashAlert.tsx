import { Alert, List, Text } from '@mantine/core'
import { IconAlertTriangle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import type { ClashAlertProps } from './types'

const SHOWN = 5

// Mods the group switches on together although they replace the same files.
export function ClashAlert({ clashes }: ClashAlertProps) {
  const { t } = useTranslation('profiles')
  if (clashes.length === 0) return null
  const more = clashes.length - SHOWN

  return (
    <Alert
      color="yellow"
      icon={<IconAlertTriangle size={20} stroke={1.8} />}
      title={t(($) => $.group.clashTitle)}
      data-testid="clash-alert"
    >
      <List size="sm" spacing={2}>
        {clashes.slice(0, SHOWN).map((clash) => (
          <List.Item key={`${clash.first}\n${clash.second}`}>
            <bdi>{t(($) => $.group.clashPair, { first: clash.first, second: clash.second })}</bdi>
          </List.Item>
        ))}
      </List>
      {more > 0 && (
        <Text size="sm" mt={4}>
          {t(($) => $.group.clashMore, { count: more })}
        </Text>
      )}
    </Alert>
  )
}
