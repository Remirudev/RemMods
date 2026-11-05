import i18next from 'i18next'
import { initReactI18next } from 'react-i18next'
import { resources } from './resources'
import type { Language } from './types'

export function detectLanguage(preferred: string | undefined): Language {
  return preferred?.toLowerCase().startsWith('de') ? 'de' : 'en'
}

function applyDocumentLanguage(language: string): void {
  if (typeof document !== 'undefined') document.documentElement.lang = language
}

i18next.on('languageChanged', applyDocumentLanguage)

void i18next.use(initReactI18next).init({
  resources,
  lng: detectLanguage(typeof navigator === 'undefined' ? undefined : navigator.language),
  fallbackLng: 'en',
  supportedLngs: ['en', 'de'],
  ns: [
    'common',
    'shell',
    'settings',
    'errors',
    'mods',
    'game',
    'onboarding',
    'profiles',
    'install',
    'skins',
  ],
  defaultNS: 'common',
  interpolation: { escapeValue: false },
  initAsync: false,
  enableSelector: true,
})

export { i18next as i18n }
