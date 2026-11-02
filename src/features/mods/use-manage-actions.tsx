import { modals } from '@mantine/modals'
import { type QueryClient, useQueryClient } from '@tanstack/react-query'
import { useMemo } from 'react'
import { i18n } from '../../lib/i18n'
import type { ModsSnapshot } from '../../lib/ipc/types'
import { queryKeys } from '../../shared/query-keys'
import { deletePermanently, trashWithFallback } from './delete-flow'
import { buildFlowDeps, originalAppRunning } from './flow-deps'
import { siblingFolds } from './name-check'
import { RenameDialog } from './RenameDialog'
import type { ManageActions, ManageSubject } from './types'

function openRename(client: QueryClient, subject: ManageSubject, suggestedName?: string | null) {
  const t = i18n.getFixedT(null, 'mods')
  const rows = client.getQueryData<ModsSnapshot>(queryKeys.mods)?.rows ?? []
  const row = rows.find((candidate) => candidate.modRef === subject.target.modRef)
  const currentName = row?.name ?? subject.name
  const modalId = crypto.randomUUID()
  modals.open({
    modalId,
    title: t(($) => $.rename.title),
    children: (
      <RenameDialog
        target={subject.target}
        currentName={currentName}
        initialName={suggestedName ?? currentName}
        siblings={siblingFolds(rows, row)}
        originalApp={originalAppRunning(client)}
        onClose={() => modals.close(modalId)}
        modalId={modalId}
      />
    ),
  })
}

// The three single-target file actions, opened from the row menu, the keyboard, the drawer and
// Repair. The returned object is stable.
export function useManageActions(): ManageActions {
  const client = useQueryClient()
  return useMemo(
    () => ({
      rename: (subject, suggestedName) => openRename(client, subject, suggestedName),
      trash: (subject) => void trashWithFallback(subject, buildFlowDeps(client)),
      remove: (subject) => void deletePermanently(subject, buildFlowDeps(client)),
    }),
    [client],
  )
}
