import type { AppError } from './bindings'

export type {
  AppError,
  Appearance,
  AppearanceUpdate,
  AppPathsDto,
  ColorScheme,
  OsErrorClass,
  OsKind,
  PlatformInfo,
  Settings,
  SettingsUpdate,
  SmokeCheck,
  SmokeReport,
  TitleBarMode,
} from './bindings'

export type CommandResult<T> = { status: 'ok'; data: T } | { status: 'error'; error: AppError }
