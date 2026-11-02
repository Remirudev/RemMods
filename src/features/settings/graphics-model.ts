import type { GraphicsMode, GraphicsState, PlatformInfo } from '../../lib/ipc/types'

export const GRAPHICS_MODES: readonly GraphicsMode[] = ['auto', 'forceShm', 'software']

export function graphicsSupported(info: PlatformInfo | undefined): boolean {
  return info?.features.graphicsFallback === true
}

export function isGraphicsMode(value: string): value is GraphicsMode {
  return (GRAPHICS_MODES as readonly string[]).includes(value)
}

// The "always use software" offer only makes sense while the stored mode is something else.
export function offersSoftware(state: GraphicsState): boolean {
  return state.safeFlag && state.mode !== 'software' && !state.readOnly
}

export function canChangeMode(state: GraphicsState | undefined, pending: boolean): boolean {
  return state?.supported === true && !state.readOnly && !pending
}
