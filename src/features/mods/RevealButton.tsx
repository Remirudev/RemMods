import { ActionIcon, Tooltip } from '@mantine/core'
import { IconFolderOpen } from '@tabler/icons-react'
import { useRevealEntry } from './queries'
import type { RevealButtonProps } from './types'

export function RevealButton({ modRef, label }: RevealButtonProps) {
  const reveal = useRevealEntry()

  return (
    <Tooltip label={label}>
      <ActionIcon
        variant="subtle"
        color="gray"
        aria-label={label}
        loading={reveal.isPending}
        onClick={() => reveal.mutate(modRef)}
      >
        <IconFolderOpen size={18} stroke={1.8} />
      </ActionIcon>
    </Tooltip>
  )
}
