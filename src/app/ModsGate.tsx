import { Alert, Button, Center, Loader, Stack, Text } from '@mantine/core'
import { IconAlertCircle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { ModsPanel } from '../features/mods/ModsPanel'
import { OnboardingPanel } from '../features/onboarding/OnboardingPanel'
import { GroupsSidebar } from '../features/profiles/GroupsSidebar'
import { SkinSetsSection } from '../features/skins/SkinSetsSection'
import { describeError } from '../lib/i18n/error-messages'
import type { ModRow } from '../lib/ipc/types'
import { useGameStatus } from '../shared/game/queries'

function groupsSidebar(rows: readonly ModRow[]) {
  return <GroupsSidebar rows={rows} footer={<SkinSetsSection />} />
}

// The mods panel only exists while a game folder is set; every other state is onboarding. The
// groups sidebar and the skin sets come from their own features, so the app puts them together.
export function ModsGate() {
  const { t } = useTranslation('game')
  const { t: tMods } = useTranslation('mods')
  const status = useGameStatus()

  if (status.isPending) {
    return (
      <Center h="100%">
        <Stack align="center" gap="sm">
          <Loader size="md" aria-label={t(($) => $.detecting)} />
          <Text size="sm" c="var(--qc-text-2)">
            {t(($) => $.detecting)}
          </Text>
        </Stack>
      </Center>
    )
  }

  if (status.isError) {
    const { title, message } = describeError(status.error)
    return (
      <Alert color="red" icon={<IconAlertCircle size={20} stroke={1.8} />} title={title}>
        <Stack gap="sm" align="flex-start">
          <Text size="sm">{message}</Text>
          <Button variant="default" onClick={() => void status.refetch()}>
            {tMods(($) => $.error.retry)}
          </Button>
        </Stack>
      </Alert>
    )
  }

  const data = status.data
  if (data.location.kind === 'ready')
    return <ModsPanel status={data} install={data.location.install} aside={groupsSidebar} />
  return <OnboardingPanel status={data} />
}
