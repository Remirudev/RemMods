import { useEffect } from 'react'
import { useSettings, useSetUiScale } from '../features/settings/queries'
import { nextUiScale, uiScaleActionForKey } from '../shared/ui-scale'

// Tauri's own zoom keys are off (`zoomHotkeysEnabled: false`), so these go through settings and
// `set_ui_scale` like the slider. Read-only settings cannot be saved, so the keys do nothing then.
export function useUiScaleHotkeys(): void {
  const settings = useSettings().data
  const currentScale = settings?.readOnly ? undefined : settings?.appearance.uiScale
  const { mutate: setUiScale } = useSetUiScale()

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const action = uiScaleActionForKey(event)
      if (!action) return
      event.preventDefault()
      if (currentScale !== undefined) setUiScale(nextUiScale(currentScale, action))
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [currentScale, setUiScale])
}
