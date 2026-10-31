import { useEffect, useState } from 'react'
import { currentWindowOrNull } from './current-window'

export function useWindowMaximized(): boolean {
  const [maximized, setMaximized] = useState(false)

  useEffect(() => {
    const appWindow = currentWindowOrNull()
    if (!appWindow) return
    let active = true

    const refresh = async (): Promise<void> => {
      const value = await appWindow.isMaximized().catch(() => false)
      if (active) setMaximized(value)
    }

    refresh()
    const listening = appWindow.onResized(() => refresh()).catch(() => null)
    return () => {
      active = false
      listening.then((unlisten) => unlisten?.())
    }
  }, [])

  return maximized
}
