import type { common as en } from '../en/common'
import type { LocaleShape } from '../types'

export const common: LocaleShape<typeof en> = {
  appName: 'RemMods',
  actions: {
    tryAgain: 'Nochmal versuchen',
    reload: 'App neu laden',
  },
  errorBoundary: {
    title: 'In {{area}} ist etwas kaputtgegangen.',
    titleGeneric: 'Etwas ist kaputtgegangen.',
    hint: 'Der Rest von RemMods läuft weiter. Versuch es nochmal oder lade die App neu, wenn es bleibt.',
  },
}
