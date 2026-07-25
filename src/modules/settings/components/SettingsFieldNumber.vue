<script setup lang="ts">
import { ref, watch } from 'vue'

const props = withDefaults(defineProps<{
  modelValue: number
  min?: number
  max?: number
  step?: number
  disabled?: boolean
}>(), {
  step: 1,
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: number): void
}>()

const inputRef = ref<HTMLInputElement | null>(null)
const localValue = ref(String(props.modelValue ?? ''))

watch(() => props.modelValue, (v) => {
  localValue.value = String(v ?? '')
})

function commit() {
  const n = Number(localValue.value)
  if (isNaN(n)) {
    localValue.value = String(props.modelValue ?? '')
    return
  }
  let clamped = n
  if (props.min !== undefined) clamped = Math.max(props.min, clamped)
  if (props.max !== undefined) clamped = Math.min(props.max, clamped)
  emit('update:modelValue', clamped)
  localValue.value = String(clamped)
}

function onKeyDown(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    commit()
    inputRef.value?.blur()
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    const n = Number(localValue.value) || 0
    localValue.value = String(n + (props.step ?? 1))
    commit()
  } else if (e.key === 'ArrowDown') {
    e.preventDefault()
    const n = Number(localValue.value) || 0
    localValue.value = String(n - (props.step ?? 1))
    commit()
  }
}
</script>

<template>
  <input
    ref="inputRef"
    :value="localValue"
    :disabled="disabled"
    class="settings-number-input"
    type="text"
    inputmode="numeric"
    @input="(e) => localValue = (e.target as HTMLInputElement).value"
    @blur="commit"
    @keydown="onKeyDown"
  />
</template>

<style scoped>
.settings-number-input {
  width: 80px;
  padding: var(--sp-2) var(--sp-3);
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--text-primary);
  background: var(--surface-raised);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  font-family: var(--font-mono);
  text-align: right;
  outline: none;
  transition:
    border-color var(--dur-fast) var(--ease-out),
    box-shadow var(--dur-fast) var(--ease-out);
}

.settings-number-input::placeholder {
  color: var(--text-quaternary);
}

.settings-number-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-glow);
}

.settings-number-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.settings-number-input::-webkit-inner-spin-button,
.settings-number-input::-webkit-outer-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
</style>
