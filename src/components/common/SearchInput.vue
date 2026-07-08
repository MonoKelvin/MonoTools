<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { Search, X } from "@lucide/vue"

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
  <!-- data-tauri-drag-region: background can drag; <input> is auto-excluded by Tauri -->
  <div
    class="search-input-wrapper"
    :class="{ 'is-focused': focused }"
    data-tauri-drag-region
  >
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
      <button
        v-if="localValue"
        class="search-clear"
        type="button"
        @click="localValue = ''"
        aria-label="清空"
      >
        <X :size="13" :stroke-width="2" />
      </button>
    </Transition>
  </div>
</template>

<style scoped>
.search-input-wrapper {
  display: flex;
  align-items: center;
  padding: 12px 14px;
  gap: 10px;
  background: transparent;
  transition: background var(--duration-fast) var(--ease-out);
  flex-shrink: 0;
}

.search-icon {
  flex-shrink: 0;
  width: 16px;
  height: 16px;
  color: var(--text-mute);
  transition: color var(--duration-fast) var(--ease-out);
}

.search-input-wrapper.is-focused .search-icon {
  color: var(--text-ink);
}

.search-input {
  flex: 1;
  min-width: 0;
  background: transparent;
  border: none;
  outline: none;
  color: var(--text-ink);
  font-size: 16px;
  font-weight: 500;
  font-family: var(--font-sans);
  letter-spacing: 0.005em;
  caret-color: var(--on-dark);
}

.search-input::placeholder {
  color: var(--text-ash);
  font-weight: 400;
}

.search-clear {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  padding: 0;
  border: none;
  background: var(--surface-card);
  color: var(--text-mute);
  border-radius: var(--radius-xs);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}
.search-clear:hover {
  background: var(--surface-elevated);
  color: var(--text-ink);
}
</style>
