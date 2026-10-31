import { useEffect } from 'react'
import { ipc } from '../lib/ipc'
import { describeForLog } from '../lib/ipc/ipc-error'
import { logError } from '../lib/log/frontend-log'

function reportReady(): void {
  ipc.appReady().catch((error: unknown) => logError(`app_ready failed: ${describeForLog(error)}`))
}

// The window stays hidden until the first frame is painted, so it never flashes empty. A hidden
// page gets no frames (WebKitGTK pauses requestAnimationFrame), so there the committed render is
// enough and the window's background colour covers the first paint. `app_ready` is idempotent.
export function useAppReady(): void {
  useEffect(() => {
    if (document.visibilityState === 'hidden') {
      reportReady()
      return
    }
    const frame = requestAnimationFrame(reportReady)
    return () => cancelAnimationFrame(frame)
  }, [])
}
