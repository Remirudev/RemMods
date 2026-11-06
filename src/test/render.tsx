import { MantineProvider } from '@mantine/core'
import { ModalsProvider } from '@mantine/modals'
import { QueryClientProvider } from '@tanstack/react-query'
import { render } from '@testing-library/react'
import type { ReactElement } from 'react'
import { createQueryClient } from '../lib/query/client'
import { queryKeys } from '../shared/query-keys'
import { buildTheme } from '../shared/theme/build-theme'
import { cssVariablesResolver } from '../shared/theme/css-variables'
import { rememberPlatformOs } from '../shared/trash-place'
import { makeGameStatus, TEST_PLATFORM_INFO, TEST_SETTINGS } from './fixtures'
import type { RenderedWithProviders, RenderOptions } from './types'

const theme = buildTheme()

function withDefault<T>(value: T | null | undefined, fallback: T): T | null {
  return value === undefined ? fallback : value
}

// The app's real query client and theme; `env="test"` turns off Mantine transitions and portals.
export function renderWithProviders(
  ui: ReactElement,
  options: RenderOptions = {},
): RenderedWithProviders {
  const queryClient = createQueryClient()
  const settings = withDefault(options.settings, TEST_SETTINGS)
  const platformInfo = withDefault(options.platformInfo, TEST_PLATFORM_INFO)
  if (settings) queryClient.setQueryData(queryKeys.settings, settings)
  if (platformInfo) queryClient.setQueryData(queryKeys.platformInfo, platformInfo)
  rememberPlatformOs(platformInfo?.os ?? null)
  const gameStatus = withDefault(options.gameStatus, makeGameStatus())
  if (gameStatus) queryClient.setQueryData(queryKeys.gameStatus, gameStatus)

  const { unmount } = render(
    <QueryClientProvider client={queryClient}>
      <MantineProvider theme={theme} cssVariablesResolver={cssVariablesResolver} env="test">
        <ModalsProvider>{ui}</ModalsProvider>
      </MantineProvider>
    </QueryClientProvider>,
  )

  return { queryClient, unmount }
}
