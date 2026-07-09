<script setup lang="ts">
import type { Component } from 'vue'

interface Props {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger'
  size?: 'sm' | 'md' | 'lg'
  disabled?: boolean
  loading?: boolean
  icon?: Component
  iconRight?: Component
}

withDefaults(defineProps<Props>(), {
  variant: 'secondary',
  size: 'md',
  disabled: false,
  loading: false,
})

const emit = defineEmits<{
  (e: 'click', event: MouseEvent): void
}>()
</script>

<template>
  <button
    :class="[
      'mt-btn',
      `mt-btn--${variant}`,
      `mt-btn--${size}`,
      { 'mt-btn--disabled': disabled || loading, 'mt-btn--loading': loading },
    ]"
    :disabled="disabled || loading"
    @click="emit('click', $event)"
  >
    <span v-if="loading" class="mt-btn__loader"></span>
    <component v-else-if="icon" :is="icon" class="mt-btn__icon" />
    <span class="mt-btn__text">
      <slot />
    </span>
    <component v-if="iconRight" :is="iconRight" class="mt-btn__icon mt-btn__icon--right" />
  </button>
</template>

<style scoped>
.mt-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--sp-2);
  padding: 0;
  border: none;
  border-radius: var(--radius-md);
  font-family: var(--font-sans);
  font-weight: 500;
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
  position: relative;
  overflow: hidden;
}

.mt-btn--sm {
  height: 28px;
  padding: 0 var(--sp-4);
  font-size: var(--text-sm);
}

.mt-btn--md {
  height: 34px;
  padding: 0 var(--sp-5);
  font-size: var(--text-md);
}

.mt-btn--lg {
  height: 40px;
  padding: 0 var(--sp-6);
  font-size: var(--text-base);
}

.mt-btn__icon {
  flex-shrink: 0;
}

.mt-btn--sm .mt-btn__icon {
  width: 14px;
  height: 14px;
}

.mt-btn--md .mt-btn__icon {
  width: 16px;
  height: 16px;
}

.mt-btn--lg .mt-btn__icon {
  width: 18px;
  height: 18px;
}

.mt-btn__icon--right {
  order: 2;
}

.mt-btn__text {
  order: 1;
}

.mt-btn--primary {
  background: var(--accent);
  color: #000;
}

.mt-btn--primary:hover:not(:disabled) {
  background: var(--accent-hover);
  transform: translateY(-0.5px);
  box-shadow: 0 4px 12px var(--accent-glow);
}

.mt-btn--primary:active:not(:disabled) {
  background: var(--accent-active);
  transform: translateY(0);
}

.mt-btn--secondary {
  background: var(--surface-overlay);
  color: var(--text-primary);
  border: 1px solid var(--border-default);
}

.mt-btn--secondary:hover:not(:disabled) {
  background: var(--surface-hover);
  border-color: var(--border-hover);
}

.mt-btn--secondary:active:not(:disabled) {
  background: var(--surface-active);
}

.mt-btn--ghost {
  background: transparent;
  color: var(--text-secondary);
}

.mt-btn--ghost:hover:not(:disabled) {
  background: var(--surface-hover);
  color: var(--text-primary);
}

.mt-btn--ghost:active:not(:disabled) {
  background: var(--surface-active);
}

.mt-btn--danger {
  background: transparent;
  color: var(--color-danger);
  border: 1px solid transparent;
}

.mt-btn--danger:hover:not(:disabled) {
  background: var(--color-danger-bg);
  border-color: var(--color-danger);
}

.mt-btn--disabled,
.mt-btn--loading {
  opacity: 0.5;
  cursor: not-allowed;
}

.mt-btn__loader {
  width: 16px;
  height: 16px;
  border: 2px solid currentColor;
  border-right-color: transparent;
  border-radius: 50%;
  animation: mt-btn-loader-spin 0.6s linear infinite;
}

@keyframes mt-btn-loader-spin {
  to { transform: rotate(360deg); }
}
</style>
