<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useTauri } from '@tauri-apps/api/tauri'
import { useHotkey } from './composables/useHotkey'
import { useWindow } from './composables/useWindow'
import SearchBox from './components/search/SearchBox.vue'

const { listen } = useTauri()
const { showWindow, hideWindow } = useWindow()
const { registerHotkey, unregisterHotkey } = useHotkey()

onMounted(async () => {
  // 注册全局快捷键 Alt+Space
  await registerHotkey('Alt+Space', (event) => {
    event.preventDefault()
    showWindow()
  })

  // 监听窗口失焦事件
  await listen('tauri://focus', (event) => {
    if (!event.windowState?.focused) {
      hideWindow()
    }
  })

  // 监听主题切换事件
  await listen('theme:changed', (event) => {
    const { cssVariables } = event.payload as { cssVariables: Record<string, string> }
    applyThemeVariables(cssVariables)
  })
})

onUnmounted(() => {
  unregisterHotkey('Alt+Space')
})

function applyThemeVariables(variables: Record<string, string>) {
  const root = document.documentElement
  Object.entries(variables).forEach(([key, value]) => {
    root.style.setProperty(key, value)
  })
}
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
