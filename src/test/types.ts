import type { QueryClient } from '@tanstack/react-query'
import type {
  GameStatus,
  ModContents,
  ModDetails,
  ModsSnapshot,
  PlatformInfo,
  Settings,
} from '../lib/ipc/types'

export interface IpcCall {
  command: string
  payload: unknown
}

// A handler may return a value, return a promise or throw an AppError object to reject.
export type IpcHandler = (payload: unknown) => unknown

export interface IpcMockOptions {
  gameStatus?: GameStatus
  snapshot?: ModsSnapshot
  details?: ModDetails
  contents?: ModContents
  handlers?: Record<string, IpcHandler>
}

export interface IpcMock {
  calls: IpcCall[]
  callsOf: (command: string) => IpcCall[]
}

export interface RenderOptions {
  settings?: Settings | null
  platformInfo?: PlatformInfo | null
  gameStatus?: GameStatus | null
}

export interface RenderedWithProviders {
  queryClient: QueryClient
  unmount: () => void
}
