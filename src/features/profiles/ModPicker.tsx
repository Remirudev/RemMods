import { Button, Group, ScrollArea, Stack, Text, TextInput } from '@mantine/core'
import { IconRestore, IconSearch } from '@tabler/icons-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { countPicks, matchesPickSearch } from './group-picks'
import classes from './groups.module.css'
import { ModPickRow } from './ModPickRow'
import type { ModPickerProps } from './types'

// Every mod with on / off / leave alone; the buttons set all mods the search shows at once.
export function ModPicker({ items, picks, onPick, onFromCurrent }: ModPickerProps) {
  const { t, i18n } = useTranslation('profiles')
  const [search, setSearch] = useState('')
  const locale = i18n.language
  const needle = search.trim().toLocaleLowerCase(locale)
  const shown = useMemo(
    () => items.filter((item) => matchesPickSearch(item, needle, locale)),
    [items, needle, locale],
  )
  const shownKeys = shown.map((item) => item.key)
  const counts = countPicks(picks)

  return (
    <Stack gap="xs">
      <Group gap="sm" align="flex-end" wrap="wrap">
        <TextInput
          flex={1}
          miw={220}
          label={t(($) => $.group.mods)}
          placeholder={t(($) => $.group.search)}
          leftSection={<IconSearch size={16} stroke={1.8} />}
          value={search}
          onChange={(event) => setSearch(event.currentTarget.value)}
        />
        <Button
          variant="default"
          leftSection={<IconRestore size={16} stroke={1.8} />}
          onClick={onFromCurrent}
        >
          {t(($) => $.group.fromCurrent)}
        </Button>
      </Group>
      <Group gap="xs" wrap="wrap">
        <Text size="xs" c="var(--qc-text-2)">
          {t(($) => $.group.allShown, { count: shown.length })}
        </Text>
        <Button size="compact-xs" variant="light" onClick={() => onPick(shownKeys, 'on')}>
          {t(($) => $.group.pick.on)}
        </Button>
        <Button size="compact-xs" variant="light" onClick={() => onPick(shownKeys, 'off')}>
          {t(($) => $.group.pick.off)}
        </Button>
        <Button size="compact-xs" variant="light" onClick={() => onPick(shownKeys, 'skip')}>
          {t(($) => $.group.pick.skip)}
        </Button>
      </Group>
      <ScrollArea.Autosize mah={340} className={classes.pickList}>
        {shown.length === 0 ? (
          <Text size="sm" c="var(--qc-text-2)" p="sm">
            {items.length === 0 ? t(($) => $.group.noMods) : t(($) => $.group.noMatch)}
          </Text>
        ) : (
          shown.map((item) => (
            <ModPickRow
              key={item.key}
              item={item}
              pick={picks.get(item.key) ?? 'skip'}
              onPick={onPick}
            />
          ))
        )}
      </ScrollArea.Autosize>
      <Text size="sm" c="var(--qc-text-2)" aria-live="polite" data-testid="pick-counts">
        {t(($) => $.group.counts, { on: counts.on, off: counts.off })}
      </Text>
    </Stack>
  )
}
