import { useQueryClient } from '@tanstack/react-query'
import { useMemo } from 'react'
import { buildBulkDeps } from './bulk-deps'
import { deleteMany, trashMany } from './bulk-flow'
import { bulkSelection } from './mod-actions'
import type { BulkActions } from './types'

// Bulk trash and delete for the ActionBar and the keyboard. Only the rows the matrix allows are
// sent; the others are counted as skipped. The returned object is stable.
export function useBulkActions(): BulkActions {
  const client = useQueryClient()
  return useMemo(
    () => ({
      trash: (rows) => void trashMany(bulkSelection(rows, 'trash'), buildBulkDeps(client)),
      remove: (rows) => void deleteMany(bulkSelection(rows, 'delete'), buildBulkDeps(client)),
    }),
    [client],
  )
}
