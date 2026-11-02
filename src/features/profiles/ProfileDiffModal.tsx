import { Alert, Button, Group, Loader, ScrollArea, Stack, Text } from '@mantine/core'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { i18n } from '../../lib/i18n'
import { isIpcError } from '../../lib/ipc/ipc-error'
import { confirmApplyWhileRunning } from '../../shared/fs-dialogs'
import { routeFsError } from '../../shared/fs-error-route'
import { useModalLock } from '../../shared/use-modal-lock'
import { DiffSection } from './DiffSection'
import { groupDiff, keyName } from './diff-groups'
import { useProfileDiff, useProfiles } from './queries'
import type { ProfileDiffModalProps } from './types'
import { useApplyProfile } from './use-profile-mutations'

export function ProfileDiffModal({ profile, onClose, modalId }: ProfileDiffModalProps) {
  const { t } = useTranslation('profiles')
  const diff = useProfileDiff(profile.id)
  const readOnly = useProfiles().data?.readOnly ?? false
  const apply = useApplyProfile()
  const [changedNotice, setChangedNotice] = useState(false)
  const data = diff.data
  const groups = data ? groupDiff(data) : null
  useModalLock(modalId, apply.isPending)

  const run = (applyWhileRunning: boolean) => {
    if (!data) return
    apply.mutate(
      { profileId: profile.id, fingerprint: data.fingerprint, applyWhileRunning },
      {
        onSuccess: onClose,
        onError: (error) => {
          const kind = isIpcError(error) ? error.appError.kind : null
          if (kind === 'stale') {
            // The disk changed since the diff was shown: show the new diff, apply nothing.
            setChangedNotice(true)
            void diff.refetch()
          } else if (kind === 'gameRunning') {
            void confirmApplyWhileRunning(i18n.getFixedT(null, 'errors')).then((confirmed) => {
              if (confirmed) run(true)
            })
          } else {
            routeFsError(error)
          }
        },
      },
    )
  }

  if (diff.isPending) {
    return (
      <Group justify="center" py="lg">
        <Loader size="sm" />
      </Group>
    )
  }
  if (!data || !groups) {
    return (
      <Alert color="red" role="alert">
        {t(($) => $.diff.error)}
      </Alert>
    )
  }

  return (
    <Stack gap="sm" data-testid="profile-diff">
      <Text size="sm">{t(($) => $.mode[data.mode].hint)}</Text>
      {changedNotice && (
        <Alert color="yellow" role="status">
          {t(($) => $.diff.changed)}
        </Alert>
      )}
      <ScrollArea.Autosize mah={360}>
        <Stack gap="sm">
          <DiffSection
            title={t(($) => $.diff.enable, { count: groups.enable.length })}
            count={groups.enable.length}
          >
            {groups.enable.map((change) => (
              <Text key={change.key} size="sm">
                <bdi>{change.name}</bdi>
              </Text>
            ))}
          </DiffSection>
          <DiffSection
            title={t(($) => $.diff.disable, { count: groups.disable.length })}
            count={groups.disable.length}
          >
            {groups.disable.map((change) => (
              <Text key={change.key} size="sm">
                <bdi>{change.name}</bdi>
                {change.reason === 'unlisted' && (
                  <Text span size="xs" c="var(--qc-text-2)">
                    {' · '}
                    {t(($) => $.diff.notInProfile)}
                  </Text>
                )}
              </Text>
            ))}
          </DiffSection>
          <DiffSection
            title={t(($) => $.diff.skipped, { count: groups.skipped.length })}
            count={groups.skipped.length}
          >
            {groups.skipped.map((skip) => (
              <Text key={skip.key} size="sm">
                <bdi>{skip.name ?? keyName(skip.key)}</bdi>
                <Text span size="xs" c="var(--qc-text-2)">
                  {' · '}
                  {t(($) => $.diff.skipReason[skip.reason])}
                </Text>
              </Text>
            ))}
          </DiffSection>
          <DiffSection
            title={t(($) => $.diff.missing, { count: groups.missing.length })}
            count={groups.missing.length}
          >
            {groups.missing.map((key) => (
              <Text key={key} size="sm">
                <bdi>{keyName(key)}</bdi>
              </Text>
            ))}
          </DiffSection>
          {!groups.hasChanges && <Text size="sm">{t(($) => $.diff.nothing)}</Text>}
        </Stack>
      </ScrollArea.Autosize>
      <Group justify="flex-end">
        <Button variant="default" onClick={onClose} disabled={apply.isPending}>
          {t(($) => $.diff.cancel)}
        </Button>
        <Button
          onClick={() => run(false)}
          loading={apply.isPending}
          disabled={!groups.hasChanges || readOnly || diff.isFetching}
        >
          {t(($) => $.diff.apply)}
        </Button>
      </Group>
    </Stack>
  )
}
