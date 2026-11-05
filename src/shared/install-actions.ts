import { ipc } from '../lib/ipc'
import type { PickedSource } from '../lib/ipc/types'
import { showErrorToast } from '../lib/query/error-toast'
import { useInstallQueue } from './install-queue'
import type { QueuedSource } from './types'

export function queuedFromPicked(source: PickedSource): QueuedSource {
  return { kind: 'token', id: source.id, name: source.name, sourceKind: source.kind }
}

// The only callers of pick_archives and pick_folder. A cancelled dialog queues nothing; a
// refused folder (inside Paks) is a toast.
export async function pickArchivesToQueue(title: string): Promise<void> {
  try {
    const sources = await ipc.pickArchives(title)
    useInstallQueue.getState().push(sources.map(queuedFromPicked))
  } catch (error) {
    showErrorToast(error)
  }
}

export async function pickFolderToQueue(title: string): Promise<void> {
  try {
    const source = await ipc.pickFolder(title)
    if (source) useInstallQueue.getState().push([queuedFromPicked(source)])
  } catch (error) {
    showErrorToast(error)
  }
}

export function queueArchiveRow(row: string, name: string): void {
  useInstallQueue.getState().push([{ kind: 'archive', row, name }])
}
