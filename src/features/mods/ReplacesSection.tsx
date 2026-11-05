import { Chip, Group, Stack, Text } from '@mantine/core'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { characterLabel } from './content-hints'
import { DrawerSection } from './DrawerSection'
import { useModsUiStore } from './mods-ui-store'
import { useModItems } from './queries'
import { ReplacesOther } from './ReplacesOther'
import { ReplacesRow } from './ReplacesRow'
import { charactersOf, chosenCharacter, itemGroups } from './replaces-model'
import type { ReplacesSectionProps } from './types'

// One line per item of the chosen character, the way the game's own menu lists them. The choice
// stays while the drawer moves on to other mods.
export function ReplacesSection({ modRef }: ReplacesSectionProps) {
  const { t, i18n } = useTranslation('mods')
  const items = useModItems(modRef, i18n.language).data?.items
  const wanted = useModsUiStore((state) => state.detailsCharacter)
  const setCharacter = useModsUiStore((state) => state.setDetailsCharacter)
  const characters = useMemo(() => charactersOf(items ?? [], i18n.language), [items, i18n.language])
  const character = chosenCharacter(characters, wanted)
  const { main, other } = useMemo(() => itemGroups(items ?? [], character), [items, character])

  if (characters.length === 0) return null

  return (
    <DrawerSection title={t(($) => $.replaces.title)}>
      <Stack gap="sm">
        {characters.length === 1 ? (
          <Text size="sm" c="var(--qc-text-2)">
            <bdi>{characters[0] && characterLabel(characters[0], t)}</bdi>
          </Text>
        ) : (
          <Chip.Group
            multiple={false}
            value={character}
            onChange={(code) => {
              if (typeof code === 'string') setCharacter(code)
            }}
          >
            <Group gap={6} role="radiogroup" aria-label={t(($) => $.replaces.character)}>
              {characters.map((hint) => (
                <Chip key={hint.code} value={hint.code} size="xs" variant="light">
                  <bdi>{characterLabel(hint, t)}</bdi>
                </Chip>
              ))}
            </Group>
          </Chip.Group>
        )}
        <Stack gap="xs">
          {main.map((item) => (
            <ReplacesRow key={item.key} item={item} />
          ))}
        </Stack>
        {other.length > 0 && <ReplacesOther key={character} items={other} />}
      </Stack>
    </DrawerSection>
  )
}
