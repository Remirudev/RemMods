import { ActionIcon, Box, Kbd, Menu } from '@mantine/core'
import { IconDots, IconPencil, IconTrash, IconTrashX } from '@tabler/icons-react'
import type { MouseEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { canManage, rowSubject } from './manage-model'
import type { RowMenuProps } from './types'

export function RowMenu({ row, onManage }: RowMenuProps) {
  const { t } = useTranslation('mods')
  const subject = rowSubject(row)

  // Like the switch cell: using the menu never opens the drawer or moves the selection.
  return (
    <Box
      component="span"
      display="inline-flex"
      data-testid="row-menu"
      onClick={(event: MouseEvent) => event.stopPropagation()}
    >
      <Menu position="bottom-end" withinPortal>
        <Menu.Target>
          <ActionIcon
            variant="subtle"
            color="gray"
            aria-label={t(($) => $.rowMenu.label, { name: row.name })}
          >
            <IconDots size={18} stroke={1.8} />
          </ActionIcon>
        </Menu.Target>
        <Menu.Dropdown>
          <Menu.Item
            leftSection={<IconPencil size={16} stroke={1.8} />}
            rightSection={<Kbd size="xs">F2</Kbd>}
            disabled={!canManage(subject, 'rename')}
            onClick={() => onManage('rename', row)}
          >
            {t(($) => $.rowMenu.rename)}
          </Menu.Item>
          <Menu.Item
            leftSection={<IconTrash size={16} stroke={1.8} />}
            rightSection={<Kbd size="xs">Del</Kbd>}
            disabled={!canManage(subject, 'trash')}
            onClick={() => onManage('trash', row)}
          >
            {t(($) => $.rowMenu.trash)}
          </Menu.Item>
          <Menu.Item
            color="red"
            leftSection={<IconTrashX size={16} stroke={1.8} />}
            rightSection={<Kbd size="xs">⇧Del</Kbd>}
            disabled={!canManage(subject, 'delete')}
            onClick={() => onManage('delete', row)}
          >
            {t(($) => $.rowMenu.delete)}
          </Menu.Item>
        </Menu.Dropdown>
      </Menu>
    </Box>
  )
}
