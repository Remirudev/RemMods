import { Spotlight, type SpotlightActionData, spotlight } from '@mantine/spotlight'
import {
  IconArchive,
  IconFolder,
  IconFolderOpen,
  IconMoonStars,
  IconPlayerPlay,
  IconRefresh,
  IconSearch,
} from '@tabler/icons-react'
import { useEffect, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useRefreshMods } from '../features/mods/queries'
import { useGameStatus, useLaunchGame, useRevealPaks } from '../shared/game/queries'
import { pickArchivesToQueue, pickFolderToQueue } from '../shared/install-actions'
import { useUpdateRun } from '../shared/update-run-store'
import { useActivePanelStore } from './active-panel-store'
import { PANEL_IDS, panelRegistry } from './panels'
import { useColorSchemeToggle } from './use-color-scheme-toggle'

export function CommandPalette() {
  const { t } = useTranslation('shell')
  const { t: tInstall } = useTranslation('install')
  const setActivePanel = useActivePanelStore((state) => state.setActivePanel)
  const { toggle: toggleScheme } = useColorSchemeToggle()
  const status = useGameStatus().data
  const { mutate: launchGame } = useLaunchGame()
  const { mutate: refreshMods } = useRefreshMods()
  const { mutate: revealPaks } = useRevealPaks()
  const ready = status?.location.kind === 'ready'
  const running = status?.running === 'running'
  const updating = useUpdateRun((state) => state.updating)

  // No palette while a self-update runs: its actions would hide the progress dialog or start work.
  useEffect(() => {
    if (updating) spotlight.close()
  }, [updating])

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
    const gameActions: SpotlightActionData[] = [
      {
        id: 'launch-game',
        label: t(($) => $.palette.launchGame),
        leftSection: <IconPlayerPlay size={18} stroke={1.8} />,
        disabled: running,
        onClick: () => launchGame(),
      },
      ...(ready
        ? [
            {
              id: 'refresh-mods',
              label: t(($) => $.palette.refreshMods),
              leftSection: <IconRefresh size={18} stroke={1.8} />,
              onClick: () => refreshMods(),
            },
            {
              id: 'install-archives',
              label: tInstall(($) => $.palette.archives),
              leftSection: <IconArchive size={18} stroke={1.8} />,
              onClick: () => void pickArchivesToQueue(tInstall(($) => $.pick.archives)),
            },
            {
              id: 'install-folder',
              label: tInstall(($) => $.palette.folder),
              leftSection: <IconFolder size={18} stroke={1.8} />,
              onClick: () => void pickFolderToQueue(tInstall(($) => $.pick.folder)),
            },
            {
              id: 'open-paks',
              label: t(($) => $.palette.openPaks),
              leftSection: <IconFolderOpen size={18} stroke={1.8} />,
              onClick: () => revealPaks(),
            },
          ]
        : []),
    ]
    return [
      ...panelActions,
      ...gameActions,
      {
        id: 'toggle-scheme',
        label: t(($) => $.palette.toggleScheme),
        leftSection: <IconMoonStars size={18} stroke={1.8} />,
        onClick: toggleScheme,
      },
    ]
  }, [
    t,
    tInstall,
    setActivePanel,
    toggleScheme,
    running,
    ready,
    launchGame,
    refreshMods,
    revealPaks,
  ])

  return (
    <Spotlight
      actions={actions}
      shortcut={updating ? null : 'mod + K'}
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
