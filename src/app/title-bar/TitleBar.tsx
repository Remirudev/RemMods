import { useTranslation } from 'react-i18next'
import classes from './title-bar.module.css'
import { WindowControls } from './WindowControls'

// Tauri only drags (and double-click maximises) from the element that carries the attribute, not
// from its children, so the title text carries it too.
export function TitleBar() {
  const { t } = useTranslation('shell')

  return (
    <div className={classes.bar} data-tauri-drag-region data-testid="title-bar">
      <span className={classes.title} data-tauri-drag-region>
        {t(($) => $.titleBar.title)}
      </span>
      <WindowControls />
    </div>
  )
}
