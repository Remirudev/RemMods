import { Spotlight, type SpotlightActionData } from '@mantine/spotlight'
import { IconMoonStars, IconSearch } from '@tabler/icons-react'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useActivePanelStore } from './active-panel-store'
import { PANEL_IDS, panelRegistry } from './panels'
import { useColorSchemeToggle } from './use-color-scheme-toggle'

export function CommandPalette() {
  const { t } = useTranslation('shell')
  const setActivePanel = useActivePanelStore((state) => state.setActivePanel)
  const { toggle: toggleScheme } = useColorSchemeToggle()

  const actions = useMemo<SpotlightActionData[]>(() => {
    const panelActions = PANEL_IDS.map((id): SpotlightActionData => {
      const Icon = panelRegistry[id].icon
      return {
        id: `panel-${id}`,
        label: t(($) => $.palette.goTo, { panel: t(($) => $.panels[id].title) }),
        description: t(($) => $.panels[id].description),
        leftSection: <Icon size={18} stroke={1.8} />,
        onClick: () => setActivePanel(id),
      }
    })
    return [
      ...panelActions,
      {
        id: 'toggle-scheme',
        label: t(($) => $.palette.toggleScheme),
        leftSection: <IconMoonStars size={18} stroke={1.8} />,
        onClick: toggleScheme,
      },
    ]
  }, [t, setActivePanel, toggleScheme])

  return (
    <Spotlight
      actions={actions}
      shortcut="mod + K"
      highlightQuery
      nothingFound={t(($) => $.palette.nothingFound)}
      overlayProps={{ backgroundOpacity: 0.45, blur: 0 }}
      searchProps={{
        leftSection: <IconSearch size={18} stroke={1.8} />,
        placeholder: t(($) => $.palette.placeholder),
      }}
    />
  )
}
