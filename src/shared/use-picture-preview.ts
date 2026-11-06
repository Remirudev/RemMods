import { type PointerEvent, useCallback, useEffect, useId, useRef } from 'react'
import { usePicturePreviewStore } from './picture-preview-store'
import type { PicturePreviewHandlers } from './types'
import { useHoverPreview } from './use-hover-preview'

const OPEN_DELAY_MS = 500

// A big picture while the mouse rests on this one. A press or a wheel means the user is busy with
// something else, so they end the wait.
export function usePicturePreview(): PicturePreviewHandlers {
  const enabled = useHoverPreview()
  const owner = useId()
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined)

  const cancel = useCallback(() => clearTimeout(timer.current), [])
  const leave = useCallback(() => {
    cancel()
    const { preview, hidePreview } = usePicturePreviewStore.getState()
    if (preview?.owner === owner) hidePreview()
  }, [cancel, owner])

  // A virtual list can drop a picture under the pointer, and then no `pointerleave` comes.
  useEffect(() => leave, [leave])

  const onPointerEnter = (event: PointerEvent<HTMLElement>) => {
    if (!enabled || event.pointerType !== 'mouse') return
    const tile = event.currentTarget
    cancel()
    timer.current = setTimeout(() => {
      const image = tile.querySelector('img')
      if (!image || image.naturalWidth === 0) return
      const { left, right, top, bottom } = tile.getBoundingClientRect()
      usePicturePreviewStore.getState().showPreview({
        owner,
        src: image.src,
        natural: { width: image.naturalWidth, height: image.naturalHeight },
        anchor: { left, right, top, bottom },
      })
    }, OPEN_DELAY_MS)
  }

  return { onPointerEnter, onPointerLeave: leave, onPointerDown: cancel, onWheel: cancel }
}
