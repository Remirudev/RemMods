import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { ipc } from '../lib/ipc'
import type { ProfilesView } from '../lib/ipc/types'
import { queryKeys } from './query-keys'
import type { QuickApplyChoice, QuickApplyState } from './types'

type Opener = (profileId: string) => void

let opener: Opener | null = null

// The app registers the Profiles panel's apply modal here, so the Mods feature can open it
// without importing another feature.
export function registerProfileApplyOpener(open: Opener): () => void {
  opener = open
  return () => {
    if (opener === open) opener = null
  }
}

export function openProfileApply(profileId: string): boolean {
  if (!opener) return false
  opener(profileId)
  return true
}

/** Every profile except snapshots, sorted by name (numeric Intl.Collator of the UI language). */
export function quickApplyChoices(
  view: ProfilesView | null | undefined,
  locale: string,
): QuickApplyChoice[] {
  const collator = new Intl.Collator(locale, { sensitivity: 'base', numeric: true })
  return (view?.profiles ?? [])
    .filter((summary) => summary.profile.source.kind !== 'snapshot')
    .map(({ profile }) => ({
      id: profile.id,
      name: profile.name,
      mode: profile.mode,
      count: profile.mods.length,
    }))
    .sort((a, b) => collator.compare(a.name, b.name))
}

export function quickApplyState(
  view: ProfilesView | null | undefined,
  locale: string,
): QuickApplyState {
  return {
    choices: quickApplyChoices(view, locale),
    lastAppliedId: view?.lastApplied?.profileId ?? null,
    readOnly: view?.readOnly ?? false,
  }
}

// The same query key and function as the Profiles panel, so both read one cache entry.
export function useProfileQuickApply(): QuickApplyState & {
  loading: boolean
  failed: boolean
  open: (profileId: string) => void
} {
  const { i18n } = useTranslation()
  const query = useQuery({ queryKey: queryKeys.profiles, queryFn: () => ipc.profilesList() })
  const locale = i18n.language
  const state = useMemo(() => quickApplyState(query.data, locale), [query.data, locale])
  return {
    ...state,
    loading: query.isPending,
    failed: query.isError && query.data === undefined,
    open: (profileId) => void openProfileApply(profileId),
  }
}
