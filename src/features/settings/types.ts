import type {
  Appearance,
  AppUpdateChannel,
  AppUpdateCheck,
  AppUpdateProgress,
  Settings,
} from '../../lib/ipc/types'

export type AppearancePatch = Partial<Appearance>

export interface OptimisticSettings {
  previous: Settings | undefined
}

export interface SegmentOption<T extends string> {
  value: T
  label: string
}

export interface DiagnosticsRow {
  label: string
  value: string | null | undefined
}

export interface SectionHeadingProps {
  title: string
  description: string
}

export interface GameRow {
  label: string
  value: string
}

export type PackageFormat = '.deb' | '.rpm'

export type UpdateNoticeKey = 'package' | 'aur' | 'unknown'

export interface UpdateNotice {
  kind: 'notice'
  notice: UpdateNoticeKey
  format: PackageFormat | null
}

// What the Updates section offers once a newer version is known.
export type UpdateOffer =
  | { kind: 'notConfigured' }
  | { kind: 'install'; installer: boolean }
  | UpdateNotice

export interface UpdateInstallState {
  running: boolean
  progress: AppUpdateProgress | null
  error: unknown
}

export interface UpdateProgressViewProps {
  progress: AppUpdateProgress | null
}

export interface UpdateResultProps {
  check: AppUpdateCheck
  channel: AppUpdateChannel
  installing: boolean
  onInstall: (version: string) => void
}

export interface UpdateKindNoticeProps {
  notice: UpdateNotice
}

export interface CopyLinkRowProps {
  label: string
  url: string
}

export interface NoticesModalProps {
  opened: boolean
  onClose: () => void
}
