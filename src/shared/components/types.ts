import type { ReactNode } from 'react'

export interface PanelHeaderProps {
  title: string
  description?: string
  actions?: ReactNode
}

export interface EmptyStateProps {
  icon: ReactNode
  title: string
  description: string
}
