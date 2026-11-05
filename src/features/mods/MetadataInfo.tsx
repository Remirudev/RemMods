import { Button, DataList, Stack, Text } from '@mantine/core'
import { IconPencil } from '@tabler/icons-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { MetadataEditor } from './MetadataEditor'
import { canManage, rowSubject } from './manage-model'
import { modInfoText } from './mod-info-text'
import { useModDetails } from './queries'
import type { MetadataInfoProps } from './types'

// The info section of the drawer: read-only values, or the form after "Edit".
export function MetadataInfo({ row }: MetadataInfoProps) {
  const { t } = useTranslation('mods')
  const [editing, setEditing] = useState(false)
  const details = useModDetails(row.modRef).data
  const originText =
    row.info.origin === 'file'
      ? t(($) => $.drawer.infoFile)
      : row.info.origin === 'missing'
        ? t(($) => $.drawer.infoMissing)
        : t(($) => $.drawer.infoInvalid)

  if (editing) return <MetadataEditor row={row} onDone={() => setEditing(false)} />

  return (
    <Stack gap="xs">
      <DataList labelWidth={120}>
        <DataList.Item>
          <DataList.ItemLabel>{t(($) => $.drawer.version)}</DataList.ItemLabel>
          <DataList.ItemValue>{modInfoText(row.info, 'version', t)}</DataList.ItemValue>
        </DataList.Item>
        <DataList.Item>
          <DataList.ItemLabel>{t(($) => $.drawer.category)}</DataList.ItemLabel>
          <DataList.ItemValue>{modInfoText(row.info, 'category', t)}</DataList.ItemValue>
        </DataList.Item>
        <DataList.Item>
          <DataList.ItemLabel>{t(($) => $.drawer.author)}</DataList.ItemLabel>
          <DataList.ItemValue>
            <bdi>{modInfoText(row.info, 'author', t)}</bdi>
          </DataList.ItemValue>
        </DataList.Item>
        <DataList.Item>
          <DataList.ItemLabel>{t(($) => $.drawer.description)}</DataList.ItemLabel>
          <DataList.ItemValue>
            <bdi>
              {modInfoText(row.info, 'description', t) || t(($) => $.drawer.descriptionEmpty)}
            </bdi>
          </DataList.ItemValue>
        </DataList.Item>
        {details?.added && (
          <DataList.Item>
            <DataList.ItemLabel>{t(($) => $.drawer.added)}</DataList.ItemLabel>
            <DataList.ItemValue>{new Date(details.added).toLocaleDateString()}</DataList.ItemValue>
          </DataList.Item>
        )}
        {details?.source && (
          <DataList.Item>
            <DataList.ItemLabel>{t(($) => $.drawer.source)}</DataList.ItemLabel>
            <DataList.ItemValue>
              <bdi>{details.source}</bdi>
            </DataList.ItemValue>
          </DataList.Item>
        )}
      </DataList>
      <Text size="xs" c="var(--qc-text-2)">
        {originText}
      </Text>
      {canManage(rowSubject(row), 'metadata') && (
        <Button
          variant="default"
          size="compact-md"
          w="fit-content"
          leftSection={<IconPencil size={16} stroke={1.8} />}
          onClick={() => setEditing(true)}
        >
          {t(($) => $.drawer.edit)}
        </Button>
      )}
    </Stack>
  )
}
