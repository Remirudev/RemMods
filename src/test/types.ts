import type { QueryClient } from '@tanstack/react-query'
import type { PlatformInfo, Settings } from '../lib/ipc/types'

export interface IpcCall {
  command: string
  payload: unknown
}

export interface IpcMock {
  calls: IpcCall[]
}

export interface RenderOptions {
  settings?: Settings | null
  platformInfo?: PlatformInfo | null
}

export interface RenderedWithProviders {
  queryClient: QueryClient
}
