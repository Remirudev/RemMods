import { Center, Loader, ScrollArea } from '@mantine/core'
import { Activity, Suspense } from 'react'
import { useTranslation } from 'react-i18next'
import { useActivePanelStore } from './active-panel-store'
import { ErrorBoundary } from './ErrorBoundary'
import classes from './panel-host.module.css'
import { PANEL_IDS, panelRegistry } from './panels'

const PANEL_LOADING = (
  <Center py="xl">
    <Loader size="sm" />
  </Center>
)

// Panels load on first visit and then stay mounted but hidden, so switching back keeps their state.
export function PanelHost() {
  const { t } = useTranslation('shell')
  const activePanel = useActivePanelStore((state) => state.activePanel)
  const visited = useActivePanelStore((state) => state.visited)

  return (
    <div className={classes.host}>
      {PANEL_IDS.filter((id) => visited.includes(id)).map((id) => {
        const { component: Panel, layout } = panelRegistry[id]
        const body = (
          <ErrorBoundary area={t(($) => $.panels[id].title)}>
            <Suspense fallback={PANEL_LOADING}>
              <Panel />
            </Suspense>
          </ErrorBoundary>
        )
        return (
          <Activity key={id} mode={id === activePanel ? 'visible' : 'hidden'}>
            {layout === 'fill' ? (
              <div className={classes.fill}>
                <div className={classes.fillContent}>{body}</div>
              </div>
            ) : (
              <ScrollArea className={classes.pane}>
                <div className={classes.content}>{body}</div>
              </ScrollArea>
            )}
          </Activity>
        )
      })}
    </div>
  )
}
