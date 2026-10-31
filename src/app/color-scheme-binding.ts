import type { QueryClient } from '@tanstack/react-query'
import type { Settings } from '../lib/ipc/types'
import { queryKeys } from '../shared/query-keys'
import { createColorSchemeMirror, toMantineScheme } from '../shared/theme/color-scheme-manager'

export const appColorScheme = createColorSchemeMirror('dark')

// The settings query (including optimistic writes) drives Mantine's scheme, so every place that
// changes the scheme only has to update settings.
export function bindColorSchemeToSettings(queryClient: QueryClient): () => void {
  return queryClient.getQueryCache().subscribe(() => {
    const settings = queryClient.getQueryData<Settings>(queryKeys.settings)
    if (settings) appColorScheme.sync(toMantineScheme(settings.appearance.scheme))
  })
}
