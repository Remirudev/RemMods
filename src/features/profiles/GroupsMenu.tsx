import { ActionIcon, Menu } from '@mantine/core'
import {
  IconBolt,
  IconDeviceFloppy,
  IconDotsVertical,
  IconHistory,
  IconListCheck,
} from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { useGroupPrefs } from './group-prefs-store'
import { openSaveCurrent, openSnapshots } from './profile-modals'
import { TransferItems } from './TransferItems'
import type { GroupsMenuProps } from './types'

// The change-list item names what it does next, like the rail's theme button, because a menu
// item cannot carry a checked state.
export function GroupsMenu({ readOnly }: GroupsMenuProps) {
  const { t } = useTranslation('profiles')
  const ask = useGroupPrefs((state) => state.askBeforeSwitch)
  const setAsk = useGroupPrefs((state) => state.setAskBeforeSwitch)

  return (
    <Menu position="bottom-end" withinPortal>
      <Menu.Target>
        <ActionIcon variant="subtle" color="gray" aria-label={t(($) => $.sidebar.menu)}>
          <IconDotsVertical size={18} stroke={1.8} />
        </ActionIcon>
      </Menu.Target>
      <Menu.Dropdown>
        <Menu.Item
          leftSection={<IconDeviceFloppy size={16} stroke={1.8} />}
          disabled={readOnly}
          onClick={openSaveCurrent}
        >
          {t(($) => $.menu.saveCurrent)}
        </Menu.Item>
        <Menu.Item
          leftSection={
            ask ? <IconBolt size={16} stroke={1.8} /> : <IconListCheck size={16} stroke={1.8} />
          }
          onClick={() => setAsk(!ask)}
        >
          {ask ? t(($) => $.menu.switchWithoutList) : t(($) => $.menu.showListFirst)}
        </Menu.Item>
        <Menu.Item leftSection={<IconHistory size={16} stroke={1.8} />} onClick={openSnapshots}>
          {t(($) => $.menu.snapshots)}
        </Menu.Item>
        <Menu.Divider />
        <TransferItems readOnly={readOnly} />
      </Menu.Dropdown>
    </Menu>
  )
}
