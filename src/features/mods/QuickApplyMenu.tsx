import { Button, Menu } from '@mantine/core'
import { IconChevronDown } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { QuickApplyItems } from './QuickApplyItems'

// "Apply profile" in the Mods header: the items load the profile list when the menu opens.
export function QuickApplyMenu() {
  const { t } = useTranslation('mods')

  return (
    <Menu position="bottom-end" withinPortal>
      <Menu.Target>
        <Button variant="default" rightSection={<IconChevronDown size={16} stroke={1.8} />}>
          {t(($) => $.quickApply.button)}
        </Button>
      </Menu.Target>
      <Menu.Dropdown miw={240}>
        <QuickApplyItems />
      </Menu.Dropdown>
    </Menu>
  )
}
