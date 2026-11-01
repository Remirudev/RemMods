import { describe, expect, it } from 'vitest'
import { formatSize } from './format-size'

const KIB = 1024
const MIB = KIB * 1024
const GIB = MIB * 1024
const TIB = GIB * 1024

describe('formatSize', () => {
  it('shows a real zero as 0 B', () => {
    expect(formatSize(0, 'en')).toBe('0 B')
  })

  it('matches the original for whole kilobytes', () => {
    expect(formatSize(2048, 'en')).toBe('2 KB')
  })

  it('uses the language decimal separator', () => {
    expect(formatSize(1536, 'en')).toBe('1.5 KB')
    expect(formatSize(1536, 'de')).toBe('1,5 KB')
  })

  it('switches units only at 1024', () => {
    expect(formatSize(1023, 'en')).toBe('1023 B')
    expect(formatSize(1048575, 'en')).toBe('1024 KB')
    expect(formatSize(MIB, 'en')).toBe('1 MB')
  })

  it('rounds to at most two decimals', () => {
    expect(formatSize(3.5 * GIB, 'en')).toBe('3.5 GB')
    expect(formatSize(1.005 * MIB + 1, 'en')).toBe('1.01 MB')
  })

  it('caps at GB and never groups digits', () => {
    expect(formatSize(TIB, 'en')).toBe('1024 GB')
    expect(formatSize(TIB, 'de')).toBe('1024 GB')
    expect(formatSize(10 * TIB, 'en')).toBe('10240 GB')
  })

  it('returns null for an unknown size', () => {
    expect(formatSize(null, 'en')).toBeNull()
  })
})
