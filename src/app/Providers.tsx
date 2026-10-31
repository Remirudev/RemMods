import { MantineProvider } from '@mantine/core'
import { ModalsProvider } from '@mantine/modals'
import { Notifications } from '@mantine/notifications'
import { QueryClientProvider } from '@tanstack/react-query'
import { buildTheme } from '../shared/theme/build-theme'
import { cssVariablesResolver } from '../shared/theme/css-variables'
import type { ProvidersProps } from './types'

const theme = buildTheme()

export function Providers({ queryClient, colorSchemeManager, children }: ProvidersProps) {
  return (
    <QueryClientProvider client={queryClient}>
      <MantineProvider
        theme={theme}
        colorSchemeManager={colorSchemeManager}
        defaultColorScheme="dark"
        cssVariablesResolver={cssVariablesResolver}
      >
        <ModalsProvider>
          <Notifications position="bottom-right" />
          {children}
        </ModalsProvider>
      </MantineProvider>
    </QueryClientProvider>
  )
}
