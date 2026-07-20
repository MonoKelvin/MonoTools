<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue'
import { insertSmartBreaks } from '../composables/useAdaptiveText'

interface Props {
  text: string
  maxLines?: number
  baseFontSize?: number
  minFontSize?: number
}

const props = withDefaults(defineProps<Props>(), {
  maxLines: 2,
  baseFontSize: 11,
  minFontSize: 9,
})

const titleRef = ref<HTMLElement | null>(null)
const fontSize = ref(props.baseFontSize)
const lineHeight = 1.3
const step = 0.5

const displayText = computed(() => insertSmartBreaks(props.text))

function measureAndAdjust() {
  const el = titleRef.value
  if (!el) return

  el.style.fontSize = `${props.baseFontSize}px`
  fontSize.value = props.baseFontSize

  const maxHeight = props.maxLines * props.baseFontSize * lineHeight
  const scrollHeight = el.scrollHeight

  if (scrollHeight <= maxHeight) {
    return
  }

  let currentSize = props.baseFontSize
  while (currentSize > props.minFontSize) {
    currentSize -= step
    el.style.fontSize = `${currentSize}px`
    const currentMaxHeight = props.maxLines * currentSize * lineHeight
    if (el.scrollHeight <= currentMaxHeight) {
      fontSize.value = currentSize
      return
    }
  }

  fontSize.value = props.minFontSize
  el.style.fontSize = `${props.minFontSize}px`
}

onMounted(() => {
  measureAndAdjust()
})

watch(
  () => props.text,
  () => {
    fontSize.value = props.baseFontSize
    measureAndAdjust()
  }
)
</script>

<template>
  <div
    ref="titleRef"
    class="gs-icon-mode-title"
    :style="{
      fontSize: `${fontSize}px`,
      WebkitLineClamp: maxLines,
    }"
  >
    {{ displayText }}
  </div>
</template>

<style scoped>
.gs-icon-mode-title {
  text-align: center;
  line-height: 1.3;
  display: -webkit-box;
  -webkit-box-orient: vertical;
  white-space: normal;
  color: var(--text-primary);
  font-weight: 500;
  overflow: hidden;
  word-break: break-word;
  overflow-wrap: anywhere;
  max-width: 86px;
  padding: 0 2px;
  margin-top: 8px;
  box-sizing: border-box;
}
</style>
