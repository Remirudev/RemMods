import { AppShell } from '@mantine/core'
import { CommandPalette } from './CommandPalette'
import { PanelHost } from './PanelHost'
import { Rail } from './rail/Rail'
import { ResizeHandles } from './title-bar/ResizeHandles'
import { TitleBar } from './title-bar/TitleBar'
import { useTitleBarMode } from './title-bar/use-title-bar-mode'
import { useAppReady } from './use-app-ready'
import { useUiScaleHotkeys } from './use-ui-scale-hotkeys'

const TITLE_BAR = { height: 36 }

// Breakpoint 0 keeps the rail inline at every width; below a real breakpoint Mantine switches to
// its mobile layout and hides the navbar.
const RAIL = { width: 72, breakpoint: 0 }

export function App() {
  const customTitleBar = useTitleBarMode() === 'custom'
  useAppReady()
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
        <AppShell.Main h="100vh" style={{ overflow: 'hidden' }}>
          <PanelHost />
        </AppShell.Main>
      </AppShell>
      {customTitleBar && <ResizeHandles />}
      <CommandPalette />
    </>
  )
}
