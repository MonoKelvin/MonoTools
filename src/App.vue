<script setup lang="ts">
import { onMounted } from 'vue'
import { listen } from '@tauri-apps/api/event'

onMounted(async () => {
  // 监听主题切换事件
  await listen('theme:changed', (event) => {
    const payload = event.payload as Record<string, unknown>
    if (payload.cssVariables) {
      const vars = payload.cssVariables as Record<string, string>
      const root = document.documentElement
      Object.entries(vars).forEach(([key, value]) => {
        root.style.setProperty(key, value)
      })
    }
  })
})
</script>

<template>
  <div id="app" class="mt-dark">
    <SearchBox />
  </div>
</template>

<style>
/* 全局样式重置 */
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html,
body {
  width: 100%;
  height: 100%;
  overflow: hidden;
  background-color: transparent;
  font-family: var(--mt-font-body, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#app {
  width: 100%;
  height: 100%;
  background: transparent;
}
</style>
