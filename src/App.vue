<template>
  <div class="app-root" data-tauri-drag-region>
    <router-view />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useThemeStore } from '@/core/stores/theme'

const themeStore = useThemeStore()

/**
 * 全局屏蔽 webview 右键菜单.
 * 软件任何地方都不允许出现浏览器原生右键菜单.
 * 所有右键交互由 SearchPage / SearchInput / ContextMenu 自行处理.
 */
if (typeof window !== 'undefined') {
  window.addEventListener('contextmenu', (e) => e.preventDefault())
}

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
  color: var(--text-primary);
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
