import { Radio, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import type { InstallCandidate, VariantGroup } from '../../lib/ipc/types'
import { CandidateRow } from './CandidateRow'
import { pickVariant, setReplace, toggleCandidate } from './install-model'
import type { CandidateListProps } from './types'

type Section =
  | { kind: 'single'; candidate: InstallCandidate }
  | { kind: 'group'; group: VariantGroup; candidates: InstallCandidate[] }

// Candidates in order; the members of a variant group come together at the first member.
function sectionsOf(
  candidates: readonly InstallCandidate[],
  groups: readonly VariantGroup[],
): Section[] {
  const sections: Section[] = []
  const placed = new Set<number>()
  for (const candidate of candidates) {
    if (placed.has(candidate.id)) continue
    const group = groups.find((entry) => entry.candidates.includes(candidate.id))
    if (!group) {
      sections.push({ kind: 'single', candidate })
      continue
    }
    const members = candidates.filter((member) => group.candidates.includes(member.id))
    for (const member of members) placed.add(member.id)
    sections.push({ kind: 'group', group, candidates: members })
  }
  return sections
}

export function CandidateList({ inspection, choices, onChoices, onName }: CandidateListProps) {
  const { t } = useTranslation('install')

  const row = (candidate: InstallCandidate, inVariantGroup: boolean) => (
    <CandidateRow
      key={candidate.id}
      candidate={candidate}
      inVariantGroup={inVariantGroup}
      chosen={choices.selected.includes(candidate.id)}
      replace={choices.replace.includes(candidate.id)}
      autoFix={choices.autoFix}
      onToggle={() => onChoices(toggleCandidate(choices, candidate.id))}
      onReplace={(replace) => onChoices(setReplace(choices, candidate.id, replace))}
      onName={(name) => onName(candidate.id, name)}
    />
  )

  return (
    <Stack gap="xs">
      <Text fw={600} size="sm">
        {t(($) => $.candidates.title)}
      </Text>
      {sectionsOf(inspection.candidates, inspection.groups).map((section) => {
        if (section.kind === 'single') return row(section.candidate, false)
        const picked = section.candidates.find((candidate) =>
          choices.selected.includes(candidate.id),
        )
        return (
          <Radio.Group
            key={`group-${section.group.id}`}
            value={picked ? String(picked.id) : ''}
            description={t(($) => $.candidates.variantHint)}
            onChange={(value) => onChoices(pickVariant(inspection, choices, Number(value)))}
          >
            <Stack gap="xs" mt="xs">
              {section.candidates.map((candidate) => row(candidate, true))}
            </Stack>
          </Radio.Group>
        )
      })}
    </Stack>
  )
}
