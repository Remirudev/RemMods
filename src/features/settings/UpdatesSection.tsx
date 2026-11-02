import { Alert, Button, DataList, Group, Modal, Paper, Stack, Text } from '@mantine/core'
import { modals } from '@mantine/modals'
import { IconAlertTriangle, IconInfoCircle, IconRefresh } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { describeError } from '../../lib/i18n/error-messages'
import { isIpcError } from '../../lib/ipc/ipc-error'
import { useUpdateChannel, useUpdateCheck } from './queries'
import { SectionHeading } from './SectionHeading'
import { UpdateKindNotice } from './UpdateKindNotice'
import { UpdateProgressView } from './UpdateProgressView'
import { UpdateResult } from './UpdateResult'
import { kindNotice, updateOffer } from './update-model'
import { useUpdateInstall } from './use-update-install'

const NO_CLOSE = () => undefined

export function UpdatesSection() {
  const { t } = useTranslation('settings')
  const channel = useUpdateChannel()
  const check = useUpdateCheck()
  const install = useUpdateInstall()
  const info = channel.data
  const offer = info ? updateOffer(info) : null
  const notice = info ? kindNotice(info) : null
  const busy = check.isPending || install.running

  // `stale`: the version asked for is no longer the one the last check found.
  const installError = (() => {
    if (!install.error) return null
    if (isIpcError(install.error) && install.error.appError.kind === 'stale') {
      return { title: t(($) => $.updates.title), message: t(($) => $.updates.staleCheck) }
    }
    return describeError(install.error)
  })()

  const runCheck = () => {
    install.clearError()
    check.mutate()
  }

  const askInstall = (version: string) => {
    const installer = offer?.kind === 'install' && offer.installer
    modals.openConfirmModal({
      title: t(($) => $.updates.confirm.title, { version }),
      children: (
        <Stack gap="xs">
          <Text size="sm">{t(($) => $.updates.confirm.body)}</Text>
          {installer && <Text size="sm">{t(($) => $.updates.confirm.installer)}</Text>}
        </Stack>
      ),
      labels: {
        confirm: t(($) => $.updates.install),
        cancel: t(($) => $.updates.confirm.cancel),
      },
      onConfirm: () => {
        void install.start(version)
      },
    })
  }

  return (
    <Paper withBorder p="lg">
      <Stack gap="lg">
        <SectionHeading
          title={t(($) => $.updates.title)}
          description={t(($) => $.updates.description)}
        />
        <DataList labelWidth={180}>
          <DataList.Item>
            <DataList.ItemLabel>{t(($) => $.updates.labels.installedAs)}</DataList.ItemLabel>
            <DataList.ItemValue>
              {info ? t(($) => $.updates.kinds[info.install]) : t(($) => $.diagnostics.unavailable)}
            </DataList.ItemValue>
          </DataList.Item>
          <DataList.Item>
            <DataList.ItemLabel>{t(($) => $.updates.labels.current)}</DataList.ItemLabel>
            <DataList.ItemValue ff="monospace">
              {info?.current ?? t(($) => $.diagnostics.unavailable)}
            </DataList.ItemValue>
          </DataList.Item>
        </DataList>
        {notice && <UpdateKindNotice notice={notice} />}
        {offer?.kind === 'notConfigured' && (
          <Alert color="gray" icon={<IconInfoCircle size={20} stroke={1.8} />}>
            {t(($) => $.updates.notices.notConfigured)}
          </Alert>
        )}
        <Group>
          <Button
            variant="default"
            leftSection={<IconRefresh size={18} stroke={1.8} />}
            disabled={!info?.configured || install.running}
            loading={check.isPending}
            onClick={runCheck}
          >
            {t(($) => $.updates.check)}
          </Button>
        </Group>
        {check.isError && (
          <Alert
            color="red"
            title={describeError(check.error).title}
            icon={<IconAlertTriangle size={20} stroke={1.8} />}
          >
            {describeError(check.error).message}
          </Alert>
        )}
        {check.data && info && !check.isPending && (
          <UpdateResult
            check={check.data}
            channel={info}
            installing={busy}
            onInstall={askInstall}
          />
        )}
        {installError && (
          <Alert
            color="red"
            title={installError.title}
            icon={<IconAlertTriangle size={20} stroke={1.8} />}
            data-testid="update-install-error"
          >
            {installError.message}
          </Alert>
        )}
      </Stack>
      <Modal
        opened={install.running}
        onClose={NO_CLOSE}
        title={t(($) => $.updates.progress.title)}
        withCloseButton={false}
        closeOnClickOutside={false}
        closeOnEscape={false}
        centered
      >
        <UpdateProgressView progress={install.progress} />
      </Modal>
    </Paper>
  )
}
