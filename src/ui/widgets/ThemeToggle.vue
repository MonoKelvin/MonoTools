<script setup lang="ts">
import { useThemeStore } from '@/core/stores/theme'

const themeStore = useThemeStore()
const label = '暗色'
void themeStore
</script>

<template>
  <div
    class="theme-toggle"
    v-tooltip="{ value: '当前主题：' + label, showDelay: 280, position: 'bottom' }"
    aria-label="主题"
  >
    <svg
      width="13"
      height="13"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
    </svg>
    <span class="theme-toggle-label">{{ label }}</span>
  </div>
</template>

<style scoped>
.theme-toggle {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-2);
  padding: var(--sp-1) var(--sp-3);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  background: transparent;
  border-radius: var(--radius-sm);
  cursor: default;
  font-weight: 500;
}
</style>