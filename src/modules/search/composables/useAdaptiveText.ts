import { ref, onMounted, watch, type Ref } from 'vue'

export interface UseAdaptiveTextOptions {
  maxLines?: number
  baseFontSize?: number
  minFontSize?: number
  lineHeight?: number
  step?: number
}

export function useAdaptiveText(
  textRef: Ref<HTMLElement | null>,
  text: Ref<string> | string,
  options: UseAdaptiveTextOptions = {}
) {
  const {
    maxLines = 2,
    baseFontSize = 11,
    minFontSize = 9,
    lineHeight = 1.3,
    step = 0.5,
  } = options

  const fontSize = ref(baseFontSize)
  const isTruncated = ref(false)

  function measureAndAdjust() {
    const el = textRef.value
    if (!el) return

    el.style.fontSize = `${baseFontSize}px`
    fontSize.value = baseFontSize
    isTruncated.value = false

    const lineHeightPx = baseFontSize * lineHeight
    const maxHeight = maxLines * lineHeightPx

    const scrollHeight = el.scrollHeight

    if (scrollHeight <= maxHeight) {
      return
    }

    let currentSize = baseFontSize
    while (currentSize > minFontSize) {
      currentSize -= step
      el.style.fontSize = `${currentSize}px`
      const currentLineHeight = currentSize * lineHeight
      const currentMaxHeight = maxLines * currentLineHeight
      if (el.scrollHeight <= currentMaxHeight) {
        fontSize.value = currentSize
        return
      }
    }

    fontSize.value = minFontSize
    el.style.fontSize = `${minFontSize}px`
    const minLineHeight = minFontSize * lineHeight
    if (el.scrollHeight > maxLines * minLineHeight) {
      isTruncated.value = true
    }
  }

  onMounted(() => {
    measureAndAdjust()
  })

  if (typeof text !== 'string') {
    watch(text, () => {
      fontSize.value = baseFontSize
      measureAndAdjust()
    })
  }

  return {
    fontSize,
    isTruncated,
    measureAndAdjust,
  }
}

export function insertSmartBreaks(text: string): string {
  if (!text) return text

  let result = text

  result = result.replace(/([a-z])([A-Z])/g, '$1\u200B$2')

  result = result.replace(/([A-Z]+)([A-Z][a-z])/g, '$1\u200B$2')

  result = result.replace(/([a-zA-Z])(\d)/g, '$1\u200B$2')
  result = result.replace(/(\d)([a-zA-Z])/g, '$1\u200B$2')

  result = result.replace(/([-_\.\/\\])/g, '$1\u200B')

  return result
}
