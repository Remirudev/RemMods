import { Button, Menu } from '@mantine/core'
import { IconArrowsExchange } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import type { TransferMenuProps } from './types'
import { useTransferProfiles } from './use-profile-mutations'

function todayStamp(): string {
  const now = new Date()
  const pad = (value: number) => String(value).padStart(2, '0')
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
}

export function TransferMenu({ readOnly }: TransferMenuProps) {
  const { t } = useTranslation('profiles')
  const transfer = useTransferProfiles()

  return (
    <Menu position="bottom-end">
      <Menu.Target>
        <Button
          variant="default"
          loading={transfer.isPending}
          leftSection={<IconArrowsExchange size={18} stroke={1.8} />}
        >
          {t(($) => $.transfer.menu)}
        </Button>
      </Menu.Target>
      <Menu.Dropdown>
        <Menu.Item
          disabled={readOnly}
          onClick={() =>
            transfer.mutate({ kind: 'importTmm', title: t(($) => $.transfer.importTmmTitle) })
          }
        >
          {t(($) => $.transfer.importTmm)}
        </Menu.Item>
        <Menu.Item
          disabled={readOnly}
          onClick={() =>
            transfer.mutate({ kind: 'import', title: t(($) => $.transfer.importTitle) })
          }
        >
          {t(($) => $.transfer.import)}
        </Menu.Item>
        <Menu.Item
          onClick={() =>
            transfer.mutate({
              kind: 'export',
              title: t(($) => $.transfer.exportTitle),
              defaultFileName: `remmods-profiles-${todayStamp()}.json`,
            })
          }
        >
          {t(($) => $.transfer.export)}
        </Menu.Item>
      </Menu.Dropdown>
    </Menu>
  )
}
