import { Kbd, Menu } from '@mantine/core'
import { IconPencil, IconTrash, IconTrashX } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { trashPlaceWord } from '../../shared/trash-place'
import { canManage, rowSubject } from './manage-model'
import type { RowMenuProps } from './types'

// The same actions behind the three dots and a right-click.
export function RowMenuItems({ row, onManage }: RowMenuProps) {
  const { t } = useTranslation('mods')
  const subject = rowSubject(row)

  return (
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
        {t(($) => $.rowMenu.trash, { place: trashPlaceWord() })}
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
  )
}
