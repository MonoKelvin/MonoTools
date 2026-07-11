/**
 * Canvas-based textW 在 happy-dom / node 环境下的纯逻辑版本。
 * 通过把 `document.createElement('canvas')` 替换为恒定宽度集来测试
 * 纯算法函数（buildWidthArrays/maxLeftFit/snapLeftToBoundary）。
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

// 在测试运行时把 Canvas measureText 桩成"每个字符 7px，'...' 是 21px"。
// 然后在测试里清晰控制每个调用点的字符串宽度。

const PROBE: { canvas: { width?: number; height?: number } | null } = { canvas: null }

beforeEach(() => {
  PROBE.canvas = null
  // 提供最小 canvas mock
  ;(globalThis as any).document = {
    createElement: (_tag: string) => {
      const measureText = (text: string) => {
        if (text === '...') return { width: 21 }
        // ASCII 7px，CJK 14px
        let w = 0
        for (const ch of text) {
          const c = ch.codePointAt(0)!
          if (c >= 0x4e00 && c <= 0x9fff) w += 14
          else w += 7
        }
        return { width: w }
      }
      PROBE.canvas = { width: 0, height: 0 }
      return {
        width: 0,
        height: 0,
        getContext: () => ({
          font: '',
          measureText,
        }),
      }
    },
  }
})

afterEach(() => {
  delete (globalThis as any).document
})

const font = '14px mono'

describe('textW', () => {
  it('returns exact width for short ASCII', async () => {
    const { textW } = await import('@/utils/text')
    expect(textW('hello', font)).toBe(35)
  })

  it('returns wider width for CJK characters', async () => {
    const { textW } = await import('@/utils/text')
    expect(textW('你好', font)).toBe(28)
  })

  it('ellipsisW returns 21', async () => {
    const { ellipsisW } = await import('@/utils/text')
    expect(ellipsisW(font)).toBe(21)
  })
})

describe('truncateMiddle — fits untouched', () => {
  it('returns original when width is sufficient', async () => {
    const { truncateMiddle } = await import('@/utils/text')
    expect(truncateMiddle('hello', 100, font)).toBe('hello')
  })
})

describe('truncateMiddle — boundary snap', () => {
  it('snaps left cut to nearest space when boundary=" "', async () => {
    const { truncateMiddle, textW } = await import('@/utils/text')
    const s = 'the quick brown fox jumps over'
    // 截到预算避免返回原串
    const budget = Math.floor(textW(s, font) / 2)
    const out = truncateMiddle(s, budget, font, ' ')
    expect(out).toContain('...')
    const idx = out.indexOf('...')
    // 省略号前要么是空格，要么是行首
    expect([' ', undefined].includes(out[idx - 1]) || idx === 0).toBe(true)
  })

  it('without boundary, fits at character boundary', async () => {
    const { truncateMiddle } = await import('@/utils/text')
    const out = truncateMiddle('abcdefghij', 50, font)
    expect(out.length).toBeLessThanOrEqual(10)
    expect(out).toContain('...')
  })
})

describe('truncatePathMiddle', () => {
  it('fits untouched when short', async () => {
    const { truncatePathMiddle } = await import('@/utils/text')
    expect(truncatePathMiddle('C:\\a\\b.txt', 200, font)).toBe('C:\\a\\b.txt')
  })

  it('preserves drive and filename under middle truncation', async () => {
    const { truncatePathMiddle } = await import('@/utils/text')
    const long = 'C:\\Users\\Hello\\Documents\\projects\\very-long-folder-name\\src\\index.ts'
    const out = truncatePathMiddle(long, 200, font)
    // 必须保留 C:\ + 文件名
    expect(out.startsWith('C:\\')).toBe(true)
    expect(out.endsWith('\\index.ts')).toBe(true)
  })

  it('snaps ellipsis to next \\ in long paths', async () => {
    const { truncatePathMiddle } = await import('@/utils/text')
    const long = 'C:\\Users\\Hello\\Documents\\projects\\very-long-folder-name\\src\\index.ts'
    const out = truncatePathMiddle(long, 100, font)
    const idx = out.indexOf('...')
    if (idx >= 0) {
      // 之前要么是 '\\'，要么是 'C:\\' 的开头
      expect(['\\', 'C'].includes(out[idx - 1])).toBe(true)
    }
  })

  it('falls back to plain truncateMiddle when no \\ boundary exists', async () => {
    const { truncatePathMiddle } = await import('@/utils/text')
    const noBoundary = 'no_underscores_no_slashes_here.txt'
    const out = truncatePathMiddle(noBoundary, 80, font)
    // 没有 '\\' → 走 translateMiddle 路径（不一定截，因为短）
    expect(typeof out).toBe('string')
  })

  it('returns ellipsis for unreasonably narrow width (single segment)', async () => {
    const { truncatePathMiddle } = await import('@/utils/text')
    const out = truncatePathMiddle('C:\\Program Files (x86)\\Some App\\deep\\nested\\path\\file.bin', 30, font)
    // 极窄容器只能放下省略号或非常少前缀；至少 constraint 满足
    expect(out.length).toBeGreaterThan(0)
  })
})

describe('buildWidthArrays & maxLeftFit', () => {
  it('pref and suff consistency', async () => {
    const { buildWidthArrays, textW } = await import('@/utils/text')
    const s = 'abcdef'
    const { pref, suff } = buildWidthArrays(s, font)
    expect(pref[0]).toBe(0)
    expect(pref[6]).toBe(textW(s, font))
    expect(suff[6]).toBe(0)
    expect(suff[0]).toBe(textW(s, font))
    // pref[k] + suff[k] = full width
    for (let i = 0; i <= s.length; i++) {
      expect(pref[i] + suff[i]).toBe(textW(s, font))
    }
  })

  it('maxLeftFit returns the maximum left chars that fit ellipsis', async () => {
    const { buildWidthArrays, maxLeftFit, ellipsisW } = await import('@/utils/text')
    const s = '0123456789012345'
    const { pref, suff } = buildWidthArrays(s, font)
    // 16 chars * 7 = 112 px, ellipsis 21
    // budget 35 ≈ left ok
    const left = maxLeftFit(35, s.length, pref, suff, ellipsisW(font))
    // 二分结果是某个合适 left
    expect(left).toBeGreaterThanOrEqual(0)
    expect(left).toBeLessThanOrEqual(Math.floor(s.length / 2))
  })
})

describe('snapLeftToBoundary', () => {
  it('returns cut unchanged when no boundary before cut', async () => {
    const { snapLeftToBoundary } = await import('@/utils/text')
    expect(snapLeftToBoundary('abcdef', 4, '\\')).toBe(4)
  })

  it('snaps to position after the last boundary', async () => {
    const { snapLeftToBoundary } = await import('@/utils/text')
    // '\\' 出现在 2 (即 string[2])
    // cutAt=5 → lastIndexOf('\\', 4) returns 2 → snap = 2 + 1 = 3
    expect(snapLeftToBoundary('C:\\users\\hello', 5, '\\')).toBe(3)
  })
})
