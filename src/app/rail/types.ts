import type { ReactNode } from 'react'

export interface RailButtonProps {
  label: string
  active?: boolean
  onClick: () => void
  children: ReactNode
}
