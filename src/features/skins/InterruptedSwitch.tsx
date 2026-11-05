import { Alert, Button, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { useSwitchSkinSet } from './queries'
import type { InterruptedSwitchProps } from './types'

// The game's files may be a mix of two sets until the switch has run to its end.
export function InterruptedSwitch({ setId, name }: InterruptedSwitchProps) {
  const { t } = useTranslation('skins')
  const finish = useSwitchSkinSet()

  return (
    <Alert color="yellow" p="xs" data-testid="skin-switch-interrupted">
      <Stack gap="xs" align="flex-start">
        <Text size="xs">{t(($) => $.interrupted, { name })}</Text>
        <Button
          size="compact-xs"
          color="yellow"
          variant="light"
          loading={finish.isPending}
          onClick={() => finish.mutate(setId)}
        >
          {t(($) => $.finish)}
        </Button>
      </Stack>
    </Alert>
  )
}
