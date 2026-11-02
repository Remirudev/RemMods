import { useQueryClient } from '@tanstack/react-query'
import { useCallback, useEffect, useRef, useState } from 'react'
import { ipc } from '../../lib/ipc'
import { describeForLog, isIpcError } from '../../lib/ipc/ipc-error'
import type { Inspection, InstallOutcome, NameEncoding, RootKind } from '../../lib/ipc/types'
import { logError } from '../../lib/log/frontend-log'
import { storeModsSnapshot } from '../../shared/mods-cache'
import type { QueuedSource } from '../../shared/types'
import {
  buildStartRequest,
  carryChoices,
  DEFAULT_OPTIONS,
  initialChoices,
  toInspectRequest,
  withName,
} from './install-model'
import type { FlowStage, InspectOptions, InstallChoices, InstallFlow, PreviewNotice } from './types'
import { useInstallJob } from './use-install-job'

type Keep = 'reset' | 'carry'

// A refusal that means the disk or the inspected targets moved: inspect again and show the new
// preview, once per attempt.
function movedUnderneath(error: unknown): boolean {
  if (!isIpcError(error)) return false
  const appError = error.appError
  return (
    appError.kind === 'stale' ||
    (appError.kind === 'installBlocked' && appError.reason === 'targetChanged')
  )
}

// The install modal's state machine: inspect, preview, run, then done, cancelled or failed.
export function useInstallFlow(source: QueuedSource): InstallFlow {
  const client = useQueryClient()
  const job = useInstallJob()
  const [stage, setStage] = useState<FlowStage>('inspecting')
  const [options, setOptions] = useState<InspectOptions>(DEFAULT_OPTIONS)
  const [inspection, setInspection] = useState<Inspection | null>(null)
  const [choices, setChoices] = useState<InstallChoices | null>(null)
  const [notice, setNotice] = useState<PreviewNotice | null>(null)
  const [outcome, setOutcome] = useState<InstallOutcome | null>(null)
  const [error, setError] = useState<unknown>(null)
  const [reinspecting, setReinspecting] = useState(false)
  const latest = useRef(0)

  // Only the newest request may change the state: options change while an earlier one is
  // still being read.
  const inspect = useCallback(
    async (next: InspectOptions, keep: Keep) => {
      const ticket = ++latest.current
      setOptions(next)
      setReinspecting(true)
      try {
        const result = await ipc.installInspect(toInspectRequest(source, next))
        if (ticket !== latest.current) return
        setInspection(result)
        setChoices((previous) =>
          keep === 'carry' && previous ? carryChoices(previous, result) : initialChoices(result),
        )
        setStage('preview')
      } catch (reason) {
        if (ticket !== latest.current) return
        logError(`install_inspect failed: ${describeForLog(reason)}`)
        setError(reason)
        setStage('failed')
      } finally {
        if (ticket === latest.current) setReinspecting(false)
      }
    },
    [source],
  )

  useEffect(() => {
    void inspect(DEFAULT_OPTIONS, 'reset')
  }, [inspect])

  const handleStartFailure = useCallback(
    (reason: unknown) => {
      const kind = isIpcError(reason) ? reason.appError.kind : null
      if (kind === 'gameRunning') {
        setStage('confirmRunning')
      } else if (kind === 'installCancelled') {
        setStage('cancelled')
      } else if (kind === 'installBusy') {
        setNotice('busy')
        setStage('preview')
      } else if (movedUnderneath(reason)) {
        setNotice('stale')
        setStage('preview')
        void inspect(options, 'carry')
      } else {
        logError(`install_start failed: ${describeForLog(reason)}`)
        setError(reason)
        setStage('failed')
      }
    },
    [inspect, options],
  )

  const start = useCallback(
    (applyWhileRunning = false) => {
      if (!inspection || !choices) return
      setNotice(null)
      setStage('running')
      job.start(buildStartRequest(inspection, choices, applyWhileRunning)).then((result) => {
        storeModsSnapshot(client, result.snapshot)
        setOutcome(result)
        setStage('done')
      }, handleStartFailure)
    },
    [inspection, choices, job.start, client, handleStartFailure],
  )

  return {
    stage,
    options,
    inspection,
    choices,
    notice,
    outcome,
    error,
    reinspecting,
    progress: job.progress,
    setRoot: (root: RootKind) => void inspect({ ...options, root, names: [] }, 'reset'),
    setEncoding: (encoding: NameEncoding) =>
      void inspect({ ...options, encoding, names: [] }, 'reset'),
    setName: (id, name) => void inspect(withName(options, id, name), 'carry'),
    setChoices,
    start,
    cancel: job.cancel,
    backToPreview: () => setStage('preview'),
  }
}
