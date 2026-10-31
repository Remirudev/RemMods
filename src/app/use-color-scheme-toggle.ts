import { useComputedColorScheme } from '@mantine/core'
import { useCallback } from 'react'
import { useUpdateSettings } from '../features/settings/queries'
import { appearanceUpdate } from '../features/settings/settings-update'
import type { ColorSchemeToggle } from './types'

export function useColorSchemeToggle(): ColorSchemeToggle {
  const computed = useComputedColorScheme('dark', { getInitialValueInEffect: false })
  const { mutate: updateSettings } = useUpdateSettings()
  const isDark = computed === 'dark'

  const toggle = useCallback(() => {
    updateSettings(appearanceUpdate({ scheme: isDark ? 'light' : 'dark' }))
  }, [isDark, updateSettings])

  return { isDark, toggle }
}
