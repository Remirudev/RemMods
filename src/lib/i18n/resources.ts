import { common as deCommon } from '../../locales/de/common'
import { errors as deErrors } from '../../locales/de/errors'
import { settings as deSettings } from '../../locales/de/settings'
import { shell as deShell } from '../../locales/de/shell'
import { common as enCommon } from '../../locales/en/common'
import { errors as enErrors } from '../../locales/en/errors'
import { settings as enSettings } from '../../locales/en/settings'
import { shell as enShell } from '../../locales/en/shell'

export const enResources = {
  common: enCommon,
  shell: enShell,
  settings: enSettings,
  errors: enErrors,
}

export const deResources = {
  common: deCommon,
  shell: deShell,
  settings: deSettings,
  errors: deErrors,
}

export const resources = {
  en: enResources,
  de: deResources,
}
