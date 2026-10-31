import type { Appearance, Settings } from '../../lib/ipc/types'

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
