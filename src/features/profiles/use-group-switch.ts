import { useCallback, useState } from 'react'
import type { ApplyDirection, Profile } from '../../lib/ipc/types'
import { showApplyToast } from './ApplyToast'
import { applyDirect } from './group-apply'
import { useGroupPrefs } from './group-prefs-store'
import { openSwitchDiff } from './profile-modals'
import { useApplyProfile } from './use-profile-mutations'

// A switch of one group or several: the change list first (the default), or straight away when the
// user turned "Ask before switching" off. Several groups are one batch, so both ways end in one
// toast whose Undo restores the previous state.
export function useGroupSwitch() {
  const ask = useGroupPrefs((state) => state.askBeforeSwitch)
  const { mutateAsync } = useApplyProfile()
  const [busyIds, setBusyIds] = useState<readonly string[]>([])

  const switchGroups = useCallback(
    (profiles: readonly Profile[], direction: ApplyDirection) => {
      if (profiles.length === 0) return
      if (ask) {
        openSwitchDiff(profiles, direction)
        return
      }
      const ids = profiles.map((profile) => profile.id)
      setBusyIds(ids)
      void applyDirect(mutateAsync, ids, direction, {
        onDone: showApplyToast,
        onStale: () => openSwitchDiff(profiles, direction),
      }).finally(() => setBusyIds([]))
    },
    [ask, mutateAsync],
  )

  return { switchGroups, busyIds }
}
