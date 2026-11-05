import { ActionIcon, Button, Group, ScrollArea, Stack, Text, Tooltip } from '@mantine/core'
import { IconHistory, IconPlus } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { useSkinSets } from '../../shared/skin-sets'
import { InterruptedSwitch } from './InterruptedSwitch'
import { SkinSetRow } from './SkinSetRow'
import { openNewSkinSet, openSkinCopies } from './skin-modals'
import classes from './skins.module.css'

// Under the groups, so skins and mods switch from one place.
export function SkinSetsSection() {
  const { t } = useTranslation('skins')
  const view = useSkinSets().data
  if (!view) return null

  const interrupted = view.sets.find((set) => set.id === view.interrupted)
  const locked = view.readOnly || interrupted !== undefined

  return (
    <Stack gap="xs" className={classes.section} data-testid="skin-sets">
      <Group justify="space-between" wrap="nowrap" gap={4}>
        <Text fw={600} size="sm">
          {t(($) => $.title)}
        </Text>
        {view.available && (
          <Group wrap="nowrap" gap={2}>
            <Tooltip label={t(($) => $.copies.title)} withinPortal>
              <ActionIcon
                variant="subtle"
                color="gray"
                aria-label={t(($) => $.copies.title)}
                disabled={view.readOnly}
                onClick={openSkinCopies}
              >
                <IconHistory size={18} stroke={1.8} />
              </ActionIcon>
            </Tooltip>
            <Tooltip label={t(($) => $.saveCurrent)} withinPortal>
              <ActionIcon
                variant="subtle"
                color="gray"
                aria-label={t(($) => $.saveCurrent)}
                disabled={locked}
                onClick={openNewSkinSet}
              >
                <IconPlus size={18} stroke={1.8} />
              </ActionIcon>
            </Tooltip>
          </Group>
        )}
      </Group>
      {!view.available && (
        <Text size="xs" c="var(--qc-text-2)">
          {t(($) => $.unavailable)}
        </Text>
      )}
      {view.readOnly && (
        <Text size="xs" c="yellow" role="status">
          {t(($) => $.readOnly)}
        </Text>
      )}
      {interrupted && <InterruptedSwitch setId={interrupted.id} name={interrupted.name} />}
      {view.available && view.sets.length === 0 && (
        <Stack gap="sm" align="flex-start">
          <Text size="sm" c="var(--qc-text-2)">
            {t(($) => $.empty)}
          </Text>
          <Button
            size="compact-sm"
            variant="light"
            leftSection={<IconPlus size={16} stroke={1.8} />}
            disabled={locked}
            onClick={openNewSkinSet}
          >
            {t(($) => $.saveFirst)}
          </Button>
        </Stack>
      )}
      {view.sets.length > 0 && (
        <ScrollArea.Autosize mah={220} type="hover">
          <Stack gap={2}>
            {view.sets.map((set) => (
              <SkinSetRow
                key={set.id}
                set={set}
                active={view.active === set.id}
                disabled={locked}
              />
            ))}
          </Stack>
        </ScrollArea.Autosize>
      )}
    </Stack>
  )
}
