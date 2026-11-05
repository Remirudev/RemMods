import { Button, Menu } from '@mantine/core'
import { IconArchive, IconChevronDown, IconDownload, IconFolder } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { pickArchivesToQueue, pickFolderToQueue } from '../../shared/install-actions'

export function InstallMenu() {
  const { t } = useTranslation('install')

  return (
    <Menu position="bottom-end" withinPortal>
      <Menu.Target>
        <Button
          variant="default"
          leftSection={<IconDownload size={18} stroke={1.8} />}
          rightSection={<IconChevronDown size={16} stroke={1.8} />}
        >
          {t(($) => $.menu.button)}
        </Button>
      </Menu.Target>
      <Menu.Dropdown>
        <Menu.Item
          leftSection={<IconArchive size={16} stroke={1.8} />}
          onClick={() => void pickArchivesToQueue(t(($) => $.pick.archives))}
        >
          {t(($) => $.menu.archives)}
        </Menu.Item>
        <Menu.Item
          leftSection={<IconFolder size={16} stroke={1.8} />}
          onClick={() => void pickFolderToQueue(t(($) => $.pick.folder))}
        >
          {t(($) => $.menu.folder)}
        </Menu.Item>
      </Menu.Dropdown>
    </Menu>
  )
}
