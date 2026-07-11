/**
 * 文本宽度测量：复用 Canvas 字符宽度做中段省略计算，以像素而非字符数判断截断位置。
 *
 * - `_canvas` 是模块单例，避免每个组件各自创建导致开销
 * - 每个字符 / 字体组合的值有缓存
 * - `truncateMiddle` 与 `truncatePathMiddle` 用于 ResultItem；测试 [tests/ui/utils/text.test.ts](tests/ui/utils/text.test.ts)
 */

let _canvas: HTMLCanvasElement | null = null
let _ctx: CanvasRenderingContext2D | null = null
const _cwCache = new Map<string, number>()

function ensureCtx(): CanvasRenderingContext2D {
  if (!_ctx) {
    if (typeof document === 'undefined') {
      throw new Error('text-measurement requires browser environment (document)')
    }
    _canvas = document.createElement('canvas')
    _ctx = _canvas.getContext('2d')!
  }
  return _ctx
}

function getCW(ch: string, font: string): number {
  const key = ch + '\x00' + font
  const hit = _cwCache.get(key)
  if (hit !== undefined) return hit
  const ctx = ensureCtx()
  if (ctx.font !== font) ctx.font = font
  const w = ctx.measureText(ch).width
  _cwCache.set(key, w)
  return w
}

export function textW(text: string, font: string): number {
  let w = 0
  for (let i = 0; i < text.length; i++) w += getCW(text[i], font)
  return w
}

/** 省略号宽度按字体懒计算 */
export function ellipsisW(font: string): number {
  const key = '...\x00' + font
  const hit = _cwCache.get(key)
  if (hit !== undefined) return hit
  const w = textW('...', font)
  _cwCache.set(key, w)
  return w
}

export const ELLIPSIS = '...'

/** 前缀/后缀宽度数组：在 shrinkLeft 中 O(1) 拿到任意 prefix / suffix 的总宽度 */
export function buildWidthArrays(text: string, font: string): { pref: number[]; suff: number[] } {
  const pref = new Array(text.length + 1)
  const suff = new Array(text.length + 1)
  pref[0] = 0
  for (let i = 0; i < text.length; i++) pref[i + 1] = pref[i] + getCW(text[i], font)
  suff[text.length] = 0
  for (let i = text.length - 1; i >= 0; i--) suff[i] = suff[i + 1] + getCW(text[i], font)
  return { pref, suff }
}

/** 二分查找：在 pref+suff 数组里找出最大"左侧字符数 left"，使 pref[left] + ... + suff[text.length-left] ≤ maxW */
export function maxLeftFit(
  maxW: number,
  textLen: number,
  pref: number[],
  suff: number[],
  eW: number,
): number {
  if (maxW <= 0) return 0
  let lo = 0
  let hi = Math.floor(textLen / 2)
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1
    if (pref[mid] + eW + suff[textLen - mid] <= maxW) lo = mid
    else hi = mid - 1
  }
  return lo
}

/** 把 cutAt 之前最近的 boundary 字符（+ 自身长度）作为新的左截点 */
export function snapLeftToBoundary(text: string, cutAt: number, boundary: string): number {
  if (cutAt <= 0 || cutAt >= text.length) return cutAt
  const idx = text.lastIndexOf(boundary, cutAt - 1)
  return idx >= 0 ? idx + boundary.length : cutAt
}

/** 中段省略：text 左右各保留 left 个字符，中间用 ... */
export function truncateMiddle(
  text: string,
  maxWidth: number,
  font: string,
  boundary?: string,
): string {
  const eW = ellipsisW(font)
  if (maxWidth <= 0) return ELLIPSIS
  if (textW(text, font) <= maxWidth) return text
  if (maxWidth <= eW) return ELLIPSIS

  const { pref, suff } = buildWidthArrays(text, font)
  const n = text.length
  const left = maxLeftFit(maxWidth, n, pref, suff, eW)
  if (left <= 0) return ELLIPSIS

  const finalLeft = boundary ? snapLeftToBoundary(text, left, boundary) : left
  if (finalLeft <= 0) return ELLIPSIS

  return text.substring(0, finalLeft) + ELLIPSIS + text.substring(n - left)
}

/** Windows 路径中段省略：保留盘符 + 文件名，中间按 '\\' 边界截断 */
export function truncatePathMiddle(path: string, maxWidth: number, font: string): string {
  const eW = ellipsisW(font)
  if (maxWidth <= 0) return ELLIPSIS
  if (textW(path, font) <= maxWidth) return path
  if (maxWidth <= eW) return ELLIPSIS

  const parts = path.split('\\')
  if (parts.length <= 2) return truncateMiddle(path, maxWidth, font)

  const drive = parts[0] + '\\'
  const filename = parts[parts.length - 1]
  const driveW = textW(drive, font)
  const fileW = textW(filename, font)

  // 极窄路径：保留 drive + ellipsis + 文件名（文件本身可能再中段截断）
  if (driveW + eW + fileW >= maxWidth) {
    const fileAvail = maxWidth - driveW - eW
    if (fileAvail <= eW) return truncateMiddle(path, maxWidth, font)
    return drive + ELLIPSIS + truncateMiddle(filename, fileAvail, font)
  }

  // 中间目录能放下：不截
  const middleParts = parts.slice(1, parts.length - 1)
  const middleStr = middleParts.join('\\')
  const middleAvail = maxWidth - driveW - eW - fileW
  if (textW(middleStr, font) <= middleAvail) {
    return drive + middleStr + '\\' + filename
  }

  // 中间需要截：snap 到 '\\' 边界，避免出现 Use...\src 之类
  return drive + truncateMiddle(middleStr, middleAvail, font, '\\') + '\\' + filename
}
