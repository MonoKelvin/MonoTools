<script setup lang="ts">
import { ref, computed } from 'vue'
import { Search, X } from "@lucide/vue"

const props = withDefaults(
  defineProps<{
    modelValue: string
    placeholder?: string
  }>(),
  { placeholder: '搜索应用、文件、命令...' },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
  (e: 'enter'): void
  (e: 'arrowUp'): void
  (e: 'arrowDown'): void
  (e: 'escape'): void
}>()

const inputRef = ref<HTMLInputElement | null>(null)
const localValue = ref(props.modelValue)
const focused = ref(false)

watch(() => props.modelValue, (v) => (localValue.value = v))
watch(localValue, (v) => emit('update:modelValue', v))

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter') { e.preventDefault(); emit('enter') }
  else if (e.key === 'ArrowDown') { e.preventDefault(); emit('arrowDown') }
  else if (e.key === 'ArrowUp') { e.preventDefault(); emit('arrowUp') }
  else if (e.key === 'Escape') { e.preventDefault(); emit('escape') }
}

onMounted(() => {
  inputRef.value?.focus()
})

// ========== Drag logic ==========

function isOverText(input: HTMLInputElement, clientX: number): boolean {
  const rect = input.getBoundingClientRect()
  const frontBoundary = rect.left + rect.width * 0.2
  if (clientX <= frontBoundary) return true

  if (!input.value || input.value.length === 0) return false

  const style = getComputedStyle(input)
  const canvas = document.createElement('canvas')
  const ctx = canvas.getContext('2d')!
  ctx.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`

  const paddingLeft = parseFloat(style.paddingLeft) || 0
  const charCount = Math.min(4, input.value.length)
  const prefixWidth = ctx.measureText(input.value.slice(0, charCount)).width
  const suffixWidth = ctx.measureText(input.value.slice(-charCount)).width
  const textWidth = ctx.measureText(input.value).width

  const textAreaStart = rect.left + paddingLeft - prefixWidth
  const textAreaEnd = rect.left + paddingLeft + textWidth + suffixWidth

  return clientX >= textAreaStart && clientX <= textAreaEnd
}

async function handleWrapperMousedown(event: MouseEvent) {
  const input = inputRef.value
  if (!input) return

  const overText = isOverText(input, event.clientX)

  if (!overText) {
    event.preventDefault()
    try {
      await invoke('start_dragging')
    } catch (error) {
      console.error('Failed to start dragging:', error)
    }
  }
}

function handleWrapperMousemove(event: MouseEvent) {
  const input = inputRef.value
  if (!input) return

  const overText = isOverText(input, event.clientX)
  const cursor = overText ? 'text' : 'default'

  const wrapper = event.currentTarget as HTMLElement
  wrapper.style.cursor = cursor
  input.style.cursor = cursor
}

function handleWrapperMouseleave() {
  const input = inputRef.value
  if (input) {
    input.style.cursor = ''
  }
}

function focus() {
  inputRef.value?.focus()
}

function select() {
  inputRef.value?.select()
}

const TEXT_FADE_KEY = 'monotools-text-fade-enabled'
const textFadeEnabled = ref(localStorage.getItem(TEXT_FADE_KEY) !== 'false')
const overflowRight = ref(false)
const overflowLeft = ref(false)

function updateOverflow() {
  const input = inputRef.value
  if (!input) return

  overflowRight.value = input.scrollWidth > input.clientWidth
  overflowLeft.value = input.scrollLeft > 1
}

const fadeClass = computed(() => {
  if (!textFadeEnabled.value) return ''
  const right = overflowRight.value
  const left = overflowLeft.value
  if (right && left) return 'mt-search-input-wrapper--fade-both'
  if (right) return 'mt-search-input-wrapper--fade-right'
  if (left) return 'mt-search-input-wrapper--fade-left'
  return ''
})

watch(() => props.modelValue, async () => {
  await nextTick()
  updateOverflow()
})

defineExpose({ focus, select, fadeClass })
</script>

<template>
  <div
    :class="['mt-search-input-wrapper', fadeClass]"
    @mousedown="handleWrapperMousedown"
    @mousemove="handleWrapperMousemove"
    @mouseleave="handleWrapperMouseleave"
  >
    <input
      ref="inputRef"
      type="text"
      :value="modelValue"
      :placeholder="placeholder"
      class="mt-search-input"
      @input="(e) => emit('update:modelValue', (e.target as HTMLInputElement).value)"
      @keydown="onKeydown"
      @focus="focused = true"
      @blur="focused = false"
      @change="updateOverflow"
    />
  </div>
</template>

<style scoped>
.mt-search-input-wrapper {
  flex: 1;
  min-width: 0;
  -webkit-app-region: no-drag;
  overflow: hidden;
  position: relative;
}

/* 右侧渐变遮罩 */
.mt-search-input-wrapper--fade-right::after {
  content: '';
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  width: 40px;
  background: linear-gradient(to right, transparent, var(--canvas) 80%);
  pointer-events: none;
  z-index: 1;
}

/* 左侧渐变遮罩 */
.mt-search-input-wrapper--fade-left::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  bottom: 0;
  width: 40px;
  background: linear-gradient(to left, transparent, var(--canvas) 80%);
  pointer-events: none;
  z-index: 1;
}

/* 两侧渐变遮罩 */
.mt-search-input-wrapper--fade-both::before,
.mt-search-input-wrapper--fade-both::after {
  content: '';
  position: absolute;
  top: 0;
  bottom: 0;
  width: 40px;
  pointer-events: none;
  z-index: 1;
}

.mt-search-input-wrapper--fade-both::before {
  left: 0;
  background: linear-gradient(to left, transparent, var(--canvas) 80%);
}

.mt-search-input-wrapper--fade-both::after {
  right: 0;
  background: linear-gradient(to right, transparent, var(--canvas) 80%);
}

/* 输入框 */
.mt-search-input {
  width: 100%;
  height: 100%;
  padding: 0;
  font-family: var(--font-sans);
  font-size: var(--text-xl);
  font-weight: 400;
  line-height: 1.4;
  color: var(--text-primary);
  background: transparent;
  border: none;
  outline: none;
  caret-color: var(--accent);
  transition: all 0.12s var(--ease-out, ease);
}

.mt-search-input::placeholder {
  color: var(--text-tertiary);
  transition: all 0.12s var(--ease-out, ease);
}

.mt-search-input:focus::placeholder {
  opacity: 0.5;
}
</style>
