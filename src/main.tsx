import '@mantine/core/styles.layer.css'
import '@mantine/notifications/styles.layer.css'
import '@mantine/spotlight/styles.layer.css'
import '@fontsource-variable/inter'
import '@fontsource-variable/jetbrains-mono'
import './shared/theme/tokens.css'
import './lib/i18n'

import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { App } from './app/App'
import { preloadAppData } from './app/bootstrap'
import { appColorScheme, bindColorSchemeToSettings } from './app/color-scheme-binding'
import { ErrorBoundary } from './app/ErrorBoundary'
import { installInputGuards } from './app/input-guards'
import { Providers } from './app/Providers'
import { queryClient } from './lib/query/client'

async function start(): Promise<void> {
  const container = document.getElementById('root')
  if (!container) throw new Error('index.html has no #root element')

  installInputGuards(window, { blockBrowserKeys: import.meta.env.PROD })
  bindColorSchemeToSettings(queryClient)
  const preloaded = await preloadAppData(queryClient)
  const smoke = preloaded.platformInfo?.smoke ? await import('./app/smoke/run-smoke') : null
  smoke?.startSmokeErrorCollection()

  createRoot(container).render(
    <StrictMode>
      <Providers queryClient={queryClient} colorSchemeManager={appColorScheme.manager}>
        <ErrorBoundary>
          <App />
        </ErrorBoundary>
      </Providers>
    </StrictMode>,
  )

  if (smoke) await smoke.runSmoke()
}

void start()
