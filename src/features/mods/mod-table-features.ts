import {
  columnSizingFeature,
  columnVisibilityFeature,
  createSortedRowModel,
  metaHelper,
  rowSortingFeature,
  tableFeatures,
} from '@tanstack/react-table'
import type { ModColumnMeta, ModTableMeta } from './types'

// Widths and visibility come from the column layout store; the drag itself is ColumnResizeHandle's,
// so the resizing feature is not registered.
export const modTableFeatures = tableFeatures({
  rowSortingFeature,
  columnSizingFeature,
  columnVisibilityFeature,
  sortedRowModel: createSortedRowModel(),
  tableMeta: metaHelper<ModTableMeta>(),
  columnMeta: metaHelper<ModColumnMeta>(),
})
