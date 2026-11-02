import { useEffect } from 'react'
import { describeForLog } from '../lib/ipc/ipc-error'
import { logError } from '../lib/log/frontend-log'
import { registerProfileApplyOpener } from '../shared/profile-quick-apply'

// Quick-apply in the Mods header opens the Profiles panel's own diff → confirm → apply modal.
// The profiles code loads on first use, so the panel stays lazy and Mods never imports it.
export function registerProfileApplyBridge(): () => void {
  return registerProfileApplyOpener((profileId) => {
    import('../features/profiles/profile-modals').then(
      (module) => {
        if (!module.openProfileApplyById(profileId)) {
          logError(`quick-apply: profile ${profileId} is not in the list`)
        }
      },
      (error: unknown) => logError(`quick-apply failed to load: ${describeForLog(error)}`),
    )
  })
}

export function useProfileApplyBridge(): void {
  useEffect(() => registerProfileApplyBridge(), [])
}
