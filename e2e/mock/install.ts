import { emit } from '@tauri-apps/api/event'
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks'
import { createBackend } from './backend'
import { createState, parseScenario } from './state'
import type { CommandArgs } from './types'

// Injected first into index.html by `vite --mode e2e` (never in a build), so the stand-in backend
// is in place before src/main.tsx asks for platform_info. No top-level await: module scripts run
// in document order only when each finishes synchronously.
const scenario = parseScenario(new URLSearchParams(window.location.search).get('scenario'))
const backend = createBackend(createState(scenario), (event, payload) => emit(event, payload))

mockWindows('main')
mockIPC((cmd, args) => backend.route(cmd, (args ?? {}) as CommandArgs), {
  shouldMockEvents: true,
})

window.__REMMODS_E2E__ = backend.handle
