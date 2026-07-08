<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { Search } from "@lucide/vue"
import { useSearchStore } from '@/stores/search'
import { useSettingsStore } from '@/stores/settings'
import { hotkeyApi, windowApi } from '@/services'
import { isTauri } from '@/services/env'
import { useRouter } from 'vue-router'

import SearchInput from '@/components/common/SearchInput.vue'
import CategoryTabs from '@/components/search/CategoryTabs.vue'
import SearchResults from '@/components/search/SearchResults.vue'
import ActionBar from '@/components/search/ActionBar.vue'
import SettingsPanel from '@/components/panels/SettingsPanel.vue'
import CommandsPanel from '@/components/panels/CommandsPanel.vue'

const search = useSearchStore()
const settings = useSettingsStore()
const router = useRouter()

const inputRef = ref<InstanceType<typeof SearchInput> | null>(null)
const containerRef = ref<HTMLElement | null>(null)
let resizeObserver: ResizeObserver | null = null
let pendingHeight = 0
let resyncTimer: number | null = null

const WINDOW_FIXED_WIDTH = 680

const fixWindowWidth = async () => {
  if (!isTauri) return
  try {
    const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow')
    const win = WebviewWindow.getCurrent()
    const currentSize = await win.innerSize()
    if (Math.abs(currentSize.width - WINDOW_FIXED_WIDTH) > 1) {
      await win.setSize({ width: WINDOW_FIXED_WIDTH, height: currentSize.height })
    }
  } catch { /* fallback */ }
}

const syncWindowHeight = () => {
  if (!isTauri) return
  if (!containerRef.value) return
  const rect = containerRef.value.getBoundingClientRect()
  const h = Math.round(rect.height)
  if (Math.abs(h - pendingHeight) < 2) return
  pendingHeight = h
  if (resyncTimer) window.clearTimeout(resyncTimer)
  resyncTimer = window.setTimeout(() => windowApi.setHeight(h), 50)
}

const onEnter = async () => { await search.executeSelected() }
const onUp = () => search.selectPrev()
const onDown = () => search.selectNext()
const onEscape = () => {
  search.hide()
}

// 不要在blur时隐藏窗口，用户点击其他地方不应该自动关闭
// const handleBlur = () => {
//   // 窗口失去焦点时隐藏
//   search.hide()
// }

const onSelect = (item: any) => search.executeItem(item)
const onHover = (idx: number) => { search.selectedIndex = idx }
const onQueryChange = (val: string) => search.setQuery(val)
const onCategorySelect = (cat: any) => search.setCategory(cat)

const onLogoContextMenu = (event: MouseEvent) => {
  // Logo 右键菜单事件处理
  console.log('Logo context menu triggered')
}

const tryRegisterHotkey = async () => {
  if (!isTauri) return
  try { await hotkeyApi.register(settings.settings.hotkey) }
  catch { /* ignore */ }
}

onMounted(async () => {
  await settings.load()
  await tryRegisterHotkey()
  await fixWindowWidth()
  await nextTick()
  if (containerRef.value && typeof ResizeObserver !== 'undefined') {
    resizeObserver = new ResizeObserver(syncWindowHeight)
    resizeObserver.observe(containerRef.value)
    syncWindowHeight()
  }
})

onBeforeUnmount(() => {
  if (resizeObserver) { resizeObserver.disconnect(); resizeObserver = null }
})

// Watch router changes to sync window height
watch(() => router.currentRoute.value.path, () => nextTick(syncWindowHeight))
</script>

<template>
  <div class="app-viewport" data-tauri-drag-region>
    <!-- 搜索界面 - 只有一个统一的面板 -->
    <div class="search-container" ref="containerRef">
      <SearchInput
        ref="inputRef"
        :model-value="search.query"
        @update:model-value="onQueryChange"
        @enter="onEnter"
        @arrow-down="onDown"
        @arrow-up="onUp"
        @escape="onEscape"
        @contextmenu="onLogoContextMenu"
        autofocus
      />
      <CategoryTabs
        :active="search.activeCategory"
        @select="onCategorySelect"
      />
      <SearchResults
        :results="search.filteredResults"
        :loading="search.loading"
        :selected-index="search.selectedIndex"
        @select="onSelect"
        @hover="onHover"
      >
        <template #empty>
          <div class="empty-state">
            <Search class="empty-icon" :size="28" :stroke-width="1.5" />
            <span v-if="!search.query" class="empty-text">
              输入关键字搜索应用、文件、命令
            </span>
            <span v-else class="empty-text">没有找到结果</span>
          </div>
        </template>
      </SearchResults>
      <ActionBar />
    </div>
  </div>
</template>

<style scoped>
.app-viewport {
  width: 100%;
  height: 100%;
  position: relative;
  overflow: hidden;
  display: flex;
  justify-content: center;
  align-items: center;
  background: var(--canvas);
}

.search-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--surface);
  border: none;
  box-shadow:
    0 4px 6px rgba(0, 0, 0, 0.1),
    0 10px 30px rgba(0, 0, 0, 0.2),
    0 0 0 1px rgba(255, 255, 255, 0.05);
  overflow: hidden;
  transition: height var(--dur-normal) var(--ease-out);
}

/* ========== Empty state ========== */

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-4);
  padding: var(--sp-12) var(--sp-6);
  flex: 1;
}
.empty-icon {
  color: var(--text-quaternary);
  opacity: 0.5;
  transition: opacity var(--dur-fast) var(--ease-out);
}
.empty-text {
  color: var(--text-tertiary);
  font-size: var(--text-md);
  letter-spacing: 0.01em;
  font-weight: 400;
}
</style>