import type { AppInstallKind, AppUpdateChannel, AppUpdateProgress } from '../../lib/ipc/types'
import type { PackageFormat, UpdateNotice, UpdateOffer } from './types'

const PACKAGE_FORMATS: Partial<Record<AppInstallKind, PackageFormat>> = {
  deb: '.deb',
  rpm: '.rpm',
}

function noticeFor(install: AppInstallKind): UpdateNotice {
  const format = PACKAGE_FORMATS[install]
  if (format) return { kind: 'notice', notice: 'package', format }
  if (install === 'aur') return { kind: 'notice', notice: 'aur', format: null }
  return { kind: 'notice', notice: 'unknown', format: null }
}

// What the section offers for an available version. Only AppImage and the Windows installer
// replace themselves (the backend allowlist decides; `selfUpdate` mirrors it); every other
// install gets its notice instead of the install button.
export function updateOffer(channel: AppUpdateChannel): UpdateOffer {
  if (!channel.configured) return { kind: 'notConfigured' }
  const { install, selfUpdate } = channel
  if (selfUpdate && (install === 'appImage' || install === 'nsis')) {
    return { kind: 'install', installer: install === 'nsis' }
  }
  return noticeFor(install)
}

// How a package install updates, shown whether or not a check ran. A copy of unknown origin
// without an update channel gets only the "not configured" notice.
export function kindNotice(channel: AppUpdateChannel): UpdateNotice | null {
  const offer = updateOffer({ ...channel, configured: true })
  if (offer.kind !== 'notice') return null
  if (offer.notice === 'unknown' && !channel.configured) return null
  return offer
}

// Null when the server announced no size, so the view shows the downloaded bytes instead.
export function downloadPercent(progress: AppUpdateProgress): number | null {
  if (progress.kind !== 'downloading') return null
  const { downloaded, total } = progress
  if (total === null || total <= 0) return null
  return Math.min(100, Math.max(0, Math.floor((downloaded / total) * 100)))
}

// An invalid or missing `pub_date` hides the date line.
export function formatPublished(date: string | null, locale: string): string | null {
  if (date === null) return null
  const parsed = new Date(date)
  if (Number.isNaN(parsed.getTime())) return null
  return new Intl.DateTimeFormat(locale, { dateStyle: 'long' }).format(parsed)
}
