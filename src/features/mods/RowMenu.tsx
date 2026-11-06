import { ActionIcon, Box, Menu } from '@mantine/core'
import { IconDots } from '@tabler/icons-react'
import type { MouseEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { RowMenuItems } from './RowMenuItems'
import type { RowMenuProps } from './types'

export function RowMenu({ row, onManage }: RowMenuProps) {
  const { t } = useTranslation('mods')

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
        <RowMenuItems row={row} onManage={onManage} />
      </Menu>
    </Box>
  )
}
