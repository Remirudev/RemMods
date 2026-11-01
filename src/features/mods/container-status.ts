import type { TFunction } from 'i18next'
import type { ContainerSummary } from '../../lib/ipc/types'

export function containerStatusText(container: ContainerSummary, t: TFunction<'mods'>): string {
  switch (container.status) {
    case 'read':
      return t(($) => $.content.status.read)
    case 'notIoStore':
      return t(($) => $.content.status.notIoStore)
    case 'unsupportedVersion':
      return t(($) => $.content.status.unsupportedVersion, { version: container.version ?? '?' })
    case 'encrypted':
      return t(($) => $.content.status.encrypted)
    case 'noIndex':
      return t(($) => $.content.status.noIndex)
    case 'malformed':
      return t(($) => $.content.status.malformed)
    case 'tooLarge':
      return t(($) => $.content.status.tooLarge)
    case 'unreadable':
      return t(($) => $.content.status.unreadable)
  }
}
