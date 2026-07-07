<script setup lang="ts">
import { ref, watch, computed } from 'vue'

const props = withDefaults(
  defineProps<{
    modelValue: string
    placeholder?: string
    autofocus?: boolean
  }>(),
  { placeholder: 'Search apps, files, commands...', autofocus: false },
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

watch(() => props.modelValue, (v) => (localValue.value = v))
watch(localValue, (v) => emit('update:modelValue', v))

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter') {
    e.preventDefault()
    emit('enter')
  } else if (e.key === 'ArrowDown') {
    e.preventDefault()
    emit('arrowDown')
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    emit('arrowUp')
  } else if (e.key === 'Escape') {
    e.preventDefault()
    emit('escape')
  }
}

if (props.autofocus) {
  setTimeout(() => inputRef.value?.focus(), 0)
}

const focused = ref(false)
const focusInput = () => inputRef.value?.focus()
defineExpose({ focus: focusInput })
</script>

<template>
  <div class="search-input-wrapper">
    <svg
      class="search-icon"
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <circle cx="11" cy="11" r="8" />
      <path d="m21 21-4.3-4.3" />
    </svg>
    <input
      ref="inputRef"
      v-model="localValue"
      type="text"
      class="search-input"
      :placeholder="placeholder"
      spellcheck="false"
      autocomplete="off"
      autocorrect="off"
      @keydown="onKeydown"
      @focus="focused = true"
      @blur="focused = false"
    />
    <span class="kbd">ESC</span>
  </div>
</template>

<style scoped>
.search-input-wrapper {
  display: flex;
  align-items: center;
  padding: 14px 18px;
  gap: 12px;
  border-bottom: 1px solid var(--border);
  background: rgba(255, 255, 255, 0.02);
}
:global(.theme-light) .search-input-wrapper {
  background: rgba(0, 0, 0, 0.02);
}
.search-icon {
  flex-shrink: 0;
  width: 18px;
  height: 18px;
  color: var(--text-secondary);
}
.search-input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: var(--text-primary);
  font-size: 16px;
  font-family: var(--font-family);
}
.search-input::placeholder {
  color: var(--text-tertiary);
}
</style>
