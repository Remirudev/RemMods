import { Alert, Center, Loader, Modal, ScrollArea, Tabs, Text } from '@mantine/core'
import { IconAlertTriangle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { describeError } from '../../lib/i18n/error-messages'
import type { Notices } from '../../lib/ipc/types'
import { useNotices } from './queries'
import classes from './settings.module.css'
import type { NoticesModalProps } from './types'

const TABS: readonly (keyof Notices)[] = ['rust', 'npm']

// The texts are compiled into the binary; they are fetched only once the dialog opens.
export function NoticesModal({ opened, onClose }: NoticesModalProps) {
  const { t } = useTranslation('settings')
  const notices = useNotices(opened)
  const labels: Record<keyof Notices, string> = {
    rust: t(($) => $.about.notices.tabs.rust),
    npm: t(($) => $.about.notices.tabs.npm),
  }

  return (
    <Modal opened={opened} onClose={onClose} size="xl" title={t(($) => $.about.notices.title)}>
      {notices.isPending && (
        <Center py="xl">
          <Loader size="sm" />
        </Center>
      )}
      {notices.isError && (
        <Alert
          color="red"
          title={t(($) => $.about.notices.loadFailed)}
          icon={<IconAlertTriangle size={20} stroke={1.8} />}
        >
          {describeError(notices.error).message}
        </Alert>
      )}
      {notices.data && (
        <Tabs defaultValue="rust">
          <Tabs.List>
            {TABS.map((tab) => (
              <Tabs.Tab key={tab} value={tab}>
                {labels[tab]}
              </Tabs.Tab>
            ))}
          </Tabs.List>
          {TABS.map((tab) => (
            <Tabs.Panel key={tab} value={tab} pt="md">
              <ScrollArea h="60vh" data-testid="notice-scroll">
                <Text component="pre" size="xs" ff="monospace" className={classes.notice}>
                  {notices.data[tab]}
                </Text>
              </ScrollArea>
            </Tabs.Panel>
          ))}
        </Tabs>
      )}
    </Modal>
  )
}
