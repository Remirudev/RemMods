import { Menu } from '@mantine/core'
import { IconDownload, IconFileImport, IconUpload } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import type { TransferItemsProps } from './types'
import { useTransferProfiles } from './use-profile-mutations'

function todayStamp(): string {
  const now = new Date()
  const pad = (value: number) => String(value).padStart(2, '0')
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
}

// Import and export, as items of the groups menu.
export function TransferItems({ readOnly }: TransferItemsProps) {
  const { t } = useTranslation('profiles')
  const transfer = useTransferProfiles()
  const busy = transfer.isPending

  return (
    <>
      <Menu.Item
        leftSection={<IconFileImport size={16} stroke={1.8} />}
        disabled={readOnly || busy}
        onClick={() =>
          transfer.mutate({
            kind: 'importOtherManager',
            title: t(($) => $.transfer.importOtherManagerTitle),
          })
        }
      >
        {t(($) => $.transfer.importOtherManager)}
      </Menu.Item>
      <Menu.Item
        leftSection={<IconDownload size={16} stroke={1.8} />}
        disabled={readOnly || busy}
        onClick={() => transfer.mutate({ kind: 'import', title: t(($) => $.transfer.importTitle) })}
      >
        {t(($) => $.transfer.import)}
      </Menu.Item>
      <Menu.Item
        leftSection={<IconUpload size={16} stroke={1.8} />}
        disabled={busy}
        onClick={() =>
          transfer.mutate({
            kind: 'export',
            title: t(($) => $.transfer.exportTitle),
            defaultFileName: `remmods-groups-${todayStamp()}.json`,
          })
        }
      >
        {t(($) => $.transfer.export)}
      </Menu.Item>
    </>
  )
}
