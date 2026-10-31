import type { BrowserKeyEvent, InputGuardOptions } from './types'

const TEXT_INPUT_TYPES = new Set(['text', 'search', 'url', 'tel', 'email', 'password', 'number'])
const BLOCKED_FUNCTION_KEYS = new Set(['F3', 'F5', 'F7'])
const BLOCKED_CTRL_KEYS = new Set(['f', 'p', 'r'])
const BLOCKED_CTRL_CODES = new Set(['KeyF', 'KeyP', 'KeyR'])

export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  if (target.isContentEditable || target instanceof HTMLTextAreaElement) return true
  return target instanceof HTMLInputElement && TEXT_INPUT_TYPES.has(target.type)
}

// F5, Ctrl+R / Ctrl+Shift+R, Ctrl+P, F3, F7 and Ctrl+F are browser actions in the webview
// (reload, print, find, caret browsing). AltGr arrives as Ctrl+Alt and must keep typing. The
// physical key counts too: on a Cyrillic or Greek layout Ctrl+R types 'к', but WebView2 matches
// its accelerators by virtual key and would still reload.
export function isBlockedBrowserKey(event: BrowserKeyEvent): boolean {
  if (BLOCKED_FUNCTION_KEYS.has(event.key)) return true
  if (!event.ctrlKey || event.altKey || event.metaKey) return false
  return BLOCKED_CTRL_KEYS.has(event.key.toLowerCase()) || BLOCKED_CTRL_CODES.has(event.code)
}

// WebKitGTK's stock context menu offers Reload, so it is suppressed on both OSes; text fields keep
// theirs for copy and paste.
export function installInputGuards(target: Window, options: InputGuardOptions): () => void {
  const onContextMenu = (event: MouseEvent) => {
    if (!isEditableTarget(event.target)) event.preventDefault()
  }
  const onKeyDown = (event: KeyboardEvent) => {
    if (isBlockedBrowserKey(event)) event.preventDefault()
  }

  target.addEventListener('contextmenu', onContextMenu)
  if (options.blockBrowserKeys) target.addEventListener('keydown', onKeyDown, { capture: true })

  return () => {
    target.removeEventListener('contextmenu', onContextMenu)
    target.removeEventListener('keydown', onKeyDown, { capture: true })
  }
}
