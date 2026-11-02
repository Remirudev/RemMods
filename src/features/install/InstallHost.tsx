import { useInstallQueue } from '../../shared/install-queue'
import { InstallModal } from './InstallModal'

// Mounted once in the app: the head of the queue gets a modal, then the next one follows.
export function InstallHost() {
  const current = useInstallQueue((state) => state.current)
  const turn = useInstallQueue((state) => state.turn)
  const remaining = useInstallQueue((state) => Math.max(0, state.queue.length - 1))
  const shift = useInstallQueue((state) => state.shift)
  const clear = useInstallQueue((state) => state.clear)

  if (!current) return null
  return (
    <InstallModal
      key={turn}
      source={current}
      remaining={remaining}
      onClose={shift}
      onStop={clear}
    />
  )
}
