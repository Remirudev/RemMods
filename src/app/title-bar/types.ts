import type { getCurrentWindow } from '@tauri-apps/api/window'

export type AppWindow = ReturnType<typeof getCurrentWindow>

export type ResizeEdge =
  | 'North'
  | 'South'
  | 'East'
  | 'West'
  | 'NorthEast'
  | 'NorthWest'
  | 'SouthEast'
  | 'SouthWest'
