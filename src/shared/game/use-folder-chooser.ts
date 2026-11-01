import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { usePickGameLocation } from './queries'
import type { FolderChooser } from './types'

// The native dialog is Rust-only: the UI sends the dialog title and gets back an outcome.
export function useFolderChooser(): FolderChooser {
  const { t } = useTranslation('onboarding')
  const pickLocation = usePickGameLocation()
  const [cancelled, setCancelled] = useState(false)
  const [invalid, setInvalid] = useState<FolderChooser['invalid']>(null)

  const pick = () => {
    pickLocation.mutate(
      t(($) => $.choose.dialogTitle),
      {
        onSuccess: (outcome) => {
          setCancelled(outcome.kind === 'cancelled')
          setInvalid(
            outcome.kind === 'invalid' ? { reason: outcome.reason, path: outcome.path } : null,
          )
        },
      },
    )
  }

  return { pick, picking: pickLocation.isPending, cancelled, invalid }
}
