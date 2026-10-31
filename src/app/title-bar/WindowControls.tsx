import { UnstyledButton } from '@mantine/core'
import { IconCopy, IconMinus, IconSquare, IconX } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { runWindowAction } from './current-window'
import classes from './title-bar.module.css'
import { useWindowMaximized } from './use-window-maximized'

export function WindowControls() {
  const { t } = useTranslation('shell')
  const maximized = useWindowMaximized()
  const minimizeLabel = t(($) => $.titleBar.minimize)
  const maximizeLabel = maximized ? t(($) => $.titleBar.restore) : t(($) => $.titleBar.maximize)
  const closeLabel = t(($) => $.titleBar.close)

  return (
    <div className={classes.controls}>
      <UnstyledButton
        className={classes.control}
        aria-label={minimizeLabel}
        title={minimizeLabel}
        onClick={() => runWindowAction((appWindow) => appWindow.minimize())}
      >
        <IconMinus size={16} stroke={1.8} />
      </UnstyledButton>
      <UnstyledButton
        className={classes.control}
        aria-label={maximizeLabel}
        title={maximizeLabel}
        onClick={() => runWindowAction((appWindow) => appWindow.toggleMaximize())}
      >
        {maximized ? <IconCopy size={14} stroke={1.8} /> : <IconSquare size={14} stroke={1.8} />}
      </UnstyledButton>
      <UnstyledButton
        className={`${classes.control} ${classes.close}`}
        aria-label={closeLabel}
        title={closeLabel}
        onClick={() => runWindowAction((appWindow) => appWindow.close())}
      >
        <IconX size={16} stroke={1.8} />
      </UnstyledButton>
    </div>
  )
}
