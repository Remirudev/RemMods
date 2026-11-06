import { getDefaultZIndex, Image, Portal } from '@mantine/core'
import { useEffect } from 'react'
import { usePicturePreviewStore } from '../picture-preview-store'
import { previewBox } from '../preview-box'
import classes from './item-picture.module.css'

// Over everything and never under the pointer. `pointerleave` does not come when the picture goes
// away under a resting pointer (a key, a wheel, Alt+Tab), so while it shows those close it too.
export function PicturePreview() {
  const preview = usePicturePreviewStore((state) => state.preview)
  const hidePreview = usePicturePreviewStore((state) => state.hidePreview)
  const shown = preview !== null

  useEffect(() => {
    if (!shown) return
    const close = () => hidePreview()
    const onOut = (event: MouseEvent) => {
      if (event.relatedTarget === null) close()
    }
    const options = { capture: true, passive: true }
    window.addEventListener('pointerdown', close, options)
    window.addEventListener('keydown', close, options)
    window.addEventListener('wheel', close, options)
    window.addEventListener('blur', close)
    document.addEventListener('mouseout', onOut, options)
    return () => {
      window.removeEventListener('pointerdown', close, options)
      window.removeEventListener('keydown', close, options)
      window.removeEventListener('wheel', close, options)
      window.removeEventListener('blur', close)
      document.removeEventListener('mouseout', onOut, options)
    }
  }, [shown, hidePreview])

  if (!preview) return null
  const viewport = { width: window.innerWidth, height: window.innerHeight }
  const box = previewBox(preview.anchor, preview.natural, viewport)

  return (
    <Portal>
      <div
        className={classes.preview}
        style={{ ...box, zIndex: getDefaultZIndex('max') }}
        data-testid="picture-preview"
        aria-hidden="true"
      >
        <Image src={preview.src} alt="" fit="contain" h="100%" draggable={false} />
      </div>
    </Portal>
  )
}
