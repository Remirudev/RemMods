import { Badge, Button, Group, ScrollArea, Stack, Text, TextInput } from '@mantine/core'
import { IconSearch } from '@tabler/icons-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ENTRY_RENDER_LIMIT, filterEntries, originLabelKey, sharedLabel } from './content-entries'
import { characterLabel, slotLabel } from './content-hints'
import classes from './mods.module.css'
import type { ContentListProps } from './types'

const ORIGIN_COLORS = { adds: 'green', replaces: 'orange', unknown: 'gray' } as const

export function ContentList({ entries }: ContentListProps) {
  const { t, i18n } = useTranslation('mods')
  const [needle, setNeedle] = useState('')
  const [showAll, setShowAll] = useState(false)
  const matches = filterEntries(entries, needle, t, i18n.language)
  const visible = showAll ? matches : matches.slice(0, ENTRY_RENDER_LIMIT)

  return (
    <Stack gap="xs">
      <TextInput
        label={t(($) => $.content.filter)}
        leftSection={<IconSearch size={16} stroke={1.8} />}
        value={needle}
        onChange={(event) => setNeedle(event.currentTarget.value)}
      />
      <ScrollArea.Autosize mah={360} type="auto">
        <ul className={classes.fileList}>
          {visible.map((entry) => {
            const origin = originLabelKey(entry.origin)
            const shared = sharedLabel(entry.origin, entry.sharedWith.length, t)
            const hints = [
              entry.character ? characterLabel(entry.character, t) : null,
              entry.slot ? slotLabel(entry.slot, t) : null,
            ].filter((label): label is string => label !== null)
            return (
              <li key={`${entry.container}|${entry.path}`}>
                <Group justify="space-between" wrap="nowrap" gap="sm" py={2} align="flex-start">
                  <Stack gap={0}>
                    <Text size="sm" ff="monospace" style={{ overflowWrap: 'anywhere' }}>
                      <bdi>{entry.path}</bdi>
                    </Text>
                    {(hints.length > 0 || shared !== null) && (
                      <Text size="xs" c="var(--qc-text-2)">
                        <bdi>{hints.join(' · ')}</bdi>
                        {hints.length > 0 && shared !== null && ' · '}
                        {shared}
                      </Text>
                    )}
                  </Stack>
                  <Group gap="xs" wrap="nowrap">
                    {!entry.live && (
                      <Badge variant="light" color="gray">
                        {t(($) => $.content.off)}
                      </Badge>
                    )}
                    <Badge variant="light" color={ORIGIN_COLORS[origin]}>
                      {t(($) => $.content.origin[origin])}
                    </Badge>
                  </Group>
                </Group>
              </li>
            )
          })}
        </ul>
      </ScrollArea.Autosize>
      {!showAll && matches.length > ENTRY_RENDER_LIMIT && (
        <Button variant="default" w="fit-content" onClick={() => setShowAll(true)}>
          {t(($) => $.content.showAll, { count: matches.length })}
        </Button>
      )}
    </Stack>
  )
}
