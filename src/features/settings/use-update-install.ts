import { Channel } from '@tauri-apps/api/core'
import { useCallback, useState } from 'react'
import { ipc } from '../../lib/ipc'
import type { AppUpdateProgress } from '../../lib/ipc/types'
import type { UpdateInstallState } from './types'

const IDLE: UpdateInstallState = { running: false, progress: null, error: null }

// One Channel per run. On success RemMods exits (Linux relaunches the AppImage, Windows hands
// over to the installer), so the progress stays on "restarting" until the window closes; any
// error ends the run and is shown by the section.
export function useUpdateInstall() {
  const [state, setState] = useState<UpdateInstallState>(IDLE)

  const start = useCallback(async (version: string): Promise<void> => {
    const channel = new Channel<AppUpdateProgress>()
    channel.onmessage = (progress) => setState((current) => ({ ...current, progress }))
    setState({ running: true, progress: null, error: null })
    try {
      await ipc.updateInstall(version, channel)
    } catch (error) {
      setState({ running: false, progress: null, error })
    }
  }, [])

  const clearError = useCallback(() => setState(IDLE), [])

  return { ...state, start, clearError }
}
