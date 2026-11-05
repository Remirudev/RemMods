import type { Channel } from '@tauri-apps/api/core'
import { commands } from './bindings'
import { unwrapResult } from './ipc-error'
import type {
  ApplyDirection,
  ApplyRequest,
  AppUpdateProgress,
  BulkDeleteRequest,
  BulkTrashRequest,
  DeleteRequest,
  GraphicsMode,
  InspectRequest,
  InstallProgress,
  LocationChoice,
  MetadataRequest,
  ModTarget,
  ProfileSave,
  RenameRequest,
  SettingsUpdate,
  SmokeReport,
  StartRequest,
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
  modsItems: (modRef: string, language: string) =>
    unwrapResult('mods_items', () => commands.modsItems(modRef, language)),
  assetsStatus: (language: string) =>
    unwrapResult('assets_status', () => commands.assetsStatus(language)),
  assetsRefresh: (language: string) =>
    unwrapResult('assets_refresh', () => commands.assetsRefresh(language)),
  assetsPicture: (id: string) => unwrapResult('assets_picture', () => commands.assetsPicture(id)),
  pickPicture: (title: string) => unwrapResult('pick_picture', () => commands.pickPicture(title)),
  modsSetPicture: (target: ModTarget, source: string) =>
    unwrapResult('mods_set_picture', () => commands.modsSetPicture(target, source)),
  modsClearPicture: (target: ModTarget) =>
    unwrapResult('mods_clear_picture', () => commands.modsClearPicture(target)),
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
  modsTrashMany: (request: BulkTrashRequest) =>
    unwrapResult('mods_trash_many', () => commands.modsTrashMany(request)),
  modsDeletePrepareMany: (targets: ModTarget[]) =>
    unwrapResult('mods_delete_prepare_many', () => commands.modsDeletePrepareMany(targets)),
  modsDeleteMany: (request: BulkDeleteRequest) =>
    unwrapResult('mods_delete_many', () => commands.modsDeleteMany(request)),
  modsBusyCancel: (waitId: number) =>
    unwrapResult('mods_busy_cancel', () => commands.modsBusyCancel(waitId)),
  modsSetMetadata: (request: MetadataRequest) =>
    unwrapResult('mods_set_metadata', () => commands.modsSetMetadata(request)),
  repairList: () => unwrapResult('repair_list', () => commands.repairList()),
  profilesList: () => unwrapResult('profiles_list', () => commands.profilesList()),
  profilesSave: (save: ProfileSave) =>
    unwrapResult('profiles_save', () => commands.profilesSave(save)),
  profilesDelete: (id: string) =>
    unwrapResult('profiles_delete', () => commands.profilesDelete(id)),
  profilesDiff: (id: string, direction: ApplyDirection) =>
    unwrapResult('profiles_diff', () => commands.profilesDiff(id, direction)),
  profilesApply: (request: ApplyRequest) =>
    unwrapResult('profiles_apply', () => commands.profilesApply(request)),
  profilesTransfer: (request: TransferRequest) =>
    unwrapResult('profiles_transfer', () => commands.profilesTransfer(request)),
  pickArchives: (title: string) =>
    unwrapResult('pick_archives', () => commands.pickArchives(title)),
  pickFolder: (title: string) => unwrapResult('pick_folder', () => commands.pickFolder(title)),
  installInspect: (request: InspectRequest) =>
    unwrapResult('install_inspect', () => commands.installInspect(request)),
  installStart: (request: StartRequest, progress: Channel<InstallProgress>) =>
    unwrapResult('install_start', () => commands.installStart(request, progress)),
  installCancel: () => unwrapResult('install_cancel', () => commands.installCancel()),
  modsWatchStatus: () => unwrapResult('mods_watch_status', () => commands.modsWatchStatus()),
  updateChannel: () => unwrapResult('update_channel', () => commands.updateChannel()),
  updateCheck: () => unwrapResult('update_check', () => commands.updateCheck()),
  updateInstall: (version: string, progress: Channel<AppUpdateProgress>) =>
    unwrapResult('update_install', () => commands.updateInstall(version, progress)),
  graphicsGet: () => unwrapResult('graphics_get', () => commands.graphicsGet()),
  graphicsSetMode: (mode: GraphicsMode) =>
    unwrapResult('graphics_set_mode', () => commands.graphicsSetMode(mode)),
  graphicsAckStep: (revert: boolean) =>
    unwrapResult('graphics_ack_step', () => commands.graphicsAckStep(revert)),
  aboutNotices: () => unwrapResult('about_notices', () => commands.aboutNotices()),
}
