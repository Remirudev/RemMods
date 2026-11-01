import { Badge, Group, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { characterLabel, isRawCharacter, slotLabel } from './content-hints'
import type { ContentHintsProps } from './types'

export function ContentHints({ content }: ContentHintsProps) {
  const { t } = useTranslation('mods')
  const { characters, slots } = content
  if (characters.length === 0 && slots.length === 0) return null

  return (
    <Stack gap="xs">
      {characters.length > 0 && (
        <Group gap="xs">
          <Text size="sm" c="var(--qc-text-2)">
            {t(($) => $.content.characters)}
          </Text>
          {characters.map((hint) => (
            <Badge
              key={hint.code}
              variant="light"
              title={isRawCharacter(hint) ? t(($) => $.content.rawCodeHint) : undefined}
            >
              <bdi>{characterLabel(hint, t)}</bdi>
            </Badge>
          ))}
        </Group>
      )}
      {slots.length > 0 && (
        <Group gap="xs">
          <Text size="sm" c="var(--qc-text-2)">
            {t(($) => $.content.slots)}
          </Text>
          {slots.map((hint) => (
            <Badge
              key={hint.code}
              variant="light"
              color="gray"
              title={hint.slot === null ? t(($) => $.content.rawCodeHint) : undefined}
            >
              <bdi>{slotLabel(hint, t)}</bdi>
            </Badge>
          ))}
        </Group>
      )}
    </Stack>
  )
}
