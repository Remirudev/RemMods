import { Checkbox, Group, Paper, Radio, Stack, Switch, Text, TextInput } from '@mantine/core'
import { useDebouncedCallback } from '@mantine/hooks'
import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isNameIssue } from '../../lib/i18n/error-messages'
import { formatSize } from '../../shared/format-size'
import classes from './install.module.css'
import type { CandidateRowProps } from './types'

const NAME_EDIT_DELAY_MS = 300

export function CandidateRow({
  candidate,
  inVariantGroup,
  chosen,
  replace,
  autoFix,
  onToggle,
  onReplace,
  onName,
}: CandidateRowProps) {
  const { t, i18n } = useTranslation('install')
  const { t: tMods } = useTranslation('mods')
  const [value, setValue] = useState(candidate.name)
  const sent = useRef(candidate.name)
  const sendName = useDebouncedCallback((name: string) => {
    sent.current = name
    onName(name)
  }, NAME_EDIT_DELAY_MS)

  // A name that did not come from this box (another root or encoding) replaces the text.
  useEffect(() => {
    if (candidate.name === sent.current) return
    sent.current = candidate.name
    setValue(candidate.name)
  }, [candidate.name])

  const label = t(($) => $.row.installLabel, { name: candidate.name })
  const existing = candidate.existing
  const issue = candidate.nameIssue
  const issueText = issue ? tMods(($) => $.nameIssue[isNameIssue(issue) ? issue : 'invalid']) : ''

  return (
    <Paper
      withBorder
      p="sm"
      className={chosen ? undefined : classes.dimmed}
      data-testid="candidate"
    >
      <Stack gap={6}>
        <Group wrap="nowrap" align="flex-start" gap="sm">
          {inVariantGroup ? (
            <Radio value={String(candidate.id)} aria-label={label} mt={8} />
          ) : (
            <Checkbox checked={chosen} onChange={onToggle} aria-label={label} mt={8} />
          )}
          <TextInput
            flex={1}
            value={value}
            aria-label={t(($) => $.candidates.nameLabel, { name: candidate.name })}
            onChange={(event) => {
              setValue(event.currentTarget.value)
              sendName(event.currentTarget.value)
            }}
          />
        </Group>
        <Text size="xs" c="var(--qc-text-2)" pl={34}>
          {[
            candidate.inner !== '' ? t(($) => $.candidates.from, { inner: candidate.inner }) : null,
            t(($) => $.summary.files, { count: candidate.files }),
            t(($) => $.candidates.payload, { count: candidate.payload }),
            formatSize(candidate.bytes, i18n.language),
          ]
            .filter((part) => part !== null)
            .join(' · ')}
        </Text>
        <Stack gap={2} pl={34}>
          {candidate.shippedOff > 0 && (
            <Text size="xs" c="var(--qc-text-2)">
              {t(($) => $.candidates.shippedOff, { count: candidate.shippedOff })}
            </Text>
          )}
          {candidate.fixedName !== null && (
            <Text size="xs" c="yellow">
              {autoFix
                ? t(($) => $.candidates.willRename, { name: candidate.fixedName })
                : `${t(($) => $.candidates.nameProblem)} ${issueText}`.trim()}
            </Text>
          )}
          {existing?.ambiguous && (
            <Text size="xs" c="red">
              {t(($) => $.candidates.ambiguous)}
            </Text>
          )}
          {existing && !existing.ambiguous && (
            <Switch
              size="xs"
              checked={replace}
              disabled={!chosen}
              onChange={(event) => onReplace(event.currentTarget.checked)}
              label={`${t(($) => $.candidates.replaces, { rel: existing.rel })} ${t(($) => $.candidates.replaceSwitch)}`}
            />
          )}
          {candidate.alsoIn.length > 0 && (
            <Text size="xs" c="var(--qc-text-2)">
              {t(($) => $.candidates.alsoIn, {
                roots: candidate.alsoIn.map((root) => t(($) => $.root.options[root])).join(', '),
              })}
            </Text>
          )}
          {candidate.pathTooLong && (
            <Text size="xs" c="yellow">
              {t(($) => $.candidates.pathTooLong)}
            </Text>
          )}
        </Stack>
      </Stack>
    </Paper>
  )
}
