import { Button, Code, Group, Paper, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import type { InstalledMod } from '../../lib/ipc/types'
import type { InstallResultProps } from './types'

function ReplacedNote({ mod }: { mod: InstalledMod }) {
  const { t } = useTranslation('install')
  if (!mod.replaced) return null
  return (
    <Text size="xs" c="var(--qc-text-2)">
      {mod.replaced.kind === 'trashed'
        ? t(($) => $.result.replacedTrashed)
        : t(($) => $.result.replacedKept)}
    </Text>
  )
}

export function InstallResult({ outcome, remaining, onClose, onStop }: InstallResultProps) {
  const { t } = useTranslation('install')

  return (
    <Stack gap="md" data-testid="install-result">
      <Text fw={600}>
        {outcome
          ? t(($) => $.result.installed, { count: outcome.installed.length })
          : t(($) => $.result.cancelled)}
      </Text>
      {outcome?.installed.map((mod) => (
        <Paper key={mod.key} withBorder p="sm">
          <Stack gap={2}>
            <Group justify="space-between" wrap="nowrap">
              <Code>
                <bdi>{mod.rel}</bdi>
              </Code>
              <Text size="xs" c="var(--qc-text-2)">
                {t(($) => $.result.files, { count: mod.files })}
              </Text>
            </Group>
            <ReplacedNote mod={mod} />
          </Stack>
        </Paper>
      ))}
      <Group justify="flex-end">
        <Button variant={remaining > 0 ? 'default' : 'filled'} onClick={onStop}>
          {t(($) => $.actions.close)}
        </Button>
        {remaining > 0 && <Button onClick={onClose}>{t(($) => $.actions.next)}</Button>}
      </Group>
    </Stack>
  )
}
