<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount, nextTick, onUpdated } from 'vue'
import { Search } from '@lucide/vue'
import { useSearchStore } from '@/stores/search'
import { useSettingsStore } from '@/stores/settings'
import { hotkeyApi, windowApi, shellApi } from '@/services'
import { isTauri } from '@/services/env'
import { useRouter } from 'vue-router'
import { listenEvent } from '@/services/tauri'
import type { SearchResult } from '@/types/search'

import SearchInput from '@/components/common/SearchInput.vue'
import CategoryTabs from '@/components/search/CategoryTabs.vue'
import SearchResults from '@/components/search/SearchResults.vue'
import ActionBar from '@/components/search/ActionBar.vue'
import ContextMenu from '@/components/search/ContextMenu.vue'
import HotkeyModal from '@/components/common/HotkeyModal.vue'
import { useCommandsStore, dispatchKeyEvent } from '@/commands'

const showContextMenu = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const contextMenuItem = ref<SearchResult | null>(null)
const showHotkeyModal = ref(false)

const search = useSearchStore()
const settings = useSettingsStore()
const router = useRouter()
const commandsStore = useCommandsStore()

const inputRef = ref<InstanceType<typeof SearchInput> | null>(null)
const containerRef = ref<HTMLElement | null>(null)
let resizeObserver: ResizeObserver | null = null
let pendingHeight = 0
let resyncTimer: ReturnType<typeof setTimeout> | null = null
let unlistenIndexProgress: (() => void) | null = null

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
  } catch {}
}

const syncWindowHeight = () => {
  if (!isTauri) return
  if (!containerRef.value) return
  const rect = containerRef.value.getBoundingClientRect()
  const h = Math.round(rect.height)
  if (Math.abs(h - pendingHeight) < 2) return
  pendingHeight = h
  if (resyncTimer) clearTimeout(resyncTimer)
  resyncTimer = setTimeout(() => windowApi.setHeight(h), 50)
}

const onSelect = (item: SearchResult) => {
  const idx = search.filteredResults.findIndex((r) => r.id === item.id)
  if (idx >= 0) search.selectedIndex = idx
  search.executeItem(item)
}
const onShowHotkeys = () => {
  showHotkeyModal.value = true
}
const onHover = (idx: number) => {
  search.selectedIndex = idx
}
const onQueryChange = (val: string) => search.setQuery(val)
const onCategorySelect = (cat: any) => search.setCategory(cat)

const tryRegisterHotkey = async () => {
  if (!isTauri) return
  try {
    await hotkeyApi.register(settings.settings.hotkey)
  } catch {}
}

const handleIndexProgress = (progress: { status: string; message?: string; files?: number }) => {
  search.setIndexProgress(progress)
}

const handleContextMenu = (e: MouseEvent, item?: SearchResult) => {
  e.preventDefault()
  if (item) {
    contextMenuX.value = e.clientX
    contextMenuY.value = e.clientY
    contextMenuItem.value = item
    showContextMenu.value = true
  } else if (
    search.filteredResults.length > 0 &&
    search.selectedIndex >= 0 &&
    search.selectedIndex < search.filteredResults.length
  ) {
    contextMenuX.value = e.clientX
    contextMenuY.value = e.clientY
    contextMenuItem.value = search.filteredResults[search.selectedIndex]
    showContextMenu.value = true
  }
}

const closeContextMenu = () => {
  showContextMenu.value = false
}

/** UI 内部对 search.cmd.* 命令的实现：仅 react state；无 IPC 调用。 */
function runUiCommand(id: string) {
  switch (id) {
    case 'search.cmd.execute-selected':
      return search.executeSelected()
    case 'search.cmd.next-item':
      return search.selectNext()
    case 'search.cmd.prev-item':
      return search.selectPrev()
    case 'search.cmd.close-window':
      return search.hide()
    case 'search.cmd.toggle-window':
      return search.toggle()
    case 'search.cmd.focus-input':
      return inputRef.value?.focus()
    case 'search.cmd.clear-input':
      return search.setQuery('')
    case 'search.cmd.copy-selected-path':
      return copySelected()
    case 'search.cmd.reveal-selected':
      return revealSelected()
    default:
      return undefined
  }
}

const copySelected = async () => {
  const item = search.filteredResults[search.selectedIndex]
  if (!item) return
  try {
    await navigator.clipboard.writeText(item.subtitle || item.title)
  } catch {}
}

const revealSelected = async () => {
  const item = search.filteredResults[search.selectedIndex]
  if (!item) return
  const path = item.subtitle || item.title
  const last = path.lastIndexOf('\\')
  if (last < 0) return
  const dir = path.substring(0, last)
  try {
    await shellApi.open(dir)
  } catch {}
}

/**
 * 集中处理 keydown：UI-only (search.cmd.*) 命令优先 → 走本地实现；
 * 后端命令由 `commandsStore.execute(id)` 委托 IPC。
 */
async function handleGlobalKeydown(event: KeyboardEvent) {
  const id = dispatchKeyEvent(event, commandsStore.list() as any)
  if (!id) return
  if (id.startsWith('search.cmd.')) {
    event.preventDefault()
    await runUiCommand(id)
    return
  }
  if (['app.cmd.navigate.settings', 'app.cmd.navigate.commands'].includes(id)) {
    event.preventDefault()
    const go = id === 'app.cmd.navigate.settings' ? '/settings' : '/commands'
    await router.push(go).catch(() => {})
    return
  }
  // 其他命令：发往后端
  event.preventDefault()
  await commandsStore.execute(id).catch(() => undefined)
}

onMounted(async () => {
  await commandsStore.loadFromBackend().catch(() => undefined)
  window.addEventListener('keydown', handleGlobalKeydown)
  await settings.load()
  await tryRegisterHotkey()
  await fixWindowWidth()
  await search.loadIndexStatus()

  unlistenIndexProgress = await listenEvent('index_progress', handleIndexProgress)

  if (containerRef.value) {
    containerRef.value.addEventListener('contextmenu', handleContextMenu)
  }

  await nextTick()
  if (containerRef.value && typeof ResizeObserver !== 'undefined') {
    resizeObserver = new ResizeObserver(syncWindowHeight)
    resizeObserver.observe(containerRef.value)
    syncWindowHeight()
  }
})

onUpdated(() => {
  nextTick(syncWindowHeight)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleGlobalKeydown)
  if (resizeObserver) {
    resizeObserver.disconnect()
    resizeObserver = null
  }
  if (unlistenIndexProgress) unlistenIndexProgress()
  if (containerRef.value) {
    containerRef.value.removeEventListener('contextmenu', handleContextMenu)
  }
})

watch(() => router.currentRoute.value.path, () => nextTick(syncWindowHeight))
</script>

<template>
  <div class="search-page">
    <div class="search-container" ref="containerRef">
      <SearchInput
        ref="inputRef"
        :model-value="search.query"
        @update:model-value="onQueryChange"
        @enter="runUiCommand('search.cmd.execute-selected')"
        @arrow-down="runUiCommand('search.cmd.next-item')"
        @arrow-up="runUiCommand('search.cmd.prev-item')"
        @escape="runUiCommand('search.cmd.close-window')"
        autofocus
      />

      <CategoryTabs :active="search.activeCategory" @select="onCategorySelect" />

      <SearchResults
        :results="search.filteredResults"
        :loading="search.loading"
        :selected-index="search.selectedIndex"
        @select="onSelect"
        @hover="onHover"
        @contextmenu="handleContextMenu"
      >
        <template #empty>
          <div class="empty-state">
            <div class="empty-state__icon">
              <Search :size="32" :stroke-width="1.5" />
            </div>
            <span v-if="!search.query" class="empty-state__text">
              输入关键字搜索应用、文件、命令
            </span>
            <template v-else>
              <span class="empty-state__text">没有找到结果</span>
              <span class="empty-state__hint">尝试使用不同的关键词</span>
            </template>
          </div>
        </template>
      </SearchResults>

      <ActionBar
        :results="search.filteredResults"
        :selected-index="search.selectedIndex"
        :index-building="search.indexStatus === 'building'"
        :index-status="search.indexStatus"
        :index-message="search.indexMessage"
        @show-hotkeys="onShowHotkeys"
      />
    </div>

    <ContextMenu
      :visible="showContextMenu"
      :x="contextMenuX"
      :y="contextMenuY"
      :item="contextMenuItem"
      @close="closeContextMenu"
    />

    <HotkeyModal :visible="showHotkeyModal" @close="showHotkeyModal = false" />
  </div>
</template>

<style scoped>
.search-page {
  width: 100%;
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: flex-start;
  background: var(--canvas);
  overflow: hidden;
  position: relative;
}

.search-container {
  width: 100%;
  max-width: 720px;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--surface);
  border-radius: 0;
  box-shadow: var(--shadow-xl);
  overflow: hidden;
  animation: search-container-fade-in var(--dur-normal) var(--ease-out);
}

@keyframes search-container-fade-in {
  from {
    0% {
      opacity: 0;
      transform: translateY(-12px) scale(0.97);
      filter: blur(8px);
    }
  }
  100% {
      opacity: 1;
      transform: translateY(0) scale(1);
      filter: blur(0);
    }
  }

.index-status-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
  background: linear-gradient(90deg, rgba(255, 107, 107, 0.1) 0%, rgba(255, 107, 107, 0.05) 100%);
  border-bottom: 1px solid rgba(255, 107, 107, 0.2);
  animation: status-bar-slide-in 0.3s var(--ease-out);
}

@keyframes status-bar-slide-in {
  from {
  0% {
    opacity: 0;
    transform: translateY(-100%);
  }
}
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.index-status-bar__left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.index-status-bar__icon {
  flex-shrink: 0;
}

.index-status-bar__icon--loading {
  color: var(--accent);
  animation: spin 1s linear infinite;
}

.index-status-bar__icon--success {
  color: #10b981;
}

.index-status-bar__icon--error {
  color: #ef4444;
}

.index-status-bar__text {
  font-size: 12px;
  color: var(--text-secondary);
}

.index-status-bar__action {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: var(--text-primary);
  font-size: 11px;
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
}

.index-status-bar__action:hover {
  background: rgba(255, 255, 255, 0.15);
  border-color: rgba(255, 255, 255, 0.25);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-4);
  padding: var(--sp-10) var(--sp-5);
}

.empty-state__icon {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-quaternary);
  opacity: 0.4;
  transition: all var(--dur-normal) var(--ease-out);
}

.empty-state:hover .empty-state__icon {
  opacity: 0.6;
  transform: scale(1.05);
}

.empty-state__text {
  color: var(--text-tertiary);
  font-size: var(--text-base);
  font-weight: 400;
}

.empty-state__hint {
  color: var(--text-quaternary);
  font-size: var(--text-sm);
}

.empty-state__action {
  margin-top: var(--sp-4);
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.slide-down-enter-active,
.slide-down-leave-active {
  transition: all var(--dur-fast) var(--ease-out);
}

.slide-down-enter-from,
.slide-down-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
