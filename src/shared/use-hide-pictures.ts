import { useQuery } from '@tanstack/react-query'
import { ipc } from '../lib/ipc'
import { queryKeys } from './query-keys'

// Shares the settings query, so the switch in Settings reaches every picture at once.
export function useHidePictures(): boolean {
  const settings = useQuery({ queryKey: queryKeys.settings, queryFn: () => ipc.settingsGet() })
  return settings.data?.appearance.hidePictures ?? false
}
