import { Group, Loader, Modal, ScrollArea, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { InstallFailure } from './InstallFailure'
import { InstallPreview } from './InstallPreview'
import { InstallProgressView } from './InstallProgressView'
import { InstallResult } from './InstallResult'
import classes from './install.module.css'
import { RunningConfirm } from './RunningConfirm'
import type { InstallModalProps } from './types'
import { useInstallFlow } from './use-install-flow'

export function InstallModal({ source, remaining, onClose, onStop }: InstallModalProps) {
  const { t } = useTranslation('install')
  const flow = useInstallFlow(source)
  const { stage, inspection, choices } = flow
  const running = stage === 'running'
  const title = t(($) => $.modal.titleFor, { name: source.name })

  return (
    <Modal
      opened
      onClose={onClose}
      size="lg"
      centered
      title={
        <Group gap="sm">
          <Text fw={600}>{title}</Text>
          {remaining > 0 && (
            <Text size="xs" c="var(--qc-text-2)">
              {t(($) => $.modal.waiting, { count: remaining })}
            </Text>
          )}
        </Group>
      }
      closeOnClickOutside={!running}
      closeOnEscape={!running}
      withCloseButton={!running}
      scrollAreaComponent={ScrollArea.Autosize}
      classNames={{ content: classes.chrome }}
    >
      {stage === 'inspecting' && (
        <Group gap="sm" data-testid="install-inspecting">
          <Loader size="sm" />
          <Text size="sm">{t(($) => $.inspecting, { name: source.name })}</Text>
        </Group>
      )}
      {stage === 'preview' && inspection && choices && (
        <InstallPreview flow={flow} inspection={inspection} choices={choices} onClose={onClose} />
      )}
      {stage === 'confirmRunning' && (
        <RunningConfirm onConfirm={() => flow.start(true)} onBack={flow.backToPreview} />
      )}
      {running && <InstallProgressView progress={flow.progress} onCancel={flow.cancel} />}
      {stage === 'done' && (
        <InstallResult
          outcome={flow.outcome}
          remaining={remaining}
          onClose={onClose}
          onStop={onStop}
        />
      )}
      {stage === 'cancelled' && (
        <InstallResult outcome={null} remaining={remaining} onClose={onClose} onStop={onStop} />
      )}
      {stage === 'failed' && (
        <InstallFailure
          error={flow.error}
          canGoBack={inspection !== null && choices !== null}
          remaining={remaining}
          onBack={flow.backToPreview}
          onClose={onClose}
          onStop={onStop}
        />
      )}
    </Modal>
  )
}
