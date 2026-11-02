import { Badge, Group, Loader, Menu, Text } from '@mantine/core'
import { IconCheck } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { useProfileQuickApply } from '../../shared/profile-quick-apply'

export function QuickApplyItems() {
  const { t } = useTranslation('mods')
  const { t: tProfiles } = useTranslation('profiles')
  const { choices, lastAppliedId, loading, failed, readOnly, open } = useProfileQuickApply()

  if (loading) {
    return (
      <Group justify="center" py="xs">
        <Loader size="xs" />
      </Group>
    )
  }
  if (failed) {
    return <Menu.Item disabled>{tProfiles(($) => $.panel.loadError)}</Menu.Item>
  }

  return (
    <>
      {readOnly && (
        <Menu.Label>
          <Text size="xs">{t(($) => $.quickApply.readOnly)}</Text>
        </Menu.Label>
      )}
      {choices.length === 0 && <Menu.Item disabled>{t(($) => $.quickApply.empty)}</Menu.Item>}
      {choices.map((choice) => (
        <Menu.Item
          key={choice.id}
          disabled={readOnly}
          onClick={() => open(choice.id)}
          leftSection={
            choice.id === lastAppliedId ? (
              <IconCheck size={16} stroke={1.8} aria-label={t(($) => $.quickApply.last)} />
            ) : (
              <span style={{ display: 'inline-block', width: 16 }} />
            )
          }
          rightSection={
            <Badge variant="light" size="sm">
              {tProfiles(($) => $.mode[choice.mode].label)}
            </Badge>
          }
        >
          <bdi>{choice.name.trim() === '' ? tProfiles(($) => $.unnamed) : choice.name}</bdi>
        </Menu.Item>
      ))}
    </>
  )
}
