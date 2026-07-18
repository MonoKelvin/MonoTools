/**
 * 自适应文字显示工具函数.
 *
 * 算法流程 (nowrap / 单行):
 * 1. 使用 ResizeObserver 监听容器宽度变化
 * 2. 使用 canvas.measureText() 测量文字实际像素宽度
 * 3. 从 maxFontSize 开始, 逐步缩小直到文字完全容纳
 * 4. 如果缩小到 minFontSize 仍然超出, 使用 CSS text-overflow: ellipsis
 * 5. hover 时通过原生 title 属性显示完整文字
 *
 * 算法流程 (whiteSpace='normal' / 多行图标模式):
 * 1. 监听容器高度变化 (ResizeObserver 同时观察高度)
 * 2. 先缩放到让文字尽量在容器宽度内不换行 (保持单行优先, 用户要求"尽量单行显示")
 * 3. 如果单行缩减到 minFontSize 仍超出, 则换行, maxLines 限制行数
 * 4. 用 canvas 助手模拟换行, 找到能放下的行数和字体大小
 * 5. 超出 maxLines 的行末尾使用省略号
 *
 * 设计原则:
 * - 单行模式: 优先缩小字体 → 仍超出则省略号
 * - 多行模式 (图标模式): 优先不换行缩小字体 → 仍无法放不下则换行 → 超出 maxLines 截断
 * - hover 显示 tooltip (用户可查看完整内容)
 * - 使用 requestAnimationFrame 避免频繁重排
 */

import { ref, onMounted, onBeforeUnmount, watch } from 'vue'

export interface AdaptiveTextOptions {
  /** 初始字体大小 (px). 默认 14 */
  maxFontSize?: number
  /** 最小字体大小 (px). 默认 10 */
  minFontSize?: number
  /** 字体粗细. 默认 'normal' */
  fontWeight?: string
  /** 字体族. 默认从元素继承 */
  fontFamily?: string
  /** 预留边距 (px), 防止文字紧贴边缘. 默认 8 */
  padding?: number
  /** 省略号字符串. 默认 '...' */
  ellipsis?: string
  /** 是否启用 ResizeObserver. 默认 true */
  observeResize?: boolean
  /** 换行模式: 'nowrap' 单行(默认), 'normal' 可换行(用于图标模式). 默认 'nowrap' */
  whiteSpace?: 'nowrap' | 'normal'
  /** 最大行数 (仅 whiteSpace='normal' 时生效). 默认 2 (图标模式最多2行). 传 undefined 表示不限 */
  maxLines?: number
}

/**
 * 用数组 size 表示每行宽度占比, 模拟文字在 maxFontSize 下的占宽.
 * 用于分析多行文本的宽度分布 (不创建真实 DOM).
 */
function estimateCharWidths(text: string, fontSize: number, fontFamily: string): number[] {
  const canvas = document.createElement('canvas')
  const ctx = canvas.getContext('2d')
  if (!ctx) return text.split('').map(() => fontSize * 0.5)
  ctx.font = `400 ${fontSize}px ${fontFamily}`
  return text.split('').map((ch) => {
    if (ch === ' ') return fontSize * 0.28
    if (ch === '\t') return fontSize * 0.7
    return ctx.measureText(ch).width
  })
}

/**
 * 将 characterWidths 按可用宽度分行, 返回每行文本.
 */
function wrapCharWidths(
  text: string,
  charWidths: number[],
  availWidth: number,
): string[] {
  if (availWidth <= 0) return [text]
  const lines: string[] = []
  let line = ''
  let lineW = 0
  for (let i = 0; i < text.length; i++) {
    const w = charWidths[i] ?? 0
    if (w > availWidth) {
      // 单个字符都放不下, 硬塞
      if (line) lines.push(line)
      lines.push(text[i])
      line = ''
      lineW = 0
      continue
    }
    if (lineW + w > availWidth && line) {
      lines.push(line)
      line = ''
      lineW = 0
    }
    line += text[i]
    lineW += w
  }
  if (line) lines.push(line)
  return lines.length > 0 ? lines : ['']
}

export function useAdaptiveText(
  text: string | (() => string),
  options: AdaptiveTextOptions = {},
) {
  const {
  maxFontSize = 14,
  minFontSize = 10,
  fontWeight = 'normal',
  fontFamily,
  padding = 8,
  ellipsis = '...',
  observeResize = true,
  whiteSpace = 'nowrap',
  maxLines,
} = options

  // 默认 maxLines: nowrap=undefined, normal=2 (符合用户要求: 图标模式最多2行)
  const effectiveMaxLines: number | undefined =
    whiteSpace === 'normal'
      ? isFinite(maxLines as number)
        ? (maxLines as number)
        : 2
      : undefined

  const containerRef = ref<HTMLElement | null>(null)
  const displayText = ref('')
  const displayLines = ref<string[]>([])
  const currentFontSize = ref(maxFontSize)
  const isTruncated = ref(false)
  const whiteSpaceMode = ref(whiteSpace)
  let resizeObserver: ResizeObserver | null = null
  let rafId: number | null = null

  function measureTextWidth(text: string, fontSize: number): number {
    const canvas = document.createElement('canvas')
    const ctx = canvas.getContext('2d')
    if (!ctx) return 0
    const weight = fontWeight
    const family = fontFamily || getComputedStyle(document.body).fontFamily
    ctx.font = `${weight} ${fontSize}px ${family}`
    return ctx.measureText(text).width
  }

  /**
   * 单行模式 (nowrap): 二分查找最佳字体大小使文本整体不超出容器宽度.
   * 返回 { fontSize, text, truncated }.
   */
  function computeFitSingleLine(
  containerWidth: number,
  textStr: string,
): { fontSize: number; text: string; truncated: boolean } {
  const availWidth = containerWidth - padding * 2
  if (availWidth <= 0) {
    return { fontSize: minFontSize, text: ellipsis, truncated: true }
  }

  const textWidthAtMax = measureTextWidth(textStr, maxFontSize)

  // 最大字体就能放下
  if (textWidthAtMax <= availWidth) {
    return { fontSize: maxFontSize, text: textStr, truncated: false }
  }

  // 二分查找能让文本完全放下的最大字体大小
  let lo = minFontSize
  let hi = maxFontSize
  let bestSize = minFontSize

  for (let i = 0; i < 30; i++) {
  const mid = (lo + hi) / 2
  const w = measureTextWidth(textStr, mid)
  if (w <= availWidth) {
    bestSize = mid
    lo = mid
  } else {
    hi = mid
  }
  if (hi - lo < 0.3) break
}

  // 用 bestSize 检查是否足以放下全文
  const bestWidth = measureTextWidth(textStr, bestSize)
  if (bestWidth <= availWidth) {
    return { fontSize: bestSize, text: textStr, truncated: false }
  }

  // 缩小到 minFontSize 仍放不下, 用省略号
  const minW = measureTextWidth(textStr, minFontSize)
  if (minW > availWidth) {
    const ellipsisW = measureTextWidth(ellipsis, minFontSize)
    const availChars = Math.floor(
    (availWidth - ellipsisW) / (minW / textStr.length),
  )
  const charCount = Math.max(0, Math.min(availChars, textStr.length - 1))
  return {
    fontSize: minFontSize,
    text: textStr.substring(0, charCount) + ellipsis,
    truncated: true,
  }
}

  return { fontSize: bestSize, text: textStr, truncated: false }
}

/**
 * 多行模式 (normal): 先尝试单行缩放到 maxFontSize→minFontSize，若 maxFontSize 单行能放下则保持单行；
 * 若单行缩减到 minFontSize 仍放不下, 则换行, 受 maxLines 限制。
 *
 * 策略: 先按单行目标确定 fontSize (能放下全文且 >=minFontSize), 再决定是否换行。
 * 如果 fontSize === minFontSize 且仍放不下, 则:
 *   用 minFontSize 做换行，按 maxLines 截断, 最后一行加省略号。
 */
function computeFitMultiLine(
  containerWidth: number,
  containerHeight: number,
  textStr: string,
): {
  fontSize: number
  lines: string[]
  maxLines: number
  truncated: boolean
} {
  const availWidth = containerWidth - padding * 2
  if (availWidth <= 0) {
    return { fontSize: minFontSize, lines: [ellipsis], maxLines: effectiveMaxLines ?? 2, truncated: true }
  }

  const effectiveMaxL = effectiveMaxLines ?? 2
  const family = fontFamily || getComputedStyle(document.body).fontFamily

  // 1) 先看 maxFontSize 下单行是否能放下全文
  const fullWidthAtMax = measureTextWidth(textStr, maxFontSize)
  if (fullWidthAtMax <= availWidth) {
    // 全文单行能放下, 保持最大字体 (用户优先: "尽量缩放字体让软件名称显示完全, 不遮挡")
    return { fontSize: maxFontSize, lines: [textStr], maxLines: 1, truncated: false }
  }

  // 2) 二分查找单行能放下全文的最优 fontSize (取最小能放下全文的字体)
  //    这样优先保证"不换行"，仅在 fontSize 降到 minFontSize 仍有富余时才提前换行。
  let lo = minFontSize
  let hi = maxFontSize
  let bestSingleLineSize = minFontSize

  for (let i = 0; i < 30; i++) {
    const mid = (lo + hi) / 2
    if (measureTextWidth(textStr, mid) <= availWidth) {
      bestSingleLineSize = mid
      lo = mid
    } else {
      hi = mid
    }
    if (hi - lo < 0.3) break
  }

  // bestSingleLineSize 是单行能放下全文的最小字体(>= minFontSize)
  // 检查这个大小是否还明显 > minFontSize, 若是则尽量用单行；否则换行
  const bestFullTextWidth = measureTextWidth(textStr, bestSingleLineSize)
  // 若填充率 > 90%, 单行勉强, 考虑换行为2行 (换行后字体更大更可读)
  const fillRatio = bestFullTextWidth / availWidth

  if (fillRatio < 0.92 && bestSingleLineSize > minFontSize + 1) {
    // 单行可接受, 不换行
    return {
      fontSize: bestSingleLineSize,
      lines: [textStr],
      maxLines: 1,
      truncated: false,
    }
  }

  // 3) 无法单行容纳 => 用 minFontSize 换行，受 effectiveMaxLines 限制
  const charWidths = estimateCharWidths(textStr, minFontSize, family)
  // 估算每行字符数上限（基于平均 char 宽度，提高精度）
  const avgCharW = charWidths.reduce((a, b) => a + b, 0) / charWidths.length
  const approxCharsPerLine = Math.max(1, Math.floor(availWidth / avgCharW))
  const wrapped = wrapCharWidths(textStr, charWidths, availWidth)

  // 截取前 maxLines 行, 多余的截断到省略号
  let finalLines: string[]
  let truncated = false
  if (wrapped.length > effectiveMaxL) {
    const lastLine = wrapped[effectiveMaxL - 1]
    const lastLW = charWidths
      .filter((_, i) => textStr.indexOf(wrapped[effectiveMaxL - 1], i) >= 0)
      .reduce((a, b) => a + b, 0)
    // 简单处理: 截断最后一行并在末尾加省略号
    const ellipsisW = measureTextWidth(ellipsis, minFontSize)
    const maxLastLineChars = Math.floor(
      (availWidth - ellipsisW) / (avgCharW || 1),
    )
    const trimmed = lastLine.substring(0, Math.max(0, maxLastLineChars))
    finalLines = [...wrapped.slice(0, effectiveMaxL - 1), trimmed + ellipsis]
    truncated = true
  } else {
    finalLines = wrapped
    truncated = false
  }

  // 若容器高度不足以显示 effectiveMaxL 行，缩小字体
  const lineHeightPx = minFontSize * 1.35 * 1.05 // line-height 1.35 + margin
  const maxAllowedHeight = containerHeight - padding * 2
  const requiredHeight = finalLines.length * lineHeightPx

  let finalFontSize = minFontSize
  if (requiredHeight > maxAllowedHeight && maxAllowedHeight > 0) {
    // 尝试缩小字号
    const shrinkRatio = maxAllowedHeight / requiredHeight
    finalFontSize = Math.max(
      minFontSize,
      Math.floor(minFontSize * shrinkRatio),
    )
  }

  return {
    fontSize: finalFontSize,
    lines: finalLines,
    maxLines: effectiveMaxL,
    truncated,
  }
}

function update() {
  const el = containerRef.value
  if (!el) return

  const textStr = typeof text === 'function' ? text() : text
  if (!textStr) {
    displayText.value = ''
    displayLines.value = []
    currentFontSize.value = maxFontSize
    isTruncated.value = false
    return
  }

  const mode = whiteSpaceMode.value
  const containerWidth = el.getBoundingClientRect().width

  if (mode === 'normal') {
    const containerHeight = el.getBoundingClientRect().height
    const result = computeFitMultiLine(containerWidth, containerHeight, textStr)
    displayLines.value = result.lines
    currentFontSize.value = result.fontSize
    // truncated 仅当文本确实被截断
    isTruncated.value =
      result.truncated || (result.lines.length >= result.maxLines && textStr !== result.lines.join('\n'))
  } else {
    const result = computeFitSingleLine(containerWidth, textStr)
    displayText.value = result.text
    currentFontSize.value = result.fontSize
    displayLines.value = [result.text]
    isTruncated.value = result.truncated
  }
}

function scheduleUpdate() {
  if (rafId !== null) return
  rafId = requestAnimationFrame(() => {
    rafId = null
    update()
  })
}

onMounted(() => {
  update()
  if (observeResize && containerRef.value) {
    resizeObserver = new ResizeObserver(() => {
      scheduleUpdate()
    })
    resizeObserver.observe(containerRef.value)
  }
})

onBeforeUnmount(() => {
  if (resizeObserver) {
    resizeObserver.disconnect()
  }
  if (rafId !== null) {
    cancelAnimationFrame(rafId)
  }
})

if (typeof text !== 'function') {
  watch(() => text, () => scheduleUpdate())
}

// 暴露 displayLines (多行模式用) 和 displayText (单行模式兼容用)
return {
  containerRef,
  displayText,
  displayLines,
  currentFontSize,
  isTruncated,
  whiteSpaceMode,
  update,
}
}
