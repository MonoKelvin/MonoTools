import { describe, expect, it } from 'vitest'
import { formatSize, formatDate, formatTimeShort, truncate, extractFileName } from '@/utils/format'

describe('formatSize', () => {
  it('formats bytes without decimals', () => {
    expect(formatSize(500)).toBe('500 B')
  })

  it('formats KB with one decimal', () => {
    expect(formatSize(2048)).toBe('2.0 KB')
  })

  it('formats MB', () => {
    expect(formatSize(2 * 1024 * 1024)).toBe('2.0 MB')
  })

  it('formats GB', () => {
    expect(formatSize(2 * 1024 * 1024 * 1024)).toBe('2.0 GB')
  })
})

describe('formatDate', () => {
  it('returns empty string for falsy ts', () => {
    expect(formatDate(0)).toBe('')
  })

  it('formats a known ts to a non-empty string', () => {
    // 2024-01-01 UTC
    const out = formatDate(1_704_067_200)
    expect(typeof out).toBe('string')
    expect(out.length).toBeGreaterThan(0)
  })
})

describe('formatTimeShort', () => {
  it('returns empty string for 0', () => {
    expect(formatTimeShort(0)).toBe('')
  })

  it('formats a known ts to a non-empty string with HH:MM shape', () => {
    const out = formatTimeShort(1_704_067_200)
    expect(out).toMatch(/\d{2}:\d{2}/)
  })
})

describe('truncate', () => {
  it('returns input when shorter than max', () => {
    expect(truncate('hello', 10)).toBe('hello')
  })

  it('truncates with ellipsis when over limit', () => {
    const out = truncate('abcdefghij', 6)
    expect(out.length).toBeLessThan(7)
    expect(out.endsWith('…')).toBe(true)
  })
})

describe('extractFileName', () => {
  it('returns last segment after \\', () => {
    expect(extractFileName('C:\\Users\\foo\\bar.txt')).toBe('bar.txt')
  })

  it('returns last segment after /', () => {
    expect(extractFileName('/home/user/file.ts')).toBe('file.ts')
  })

  it('falls back to whole path when no separator', () => {
    expect(extractFileName('plain')).toBe('plain')
  })
})
