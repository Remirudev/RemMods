import { modals } from '@mantine/modals'
import { i18n } from '../../lib/i18n'
import type { SkinSet } from '../../lib/ipc/types'
import { NewSkinSetDialog } from './NewSkinSetDialog'
import { SkinCopiesDialog } from './SkinCopiesDialog'
import { SkinSetDialog } from './SkinSetDialog'

function skinsT() {
  return i18n.getFixedT(null, 'skins')
}

export function openNewSkinSet(): void {
  const modalId = crypto.randomUUID()
  modals.open({
    modalId,
    title: skinsT()(($) => $.newSet.title),
    children: <NewSkinSetDialog modalId={modalId} onClose={() => modals.close(modalId)} />,
  })
}

export function openKeepCopy(copy: string): void {
  const modalId = crypto.randomUUID()
  modals.open({
    modalId,
    title: skinsT()(($) => $.keepCopy.title),
    children: (
      <NewSkinSetDialog modalId={modalId} copy={copy} onClose={() => modals.close(modalId)} />
    ),
  })
}

export function openSkinCopies(): void {
  modals.open({
    title: skinsT()(($) => $.copies.title),
    size: 'lg',
    children: <SkinCopiesDialog />,
  })
}

export function openSkinSet(set: SkinSet): void {
  const modalId = crypto.randomUUID()
  modals.open({
    modalId,
    title: <bdi>{set.name}</bdi>,
    size: 'xl',
    children: (
      <SkinSetDialog setId={set.id} modalId={modalId} onClose={() => modals.close(modalId)} />
    ),
  })
}

// What the game has now may be wanted, even though it stays as an automatic copy.
export function confirmPutBack(set: SkinSet, onConfirm: () => void): void {
  const t = skinsT()
  modals.openConfirmModal({
    title: t(($) => $.putBack.title, { name: set.name }),
    children: t(($) => $.putBack.body),
    labels: { confirm: t(($) => $.putBack.confirm), cancel: t(($) => $.putBack.cancel) },
    onConfirm,
  })
}

// Only a stored copy goes, but it may be the only place those skins still exist.
export function confirmDeleteSkinSet(set: SkinSet, onConfirm: () => void): void {
  const t = skinsT()
  modals.openConfirmModal({
    title: t(($) => $.delete.title, { name: set.name }),
    children: t(($) => $.delete.body),
    labels: { confirm: t(($) => $.delete.confirm), cancel: t(($) => $.delete.cancel) },
    confirmProps: { color: 'red' },
    onConfirm,
  })
}
