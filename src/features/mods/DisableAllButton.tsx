import { Button } from '@mantine/core'
import { modals } from '@mantine/modals'
import { useTranslation } from 'react-i18next'
import { useAckUpdate } from '../../shared/game/queries'
import { useModsSnapshot } from './queries'
import { disableAllCount } from './toggle-model'
import { useDisableAllMods } from './use-toggle-mods'

export function DisableAllButton() {
  const { t } = useTranslation('mods')
  const rows = useModsSnapshot(false).data?.rows
  const acknowledge = useAckUpdate()
  const disableAll = useDisableAllMods(() => acknowledge.mutate())
  const count = rows ? disableAllCount(rows) : 0

  const confirm = () =>
    modals.openConfirmModal({
      title: t(($) => $.disableAll.confirmTitle),
      children: t(($) => $.disableAll.confirmBody, { count }),
      labels: {
        confirm: t(($) => $.disableAll.confirm),
        cancel: t(($) => $.disableAll.cancel),
      },
      onConfirm: () => disableAll.mutate(false),
    })

  return (
    <Button
      variant="default"
      size="compact-md"
      loading={disableAll.isPending}
      disabled={rows === undefined}
      onClick={confirm}
    >
      {t(($) => $.disableAll.button)}
    </Button>
  )
}
