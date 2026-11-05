import { Alert, Button, Center, Group, Loader, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { useSkinCopies } from './queries'
import { openKeepCopy } from './skin-modals'

// Nothing here touches the game: a copy becomes a set first, and that set is switched to.
export function SkinCopiesDialog() {
  const { t, i18n } = useTranslation('skins')
  const copies = useSkinCopies()

  if (copies.isError) {
    return (
      <Alert color="red" role="alert">
        {t(($) => $.copies.loadError)}
      </Alert>
    )
  }
  if (!copies.data) {
    return (
      <Center py="lg">
        <Loader size="sm" />
      </Center>
    )
  }
  return (
    <Stack gap="md" data-testid="skin-copies">
      <Text size="sm" c="var(--qc-text-2)">
        {t(($) => $.copies.hint)}
      </Text>
      {copies.data.length === 0 && <Text size="sm">{t(($) => $.copies.empty)}</Text>}
      {copies.data.map((copy) => (
        <Group key={copy.id} justify="space-between" wrap="nowrap" gap="sm" data-testid="skin-copy">
          <div>
            <Text size="sm">
              {copy.setName
                ? t(($) => $.copies.earlier, { name: copy.setName })
                : t(($) => $.copies.before)}
            </Text>
            <Text size="xs" c="var(--qc-text-2)">
              {new Date(copy.taken).toLocaleString(i18n.language)}
            </Text>
          </div>
          <Button size="compact-xs" variant="light" onClick={() => openKeepCopy(copy.id)}>
            {t(($) => $.copies.keep)}
          </Button>
        </Group>
      ))}
    </Stack>
  )
}
