import { Button, Group } from '@mantine/core'
import { IconPencil, IconTrash, IconTrashX } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { trashPlaceWord } from '../../shared/trash-place'
import { canManage, rowSubject } from './manage-model'
import type { DrawerActionsProps } from './types'
import { useManageActions } from './use-manage-actions'

export function DrawerActions({ row }: DrawerActionsProps) {
  const { t } = useTranslation('mods')
  const manage = useManageActions()
  const subject = rowSubject(row)

  return (
    <Group gap="sm">
      <Button
        variant="default"
        leftSection={<IconPencil size={18} stroke={1.8} />}
        disabled={!canManage(subject, 'rename')}
        onClick={() => manage.rename(subject)}
      >
        {t(($) => $.rowMenu.rename)}
      </Button>
      <Button
        variant="default"
        leftSection={<IconTrash size={18} stroke={1.8} />}
        disabled={!canManage(subject, 'trash')}
        onClick={() => manage.trash(subject)}
      >
        {t(($) => $.rowMenu.trash, { place: trashPlaceWord() })}
      </Button>
      <Button
        variant="default"
        color="red"
        leftSection={<IconTrashX size={18} stroke={1.8} />}
        disabled={!canManage(subject, 'delete')}
        onClick={() => manage.remove(subject)}
      >
        {t(($) => $.rowMenu.delete)}
      </Button>
    </Group>
  )
}
