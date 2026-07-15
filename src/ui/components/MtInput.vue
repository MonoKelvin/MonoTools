<script setup lang="ts">
import { ref, watch, computed, type Component } from 'vue'

interface Props {
  modelValue?: string
  placeholder?: string
  type?: 'text' | 'password' | 'search'
  icon?: Component
  clearable?: boolean
  autofocus?: boolean
  disabled?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: '',
  placeholder: '',
  type: 'text',
  clearable: false,
  autofocus: false,
  disabled: false,
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'focus', event: FocusEvent): void
  (e: 'blur', event: FocusEvent): void
  (e: 'enter', event: KeyboardEvent): void
}>()

const inputRef = ref<HTMLInputElement | null>(null)
const isFocused = ref(false)

const hasValue = computed(() => props.modelValue && props.modelValue.length > 0)

const handleInput = (event: Event) => {
  const target = event.target as HTMLInputElement
  emit('update:modelValue', target.value)
}

const handleClear = () => {
  emit('update:modelValue', '')
  inputRef.value?.focus()
}

const handleKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Enter') {
    emit('enter', event)
  }
}

watch(() => props.autofocus, (val) => {
  if (val) {
    inputRef.value?.focus()
  }
})
</script>

<template>
  <div :class="['mt-input', { 'mt-input--focused': isFocused, 'mt-input--disabled': disabled }]">
    <component v-if="icon" :is="icon" class="mt-input__icon" />
    <input
      ref="inputRef"
      :type="type"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="disabled"
      class="mt-input__field"
      @input="handleInput"
      @focus="isFocused = true; emit('focus', $event)"
      @blur="isFocused = false; emit('blur', $event)"
      @keydown="handleKeydown"
    />
    <button
      v-if="clearable && hasValue"
      class="mt-input__clear"
      @click="handleClear"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="18" y1="6" x2="6" y2="18"></line>
        <line x1="6" y1="6" x2="18" y2="18"></line>
      </svg>
    </button>
    <div class="mt-input__underline"></div>
  </div>
</template>

<style scoped>
.mt-input {
  position: relative;
  display: flex;
  align-items: center;
  background: var(--surface);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-lg);
  padding: var(--sp-3) var(--sp-4);
  transition: all var(--dur-fast) var(--ease-out);
}

.mt-input:hover {
  border-color: var(--border-hover);
}

.mt-input--focused {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-glow);
}

.mt-input--disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.mt-input__icon {
  width: 18px;
  height: 18px;
  color: var(--text-tertiary);
  margin-right: var(--sp-3);
  flex-shrink: 0;
}

.mt-input__field {
  flex: 1;
  width: 100%;
  background: transparent;
  border: none;
  outline: none;
  color: var(--text-primary);
  font-family: var(--font-sans);
  font-size: var(--text-base);
}

.mt-input__field::placeholder {
  color: var(--text-quaternary);
}

.mt-input__field:disabled {
  cursor: not-allowed;
}

.mt-input__clear {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: var(--radius-full);
  color: var(--text-tertiary);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
  margin-left: var(--sp-2);
}

.mt-input__clear:hover {
  background: var(--surface-hover);
  color: var(--text-secondary);
}

.mt-input__underline {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--accent);
  border-radius: 1px;
  transform: scaleX(0);
  transition: transform var(--dur-normal) var(--ease-out);
}

.mt-input--focused .mt-input__underline {
  transform: scaleX(1);
}
</style>
