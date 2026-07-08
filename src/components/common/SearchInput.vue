<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { Search } from "@lucide/vue"

const props = withDefaults(
  defineProps<{
    modelValue: string
    placeholder?: string
    autofocus?: boolean
  }>(),
  { placeholder: '搜索应用、文件、命令...', autofocus: false },
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
  setTimeout(() => inputRef.value?.focus(), 50)
}

const focusInput = () => inputRef.value?.focus()
defineExpose({ focus: focusInput })
</script>

<template>
  <div class="search-input-wrapper" :class="{ 'is-focused': focused }">
    <Search class="search-icon" :size="18" :stroke-width="2" />
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
    <Transition name="fade">
      <kbd v-if="localValue" class="esc-key" @click="localValue = ''">ESC</kbd>
    </Transition>
  </div>
</template>

<style scoped>
.search-input-wrapper {
  display: flex;
  align-items: center;
  padding: 14px 20px;
  gap: 12px;
  border-bottom: 1px solid var(--border);
  background: rgba(255, 255, 255, 0.02);
  transition: background var(--duration-fast) var(--ease-out);
}
:global(.theme-light) .search-input-wrapper {
  background: rgba(0, 0, 0, 0.015);
}
.search-input-wrapper.is-focused {
  background: rgba(255, 255, 255, 0.04);
}
:global(.theme-light) .search-input-wrapper.is-focused {
  background: rgba(0, 0, 0, 0.03);
}

.search-icon {
  flex-shrink: 0;
  width: 18px;
  height: 18px;
  color: var(--text-tertiary);
  transition: color var(--duration-fast) var(--ease-out);
}
.search-input-wrapper.is-focused .search-icon {
  color: var(--accent);
}

.search-input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: var(--text-primary);
  font-size: 16px;
  font-family: var(--font-family);
  letter-spacing: 0.01em;
}
.search-input::placeholder {
  color: var(--text-tertiary);
}

.esc-key {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 2px 8px;
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--text-tertiary);
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
  line-height: 1.4;
}
:global(.theme-light) .esc-key {
  background: rgba(0, 0, 0, 0.04);
}
.esc-key:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--text-primary);
}
:global(.theme-light) .esc-key:hover {
  background: rgba(0, 0, 0, 0.07);
}
</style>
