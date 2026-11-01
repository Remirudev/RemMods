import { common as deCommon } from '../../locales/de/common'
import { errors as deErrors } from '../../locales/de/errors'
import { game as deGame } from '../../locales/de/game'
import { mods as deMods } from '../../locales/de/mods'
import { onboarding as deOnboarding } from '../../locales/de/onboarding'
import { profiles as deProfiles } from '../../locales/de/profiles'
import { settings as deSettings } from '../../locales/de/settings'
import { shell as deShell } from '../../locales/de/shell'
import { common as enCommon } from '../../locales/en/common'
import { errors as enErrors } from '../../locales/en/errors'
import { game as enGame } from '../../locales/en/game'
import { mods as enMods } from '../../locales/en/mods'
import { onboarding as enOnboarding } from '../../locales/en/onboarding'
import { profiles as enProfiles } from '../../locales/en/profiles'
import { settings as enSettings } from '../../locales/en/settings'
import { shell as enShell } from '../../locales/en/shell'

export const enResources = {
  common: enCommon,
  shell: enShell,
  settings: enSettings,
  errors: enErrors,
  mods: enMods,
  game: enGame,
  onboarding: enOnboarding,
  profiles: enProfiles,
}

export const deResources = {
  common: deCommon,
  shell: deShell,
  settings: deSettings,
  errors: deErrors,
  mods: deMods,
  game: deGame,
  onboarding: deOnboarding,
  profiles: deProfiles,
}

export const resources = {
  en: enResources,
  de: deResources,
}
