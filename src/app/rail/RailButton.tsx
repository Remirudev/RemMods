import { Tooltip, UnstyledButton } from '@mantine/core'
import classes from './rail.module.css'
import type { RailButtonProps } from './types'

export function RailButton({ label, active = false, onClick, children }: RailButtonProps) {
  return (
    <Tooltip label={label} position="right" offset={10}>
      <UnstyledButton
        className={classes.button}
        data-active={active || undefined}
        aria-label={label}
        aria-current={active ? 'page' : undefined}
        onClick={onClick}
      >
        {children}
      </UnstyledButton>
    </Tooltip>
  )
}
