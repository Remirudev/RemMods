import type {
  ArchiveEntry,
  LeftoverEntry,
  ManageAction,
  ManageBlock,
  ModRow,
} from '../../lib/ipc/types'
import type { ManagedKind, ManageSubject } from './types'

const BLOCKED: Record<ManagedKind, Record<ManageAction, ManageBlock | null>> = {
  folder: { rename: null, trash: null, delete: null, metadata: null },
  linked: { rename: 'linked', trash: 'linked', delete: 'linked', metadata: 'linked' },
  loose: { rename: 'loose', trash: null, delete: 'loose', metadata: 'loose' },
  archiveInRoot: { rename: 'archive', trash: null, delete: 'archive', metadata: 'archive' },
  archiveInPaks: {
    rename: 'archive',
    trash: 'outsideRoots',
    delete: 'archive',
    metadata: 'archive',
  },
  leftover: { rename: 'leftover', trash: null, delete: null, metadata: 'leftover' },
  unmanaged: {
    rename: 'unmanaged',
    trash: 'unmanaged',
    delete: 'unmanaged',
    metadata: 'unmanaged',
  },
}

// PLAN §5.1 / SPEC-P3 §5.1: the matrix the core enforces; the UI mirrors it to hide dead buttons.
export function manageBlock(kind: ManagedKind, action: ManageAction): ManageBlock | null {
  return BLOCKED[kind][action]
}

export function canManage(subject: Pick<ManageSubject, 'kind'>, action: ManageAction): boolean {
  return manageBlock(subject.kind, action) === null
}

function rowKind(row: ModRow): ManagedKind {
  if (row.kind === 'loose') return 'loose'
  if (row.toggleBlock === 'linked' || row.issues.some((issue) => issue.kind === 'linkedMod')) {
    return 'linked'
  }
  return 'folder'
}

export function rowSubject(row: ModRow): ManageSubject {
  return {
    target: { modRef: row.modRef, key: row.key },
    name: row.name,
    sizeBytes: row.sizeBytes,
    kind: rowKind(row),
  }
}

// An archive directly in Paks has no folder part; the core refuses to move it (outside the roots).
export function archiveSubject(archive: ArchiveEntry): ManageSubject {
  const inRoot = /[\\/]/u.test(archive.relPath)
  return {
    target: { modRef: archive.modRef, key: archive.relPath },
    name: archive.relPath,
    sizeBytes: archive.sizeBytes,
    kind: inRoot ? 'archiveInRoot' : 'archiveInPaks',
  }
}

export function leftoverSubject(leftover: LeftoverEntry): ManageSubject {
  return {
    target: { modRef: leftover.modRef, key: leftover.relPath },
    name: leftover.relPath,
    sizeBytes: null,
    kind: 'leftover',
  }
}
