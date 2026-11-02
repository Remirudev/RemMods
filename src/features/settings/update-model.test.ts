import { describe, expect, it } from 'vitest'
import type { AppInstallKind, AppUpdateChannel } from '../../lib/ipc/types'
import type { UpdateOffer } from './types'
import { downloadPercent, formatPublished, kindNotice, updateOffer } from './update-model'

function channel(install: AppInstallKind, configured: boolean): AppUpdateChannel {
  return {
    install,
    selfUpdate: install === 'appImage' || install === 'nsis',
    configured,
    current: '0.1.0',
  }
}

const CONFIGURED: [AppInstallKind, UpdateOffer][] = [
  ['appImage', { kind: 'install', installer: false }],
  ['nsis', { kind: 'install', installer: true }],
  ['deb', { kind: 'notice', notice: 'package', format: '.deb' }],
  ['rpm', { kind: 'notice', notice: 'package', format: '.rpm' }],
  ['aur', { kind: 'notice', notice: 'aur', format: null }],
  ['unknown', { kind: 'notice', notice: 'unknown', format: null }],
]

describe('updateOffer', () => {
  it.each(CONFIGURED)('offers the right thing for a configured %s install', (install, offer) => {
    expect(updateOffer(channel(install, true))).toEqual(offer)
  })

  it.each(CONFIGURED.map(([install]) => install))(
    'says "not configured" for every %s install without an update channel',
    (install) => {
      expect(updateOffer(channel(install, false))).toEqual({ kind: 'notConfigured' })
    },
  )

  it('never offers the button when the backend does not allow a self-update', () => {
    const appImage = { ...channel('appImage', true), selfUpdate: false }
    expect(updateOffer(appImage)).toEqual({ kind: 'notice', notice: 'unknown', format: null })
    const deb = { ...channel('deb', true), selfUpdate: true }
    expect(updateOffer(deb).kind).toBe('notice')
  })
})

describe('kindNotice', () => {
  it.each([true, false])('names how packages update, configured %s', (isConfigured) => {
    expect(kindNotice(channel('deb', isConfigured))).toEqual({
      kind: 'notice',
      notice: 'package',
      format: '.deb',
    })
    expect(kindNotice(channel('rpm', isConfigured))?.format).toBe('.rpm')
    expect(kindNotice(channel('aur', isConfigured))?.notice).toBe('aur')
    expect(kindNotice(channel('appImage', isConfigured))).toBeNull()
    expect(kindNotice(channel('nsis', isConfigured))).toBeNull()
  })

  it('leaves an unknown copy without update channel to the "not configured" notice', () => {
    expect(kindNotice(channel('unknown', false))).toBeNull()
    expect(kindNotice(channel('unknown', true))?.notice).toBe('unknown')
  })
})

describe('downloadPercent', () => {
  it('gives whole percents of the announced size', () => {
    expect(downloadPercent({ kind: 'downloading', downloaded: 0, total: 200 })).toBe(0)
    expect(downloadPercent({ kind: 'downloading', downloaded: 101, total: 200 })).toBe(50)
    expect(downloadPercent({ kind: 'downloading', downloaded: 200, total: 200 })).toBe(100)
  })

  it('has no percent without a size', () => {
    expect(downloadPercent({ kind: 'downloading', downloaded: 5, total: null })).toBeNull()
    expect(downloadPercent({ kind: 'downloading', downloaded: 5, total: 0 })).toBeNull()
  })

  it('clamps to 100 when more bytes arrive than announced', () => {
    expect(downloadPercent({ kind: 'downloading', downloaded: 900, total: 300 })).toBe(100)
  })

  it('has no percent once the download is over', () => {
    expect(downloadPercent({ kind: 'installing' })).toBeNull()
    expect(downloadPercent({ kind: 'restarting' })).toBeNull()
  })
})

describe('formatPublished', () => {
  it('formats a valid date and hides an invalid or missing one', () => {
    expect(formatPublished('2026-11-02T10:00:00Z', 'en')).toContain('2026')
    expect(formatPublished('not a date', 'en')).toBeNull()
    expect(formatPublished(null, 'en')).toBeNull()
  })
})
