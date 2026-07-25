<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  modelValue: string
  placeholder?: string
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const inputRef = ref<HTMLInputElement | null>(null)
const internalValue = ref(props.modelValue)

function commit() {
  emit('update:modelValue', internalValue.value)
}

function onFocus() {
  if (inputRef.value) {
    inputRef.value.select()
  }
}
</script>

<template>
  <input
    ref="inputRef"
    v-model="internalValue"
    :placeholder="placeholder"
    :disabled="disabled"
    class="settings-text-input"
    @blur="commit"
    @keydown.enter="commit"
    type="text"
  />
</template>

<style scoped>
.settings-text-input {
  width: 200px;
  padding: var(--sp-2) var(--sp-3);
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--text-primary);
  background: var(--surface-elevated, rgba(255, 255, 255, 0.04));
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  font-family: var(--font-sans);
  outline: none;
  transition:
    border-color var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out);

  &::placeholder {
    color: var(--text-quaternary);
  }

  &:focus {
    border-color: var(--accent);
    background: transparent;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}
</style>
