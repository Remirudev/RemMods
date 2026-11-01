import { Center, Loader, Stack } from '@mantine/core'
import { useHotkeys } from '@mantine/hooks'
import { useEffect, useRef } from 'react'
import { useDetectGame } from '../../shared/game/queries'
import { AutoRefreshBanner } from './AutoRefreshBanner'
import { ModDetailsDrawer } from './ModDetailsDrawer'
import { ModsErrorAlert } from './ModsErrorAlert'
import { ModsHeader } from './ModsHeader'
import { ModsTabs } from './ModsTabs'
import { OriginalAppNotice } from './OriginalAppNotice'
import { useModsSnapshot, useRefreshMods } from './queries'
import { RootNotices } from './RootNotices'
import { isScanNotFound } from './scan-error'
import type { ModsPanelProps } from './types'
import { UpdateNotice } from './UpdateNotice'
import { useReconcileRefs } from './use-reconcile-refs'

export function ModsPanel({ status, install }: ModsPanelProps) {
  const snapshot = useModsSnapshot(true)
  const refresh = useRefreshMods()
  const detect = useDetectGame()
  const searchRef = useRef<HTMLInputElement>(null)
  const redetected = useRef(false)
  const data = snapshot.data
  useReconcileRefs(data?.rows)
  const scanNotFound = snapshot.isError && isScanNotFound(snapshot.error)

  // A vanished Paks folder means the saved location is wrong: detection re-validates it, which
  // turns Ready into Missing and shows the onboarding. It runs once until a scan succeeds again,
  // so a location that stays Ready cannot loop between failing scans and detections.
  useEffect(() => {
    if (data) redetected.current = false
    if (!scanNotFound || redetected.current) return
    redetected.current = true
    detect.mutate()
  }, [data, scanNotFound, detect.mutate])

  const startRefresh = () => refresh.mutate()
  useHotkeys(
    [
      ['mod+F', () => searchRef.current?.focus()],
      ['F5', startRefresh],
    ],
    [],
  )

  return (
    <>
      <Stack gap="md" flex={1} mih={0}>
        <ModsHeader
          snapshot={data}
          running={status.running}
          tracking={install.updateTracking}
          refreshing={refresh.isPending}
          onRefresh={startRefresh}
        />
        <AutoRefreshBanner />
        {status.update && <UpdateNotice update={status.update} />}
        {status.originalApp && <OriginalAppNotice />}
        {data && <RootNotices roots={data.roots} />}
        {data ? (
          <ModsTabs snapshot={data} paks={install.paks} searchRef={searchRef} />
        ) : snapshot.isError ? (
          <ModsErrorAlert error={snapshot.error} onRetry={() => void snapshot.refetch()} />
        ) : (
          <Center flex={1}>
            <Loader size="md" />
          </Center>
        )}
      </Stack>
      <ModDetailsDrawer rows={data?.rows ?? []} />
    </>
  )
}
