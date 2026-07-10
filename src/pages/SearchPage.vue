<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount, nextTick, onUpdated } from 'vue'
import { Search } from "@lucide/vue"
import { useSearchStore } from '@/stores/search'
import { useSettingsStore } from '@/stores/settings'
import { hotkeyApi, windowApi } from '@/services'
import { isTauri } from '@/services/env'
import { useRouter } from 'vue-router'
import { listenEvent } from '@/services/tauri'

import SearchInput from '@/components/common/SearchInput.vue'
import CategoryTabs from '@/components/search/CategoryTabs.vue'
import SearchResults from '@/components/search/SearchResults.vue'
import ActionBar from '@/components/search/ActionBar.vue'

const search = useSearchStore()
const settings = useSettingsStore()
const router = useRouter()

const inputRef = ref<InstanceType<typeof SearchInput> | null>(null)
const containerRef = ref<HTMLElement | null>(null)
let resizeObserver: ResizeObserver | null = null
let pendingHeight = 0
let resyncTimer: number | null = null
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
  if (resyncTimer) window.clearTimeout(resyncTimer)
  resyncTimer = window.setTimeout(() => windowApi.setHeight(h), 50)
}

const onEnter = async () => { await search.executeSelected() }
const onUp = () => search.selectPrev()
const onDown = () => search.selectNext()
const onEscape = () => { search.hide() }

const onSelect = (item: any) => search.executeItem(item)
const onHover = (idx: number) => { search.selectedIndex = idx }
const onQueryChange = (val: string) => search.setQuery(val)
const onCategorySelect = (cat: any) => search.setCategory(cat)

const tryRegisterHotkey = async () => {
  if (!isTauri) return
  try { await hotkeyApi.register(settings.settings.hotkey) }
  catch {}
}

const handleIndexProgress = (progress: { status: string; message?: string; files?: number }) => {
  search.setIndexProgress(progress)
}

onMounted(async () => {
  await settings.load()
  await tryRegisterHotkey()
  await fixWindowWidth()
  await search.loadIndexStatus()

  unlistenIndexProgress = await listenEvent('index_progress', handleIndexProgress)

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
  if (resizeObserver) { resizeObserver.disconnect(); resizeObserver = null }
  if (unlistenIndexProgress) { unlistenIndexProgress() }
})

watch(() => router.currentRoute.value.path, () => nextTick(syncWindowHeight))
</script>

<template>
  <div class="search-page" data-tauri-drag-region>
    <div class="search-container" ref="containerRef">
      <SearchInput
        ref="inputRef"
        :model-value="search.query"
        @update:model-value="onQueryChange"
        @enter="onEnter"
        @arrow-down="onDown"
        @arrow-up="onUp"
        @escape="onEscape"
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
      />
    </div>
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
    opacity: 0;
    transform: translateY(-12px) scale(0.97);
    filter: blur(8px);
  }
  to {
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
    opacity: 0;
    transform: translateY(-100%);
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
  to { transform: rotate(360deg); }
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
