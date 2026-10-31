import { useRef } from 'react'
import { useSettings } from '../../features/settings/queries'
import type { TitleBarMode } from '../../lib/ipc/types'

// The window frame is chosen when Rust creates the window, so a changed setting only applies after
// a restart. The mode first seen in this session is kept; switching live would leave the window
// with neither frame.
export function useTitleBarMode(): TitleBarMode {
  const live = useSettings().data?.appearance.titleBar
  const sessionMode = useRef<TitleBarMode | null>(null)
  if (sessionMode.current === null && live !== undefined) sessionMode.current = live
  return sessionMode.current ?? 'custom'
}
