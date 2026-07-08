<template>
  <div :class="['app-root', themeClass]" data-tauri-drag-region>
    <router-view />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useThemeStore } from '@/stores/theme'

const themeStore = useThemeStore()

// Raycast 是 dark-only；保留 css class 但实际总是 dark
const themeClass = computed(() => 'theme-dark')

const _ = themeStore

onMounted(async () => {
  await themeStore.applyTheme()
})
</script>

<style scoped>
.app-root {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--canvas);
  color: var(--text-ink);
  font-family: var(--font-sans);
  -webkit-user-select: none;
  user-select: none;
  overflow: hidden;
}

.app-root :deep(input),
.app-root :deep(textarea) {
  -webkit-user-select: text;
  user-select: text;
}
</style>
