import { Channel } from '@tauri-apps/api/core'
import { useCallback, useState } from 'react'
import { ipc } from '../../lib/ipc'
import type { InstallOutcome, InstallProgress, StartRequest } from '../../lib/ipc/types'

// One Channel per run; the latest message is the progress shown.
export function useInstallJob() {
  const [progress, setProgress] = useState<InstallProgress | null>(null)

  const start = useCallback(async (request: StartRequest): Promise<InstallOutcome> => {
    const channel = new Channel<InstallProgress>()
    channel.onmessage = setProgress
    setProgress(null)
    return ipc.installStart(request, channel)
  }, [])

  // Nothing to cancel is not an error; a failure here is only logged by the caller if at all.
  const cancel = useCallback(() => {
    ipc.installCancel().catch(() => undefined)
  }, [])

  return { progress, start, cancel }
}
