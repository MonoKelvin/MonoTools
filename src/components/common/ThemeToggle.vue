<script setup lang="ts">
import { useThemeStore } from '@/stores/theme'

const themeStore = useThemeStore()
// Theme lock is dark-only; show a static indicator
const label = '暗色'
void themeStore
</script>

<template>
  <div class="theme-toggle" :title="'当前主题：' + label" aria-label="主题">
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
  gap: 4px;
  padding: 3px 9px;
  font-size: 12px;
  color: var(--text-body);
  background: transparent;
  border-radius: var(--radius-md);
  cursor: default;
  font-weight: 500;
}
</style>
