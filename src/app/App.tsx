import { AppShell } from '@mantine/core'
import { InstallHost } from '../features/install/InstallHost'
import { GraphicsStepBanner } from '../features/settings/GraphicsStepBanner'
import { useActivePanelStore } from './active-panel-store'
import { CommandPalette } from './CommandPalette'
import { DropOverlay } from './DropOverlay'
import { PanelHost } from './PanelHost'
import classes from './panel-host.module.css'
import { useProfileApplyBridge } from './profile-apply-bridge'
import { Rail } from './rail/Rail'
import { ResizeHandles } from './title-bar/ResizeHandles'
import { TitleBar } from './title-bar/TitleBar'
import { useTitleBarMode } from './title-bar/use-title-bar-mode'
import { useAppReady } from './use-app-ready'
import { useBackendEvents } from './use-backend-events'
import { useFolderBusy } from './use-folder-busy'
import { useUiScaleHotkeys } from './use-ui-scale-hotkeys'

const TITLE_BAR = { height: 36 }

// Breakpoint 0 keeps the rail inline at every width; below a real breakpoint Mantine switches to
// its mobile layout and hides the navbar.
const RAIL = { width: 72, breakpoint: 0 }

export function App() {
  const customTitleBar = useTitleBarMode() === 'custom'
  // The Graphics section in Settings shows its own copy of the banner.
  const settingsActive = useActivePanelStore((state) => state.activePanel) === 'settings'
  useAppReady()
  useBackendEvents()
  useFolderBusy()
  useProfileApplyBridge()
  useUiScaleHotkeys()

  return (
    <>
      <AppShell header={customTitleBar ? TITLE_BAR : undefined} navbar={RAIL} padding={0}>
        {customTitleBar && (
          <AppShell.Header>
            <TitleBar />
          </AppShell.Header>
        )}
        <AppShell.Navbar>
          <Rail />
        </AppShell.Navbar>
        <AppShell.Main h="100vh" className={classes.main}>
          <div className={classes.banner}>{!settingsActive && <GraphicsStepBanner />}</div>
          <PanelHost />
        </AppShell.Main>
      </AppShell>
      {customTitleBar && <ResizeHandles />}
      <CommandPalette />
      <InstallHost />
      <DropOverlay />
    </>
  )
}
