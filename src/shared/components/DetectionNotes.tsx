import { Accordion, List, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import type { DetectionNote } from '../../lib/ipc/types'
import { isNoteDetailKey, isSteamKindKey } from '../game/note-detail'
import type { DetectionNotesProps } from './types'

export function DetectionNotes({ notes }: DetectionNotesProps) {
  const { t } = useTranslation('game')

  const detailText = (note: DetectionNote): string | null => {
    const detail = note.detail
    if (detail === null) return null
    if (note.code === 'steamRoot') {
      return isSteamKindKey(detail) ? t(($) => $.steamKinds[detail]) : null
    }
    return isNoteDetailKey(detail) ? t(($) => $.notes.details[detail]) : null
  }

  const noteText = (note: DetectionNote): string => {
    const parts: string[] = [t(($) => $.notes.codes[note.code], { path: note.path })]
    if (note.line !== null) parts.push(t(($) => $.notes.line, { line: note.line }))
    const detail = detailText(note)
    if (detail) parts.push(detail)
    return parts.join(' · ')
  }

  return (
    <Accordion variant="contained" data-testid="detection-notes">
      <Accordion.Item value="notes">
        <Accordion.Control>{t(($) => $.notes.title)}</Accordion.Control>
        <Accordion.Panel>
          {notes.length === 0 ? (
            <Text size="sm">{t(($) => $.notes.empty)}</Text>
          ) : (
            <List spacing="xs" size="sm">
              {notes.map((note) => (
                <List.Item
                  key={`${note.code}|${note.path}|${note.line ?? ''}|${note.detail ?? ''}`}
                >
                  <bdi>{noteText(note)}</bdi>
                </List.Item>
              ))}
            </List>
          )}
        </Accordion.Panel>
      </Accordion.Item>
    </Accordion>
  )
}
