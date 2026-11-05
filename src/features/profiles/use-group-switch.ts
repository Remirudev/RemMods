import { useCallback, useState } from 'react'
import type { ApplyDirection, Profile } from '../../lib/ipc/types'
import { showApplyToast } from './ApplyToast'
import { applyDirect } from './group-apply'
import { useGroupPrefs } from './group-prefs-store'
import { openSwitchDiff } from './profile-modals'
import { useApplyProfile } from './use-profile-mutations'

// A group switch: the change list first (the default), or straight away when the user turned
// "Ask before switching" off. Both end in a toast whose Undo restores the previous state.
export function useGroupSwitch() {
  const ask = useGroupPrefs((state) => state.askBeforeSwitch)
  const { mutateAsync } = useApplyProfile()
  const [busyId, setBusyId] = useState<string | null>(null)

  const switchGroup = useCallback(
    (profile: Profile, direction: ApplyDirection) => {
      if (ask) {
        openSwitchDiff(profile, direction)
        return
      }
      setBusyId(profile.id)
      void applyDirect(mutateAsync, profile.id, direction, {
        onDone: showApplyToast,
        onStale: () => openSwitchDiff(profile, direction),
      }).finally(() => setBusyId(null))
    },
    [ask, mutateAsync],
  )

  return { switchGroup, busyId }
}
