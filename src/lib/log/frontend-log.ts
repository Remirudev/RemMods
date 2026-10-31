import { error as writeErrorToLog } from '@tauri-apps/plugin-log'

// Goes to the Rust log file. Outside Tauri (tests, a plain browser) the call rejects; ignoring
// that keeps logging from ever becoming the error.
export function logError(message: string): void {
  writeErrorToLog(message).catch(() => undefined)
}
