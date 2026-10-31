import { Box } from '@mantine/core'
import { IconMoon, IconStack2, IconSun } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { useActivePanelStore } from '../active-panel-store'
import { PANEL_IDS, panelRegistry } from '../panels'
import { useColorSchemeToggle } from '../use-color-scheme-toggle'
import { RailButton } from './RailButton'
import classes from './rail.module.css'

export function Rail() {
  const { t } = useTranslation('shell')
  const activePanel = useActivePanelStore((state) => state.activePanel)
  const setActivePanel = useActivePanelStore((state) => state.setActivePanel)
  const { isDark, toggle } = useColorSchemeToggle()
  const themeLabel = isDark ? t(($) => $.rail.lightTheme) : t(($) => $.rail.darkTheme)

  return (
    <Box
      component="nav"
      className={classes.rail}
      aria-label={t(($) => $.rail.label)}
      data-testid="rail"
    >
      <div className={classes.items}>
        <div className={classes.brand}>
          <IconStack2 size={22} stroke={1.8} />
        </div>
        {PANEL_IDS.map((id) => {
          const Icon = panelRegistry[id].icon
          return (
            <RailButton
              key={id}
              label={t(($) => $.panels[id].title)}
              active={id === activePanel}
              onClick={() => setActivePanel(id)}
            >
              <Icon size={20} stroke={1.8} />
            </RailButton>
          )
        })}
      </div>
      <RailButton label={themeLabel} onClick={toggle}>
        {isDark ? <IconSun size={20} stroke={1.8} /> : <IconMoon size={20} stroke={1.8} />}
      </RailButton>
    </Box>
  )
}
