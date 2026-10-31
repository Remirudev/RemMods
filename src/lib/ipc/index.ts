import { commands } from './bindings'
import { unwrapResult } from './ipc-error'
import type { SettingsUpdate, SmokeReport } from './types'

export const ipc = {
  platformInfo: () => unwrapResult('platform_info', () => commands.platformInfo()),
  settingsGet: () => unwrapResult('settings_get', () => commands.settingsGet()),
  settingsUpdate: (update: SettingsUpdate) =>
    unwrapResult('settings_update', () => commands.settingsUpdate(update)),
  setUiScale: (scale: number) => unwrapResult('set_ui_scale', () => commands.setUiScale(scale)),
  appReady: () => unwrapResult('app_ready', () => commands.appReady()),
  openLogs: () => unwrapResult('open_logs', () => commands.openLogs()),
  smokeReport: (report: SmokeReport) =>
    unwrapResult('smoke_report', () => commands.smokeReport(report)),
}
