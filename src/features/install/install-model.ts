import type {
  Inspection,
  InspectRequest,
  InstallCandidate,
  InstallSourceRef,
  StartRequest,
} from '../../lib/ipc/types'
import type { QueuedSource } from '../../shared/types'
import { hasIssue } from './install-issues'
import type { InspectOptions, InstallChoices, StartBlocker } from './types'

export const DEFAULT_OPTIONS: InspectOptions = { encoding: 'auto', root: null, names: [] }

function without(ids: readonly number[], removed: readonly number[]): number[] {
  return ids.filter((id) => !removed.includes(id))
}

function candidateById(inspection: Inspection, id: number): InstallCandidate | undefined {
  return inspection.candidates.find((candidate) => candidate.id === id)
}

// The backend preselects one candidate per variant group and every separate mod. Replacing an
// installed mod and resolving a case clash are always the user's decision.
export function initialChoices(inspection: Inspection): InstallChoices {
  return {
    selected: inspection.candidates.filter((candidate) => candidate.selected).map((c) => c.id),
    replace: [],
    clashKeeps: {},
    autoFix: true,
  }
}

// After a name edit the same candidates exist again; keep what the user decided where it still
// applies.
export function carryChoices(previous: InstallChoices, inspection: Inspection): InstallChoices {
  const known = new Set(inspection.candidates.map((candidate) => candidate.id))
  const selected = previous.selected.filter((id) => known.has(id))
  const clashKeeps: Record<number, number> = {}
  for (const clash of inspection.clashes) {
    const keep = previous.clashKeeps[clash.id]
    if (keep !== undefined && keep < clash.paths.length) clashKeeps[clash.id] = keep
  }
  return {
    selected: selected.length > 0 ? selected : initialChoices(inspection).selected,
    replace: previous.replace.filter(
      (id) => selected.includes(id) && (candidateById(inspection, id)?.existing ?? null) !== null,
    ),
    clashKeeps,
    autoFix: previous.autoFix,
  }
}

export function toggleCandidate(choices: InstallChoices, id: number): InstallChoices {
  if (choices.selected.includes(id)) {
    return {
      ...choices,
      selected: without(choices.selected, [id]),
      replace: without(choices.replace, [id]),
    }
  }
  return { ...choices, selected: [...choices.selected, id] }
}

// Variants of one group are alternatives: choosing one drops the others.
export function pickVariant(
  inspection: Inspection,
  choices: InstallChoices,
  id: number,
): InstallChoices {
  const group = inspection.groups.find((entry) => entry.candidates.includes(id))
  const others = group ? group.candidates.filter((other) => other !== id) : []
  return {
    ...choices,
    selected: [...without(choices.selected, others).filter((other) => other !== id), id],
    replace: without(choices.replace, others),
  }
}

export function setReplace(choices: InstallChoices, id: number, replace: boolean): InstallChoices {
  const rest = without(choices.replace, [id])
  return { ...choices, replace: replace ? [...rest, id] : rest }
}

export function keepClashPath(
  choices: InstallChoices,
  clash: number,
  keep: number,
): InstallChoices {
  return { ...choices, clashKeeps: { ...choices.clashKeeps, [clash]: keep } }
}

export function setAutoFix(choices: InstallChoices, autoFix: boolean): InstallChoices {
  return { ...choices, autoFix }
}

export function needsAutoFix(inspection: Inspection): boolean {
  return hasIssue(inspection.issues, 'invalidName', 'warn')
}

function selectedCandidates(inspection: Inspection, choices: InstallChoices): InstallCandidate[] {
  return inspection.candidates.filter((candidate) => choices.selected.includes(candidate.id))
}

export function startBlockers(inspection: Inspection, choices: InstallChoices): StartBlocker[] {
  const blockers: StartBlocker[] = []
  const selected = selectedCandidates(inspection, choices)
  if (inspection.blocked) blockers.push('blocked')
  if (selected.length === 0) blockers.push('noSelection')
  if (selected.some((candidate) => candidate.existing?.ambiguous)) blockers.push('ambiguous')
  if (
    selected.some(
      (candidate) =>
        candidate.existing &&
        !candidate.existing.ambiguous &&
        !choices.replace.includes(candidate.id),
    )
  ) {
    blockers.push('replaceNotConfirmed')
  }
  if (inspection.clashes.some((clash) => choices.clashKeeps[clash.id] === undefined)) {
    blockers.push('unresolvedClash')
  }
  if (!choices.autoFix && needsAutoFix(inspection)) blockers.push('invalidNames')
  if (hasIssue(inspection.issues, 'noSpace')) blockers.push('noSpace')
  return blockers
}

export function buildStartRequest(
  inspection: Inspection,
  choices: InstallChoices,
  applyWhileRunning = false,
): StartRequest {
  return {
    inspection: inspection.id,
    picks: selectedCandidates(inspection, choices).map((candidate) => ({
      id: candidate.id,
      replace: candidate.existing !== null && choices.replace.includes(candidate.id),
    })),
    clashes: inspection.clashes
      .filter((clash) => choices.clashKeeps[clash.id] !== undefined)
      .map((clash) => ({ id: clash.id, keep: choices.clashKeeps[clash.id] ?? 0 })),
    autoFix: choices.autoFix,
    applyWhileRunning,
  }
}

export function toSourceRef(source: QueuedSource): InstallSourceRef {
  return source.kind === 'token'
    ? { kind: 'token', id: source.id }
    : { kind: 'archive', row: source.row }
}

export function toInspectRequest(source: QueuedSource, options: InspectOptions): InspectRequest {
  return {
    source: toSourceRef(source),
    encoding: options.encoding,
    root: options.root,
    names: options.names,
  }
}

export function withName(options: InspectOptions, id: number, name: string): InspectOptions {
  const rest = options.names.filter((entry) => entry.id !== id)
  return { ...options, names: name === '' ? rest : [...rest, { id, name }] }
}

export function showsEncoding(inspection: Inspection): boolean {
  return inspection.legacyNames || hasIssue(inspection.issues, 'undecodable')
}
