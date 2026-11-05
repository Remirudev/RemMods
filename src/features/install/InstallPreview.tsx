import { Alert, Badge, Button, Group, Loader, Stack, Switch, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { formatSize } from '../../shared/format-size'
import { CandidateList } from './CandidateList'
import { CaseClashPicker } from './CaseClashPicker'
import { EncodingSelect } from './EncodingSelect'
import { IssueList } from './IssueList'
import { needsAutoFix, setAutoFix, showsEncoding, startBlockers } from './install-model'
import { RootSelect } from './RootSelect'
import type { InstallPreviewProps } from './types'

export function InstallPreview({ flow, inspection, choices, onClose }: InstallPreviewProps) {
  const { t, i18n } = useTranslation('install')
  const blockers = startBlockers(inspection, choices)
  const free = formatSize(inspection.availableBytes, i18n.language)
  const size = formatSize(inspection.totalBytes, i18n.language)
  const busy = flow.reinspecting

  return (
    <Stack gap="md" data-testid="install-preview">
      <Stack gap={4}>
        <Group gap="sm" wrap="nowrap">
          <Text fw={600} style={{ overflowWrap: 'anywhere' }}>
            {inspection.sourceName}
          </Text>
          <Badge variant="light">{t(($) => $.formats[inspection.format])}</Badge>
          {busy && <Loader size="xs" />}
        </Group>
        <Text size="sm" c="var(--qc-text-2)">
          {[
            t(($) => $.summary.files, { count: inspection.files }),
            size,
            free ? t(($) => $.summary.free, { size: free }) : t(($) => $.summary.freeUnknown),
          ]
            .filter((part) => part !== null)
            .join(' · ')}
        </Text>
      </Stack>
      {flow.notice && <Alert color="yellow">{t(($) => $.notices[flow.notice ?? 'stale'])}</Alert>}
      <RootSelect
        inspection={inspection}
        value={inspection.root}
        disabled={busy}
        onChange={flow.setRoot}
      />
      {showsEncoding(inspection) && (
        <EncodingSelect value={inspection.encoding} disabled={busy} onChange={flow.setEncoding} />
      )}
      {inspection.candidates.length > 0 && (
        <CandidateList
          inspection={inspection}
          choices={choices}
          onChoices={flow.setChoices}
          onName={flow.setName}
        />
      )}
      {needsAutoFix(inspection) && (
        <Switch
          checked={choices.autoFix}
          onChange={(event) => flow.setChoices(setAutoFix(choices, event.currentTarget.checked))}
          label={t(($) => $.fixes.autoFix)}
          description={t(($) => $.fixes.autoFixHint)}
        />
      )}
      <CaseClashPicker inspection={inspection} choices={choices} onChoices={flow.setChoices} />
      <IssueList issues={inspection.issues} />
      <Stack gap="xs">
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            {t(($) => $.actions.cancel)}
          </Button>
          <Button disabled={blockers.length > 0 || busy} onClick={() => flow.start()}>
            {t(($) => $.actions.install)}
          </Button>
        </Group>
        {blockers.length > 0 && (
          <Stack gap={2} align="flex-end" data-testid="install-blockers">
            {blockers.map((blocker) => (
              <Text key={blocker} size="xs" c="var(--qc-text-2)">
                {t(($) => $.blockers[blocker])}
              </Text>
            ))}
          </Stack>
        )}
      </Stack>
    </Stack>
  )
}
