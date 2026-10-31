import type { enResources } from './resources'

export type Language = 'en' | 'de'

export interface ErrorDescription {
  title: string
  message: string
}

declare module 'i18next' {
  interface CustomTypeOptions {
    defaultNS: 'common'
    resources: typeof enResources
    enableSelector: true
  }
}
