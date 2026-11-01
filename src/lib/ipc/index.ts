import { commands } from './bindings'
import { unwrapResult } from './ipc-error'
import type {
  ApplyRequest,
  DeleteRequest,
  LocationChoice,
  MetadataRequest,
  ModTarget,
  ProfileSave,
  RenameRequest,
  SettingsUpdate,
  SmokeReport,
  ToggleRequest,
  TransferRequest,
  TrashRequest,
} from './types'

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
  modsContents: (modRef: string) =>
    unwrapResult('mods_contents', () => commands.modsContents(modRef)),
  modsReveal: (modRef: string) => unwrapResult('mods_reveal', () => commands.modsReveal(modRef)),
  modsToggle: (request: ToggleRequest) =>
    unwrapResult('mods_toggle', () => commands.modsToggle(request)),
  modsDisableAll: (applyWhileRunning: boolean) =>
    unwrapResult('mods_disable_all', () => commands.modsDisableAll(applyWhileRunning)),
  modsRename: (request: RenameRequest) =>
    unwrapResult('mods_rename', () => commands.modsRename(request)),
  modsTrash: (request: TrashRequest) =>
    unwrapResult('mods_trash', () => commands.modsTrash(request)),
  modsDeletePrepare: (target: ModTarget) =>
    unwrapResult('mods_delete_prepare', () => commands.modsDeletePrepare(target)),
  modsDeletePermanently: (request: DeleteRequest) =>
    unwrapResult('mods_delete_permanently', () => commands.modsDeletePermanently(request)),
  modsSetMetadata: (request: MetadataRequest) =>
    unwrapResult('mods_set_metadata', () => commands.modsSetMetadata(request)),
  repairList: () => unwrapResult('repair_list', () => commands.repairList()),
  profilesList: () => unwrapResult('profiles_list', () => commands.profilesList()),
  profilesSave: (save: ProfileSave) =>
    unwrapResult('profiles_save', () => commands.profilesSave(save)),
  profilesDelete: (id: string) =>
    unwrapResult('profiles_delete', () => commands.profilesDelete(id)),
  profilesDiff: (id: string) => unwrapResult('profiles_diff', () => commands.profilesDiff(id)),
  profilesApply: (request: ApplyRequest) =>
    unwrapResult('profiles_apply', () => commands.profilesApply(request)),
  profilesTransfer: (request: TransferRequest) =>
    unwrapResult('profiles_transfer', () => commands.profilesTransfer(request)),
  modsWatchStatus: () => unwrapResult('mods_watch_status', () => commands.modsWatchStatus()),
}
