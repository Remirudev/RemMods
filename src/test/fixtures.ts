import type { PlatformInfo, Settings } from '../lib/ipc/types'

export const TEST_SETTINGS: Settings = {
  version: 1,
  appearance: {
    scheme: 'dark',
    uiScale: 1,
    titleBar: 'custom',
  },
  readOnly: false,
  recovered: false,
}

export const TEST_PLATFORM_INFO: PlatformInfo = {
  os: 'linux',
  arch: 'x86_64',
  appVersion: '0.1.0',
  webviewVersion: '2.52.0',
  smoke: false,
  paths: {
    config: '/home/test/.config/com.remirudev.remmods',
    data: '/home/test/.local/share/com.remirudev.remmods',
    logs: '/home/test/.local/share/com.remirudev.remmods/logs',
  },
}
