import { ActionIcon, Menu } from '@mantine/core'
import { IconDots, IconPencil, IconRefresh, IconTrash } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { confirmProfile, openGroupDialog } from './profile-modals'
import type { GroupRowMenuProps } from './types'
import { useDeleteProfile, useSaveProfile } from './use-profile-mutations'

export function GroupRowMenu({ item, rows, readOnly }: GroupRowMenuProps) {
  const { t } = useTranslation('profiles')
  const save = useSaveProfile()
  const remove = useDeleteProfile()
  const { profile } = item.summary
  const name = item.title

  const confirmUpdate = () =>
    confirmProfile(
      t(($) => $.row.updateTitle),
      t(($) => $.row.updateBody, { name }),
      t(($) => $.row.update),
      () => save.mutate({ kind: 'overwrite', id: profile.id }),
    )
  const confirmDelete = () =>
    confirmProfile(
      t(($) => $.row.deleteTitle),
      t(($) => $.row.deleteBody, { name }),
      t(($) => $.row.delete),
      () => remove.mutate(profile.id),
      true,
    )

  return (
    <Menu position="bottom-end" withinPortal>
      <Menu.Target>
        <ActionIcon
          variant="subtle"
          color="gray"
          size="sm"
          aria-label={t(($) => $.sidebar.rowMenu, { name })}
        >
          <IconDots size={16} stroke={1.8} />
        </ActionIcon>
      </Menu.Target>
      <Menu.Dropdown>
        <Menu.Item
          leftSection={<IconPencil size={16} stroke={1.8} />}
          disabled={readOnly}
          onClick={() => openGroupDialog(profile, rows)}
        >
          {t(($) => $.row.edit)}
        </Menu.Item>
        <Menu.Item
          leftSection={<IconRefresh size={16} stroke={1.8} />}
          disabled={readOnly}
          onClick={confirmUpdate}
        >
          {t(($) => $.row.update)}
        </Menu.Item>
        <Menu.Divider />
        <Menu.Item
          color="red"
          leftSection={<IconTrash size={16} stroke={1.8} />}
          disabled={readOnly}
          onClick={confirmDelete}
        >
          {t(($) => $.row.delete)}
        </Menu.Item>
      </Menu.Dropdown>
    </Menu>
  )
}
