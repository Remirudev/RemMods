import { commands } from './bindings'
import { unwrapResult } from './ipc-error'
import type { LocationChoice, SettingsUpdate, SmokeReport, ToggleRequest } from './types'

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
  gameStatus: () => unwrapResult('game_status', () => commands.gameStatus()),
  gameDetect: () => unwrapResult('game_detect', () => commands.gameDetect()),
  gameSetLocation: (choice: LocationChoice) =>
    unwrapResult('game_set_location', () => commands.gameSetLocation(choice)),
  gamePickLocation: (title: string) =>
    unwrapResult('game_pick_location', () => commands.gamePickLocation(title)),
  gameAckUpdate: () => unwrapResult('game_ack_update', () => commands.gameAckUpdate()),
  gameLaunch: () => unwrapResult('game_launch', () => commands.gameLaunch()),
  gameReveal: () => unwrapResult('game_reveal', () => commands.gameReveal()),
  modsList: () => unwrapResult('mods_list', () => commands.modsList()),
  modsRefresh: () => unwrapResult('mods_refresh', () => commands.modsRefresh()),
  modsDetails: (modRef: string) => unwrapResult('mods_details', () => commands.modsDetails(modRef)),
  modsReveal: (modRef: string) => unwrapResult('mods_reveal', () => commands.modsReveal(modRef)),
  modsToggle: (request: ToggleRequest) =>
    unwrapResult('mods_toggle', () => commands.modsToggle(request)),
  modsDisableAll: (applyWhileRunning: boolean) =>
    unwrapResult('mods_disable_all', () => commands.modsDisableAll(applyWhileRunning)),
  modsWatchStatus: () => unwrapResult('mods_watch_status', () => commands.modsWatchStatus()),
}
