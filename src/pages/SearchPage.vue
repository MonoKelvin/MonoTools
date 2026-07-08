<script setup lang="ts">
import { ref, watch, computed, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { Search } from "@lucide/vue"
import { useSearchStore } from '@/stores/search'
import { useSettingsStore } from '@/stores/settings'
import { hotkeyApi, windowApi } from '@/services'
import { isTauri } from '@/services/env'

import SearchInput from '@/components/common/SearchInput.vue'
import CategoryTabs from '@/components/search/CategoryTabs.vue'
import SearchResults from '@/components/search/SearchResults.vue'
import ActionBar from '@/components/search/ActionBar.vue'
import SettingsPanel from '@/components/panels/SettingsPanel.vue'
import CommandsPanel from '@/components/panels/CommandsPanel.vue'

const search = useSearchStore()
const settings = useSettingsStore()

type Panel = 'search' | 'settings' | 'commands'
const currentPanel = ref<Panel>('search')

const showPanel = (panel: Panel) => {
  currentPanel.value = panel
}

const goSearch = () => {
  currentPanel.value = 'search'
}

const inputRef = ref<InstanceType<typeof SearchInput> | null>(null)
const containerRef = ref<HTMLElement | null>(null)
let resizeObserver: ResizeObserver | null = null
let pendingHeight = 0
let resyncTimer: number | null = null

const syncWindowHeight = () => {
  if (!isTauri) return
  if (!containerRef.value) return
  const rect = (containerRef.value as HTMLElement).getBoundingClientRect()
  const h = Math.round(rect.height)
  if (Math.abs(h - pendingHeight) < 2) return
  pendingHeight = h
  if (resyncTimer) window.clearTimeout(resyncTimer)
  resyncTimer = window.setTimeout(() => {
    windowApi.setHeight(h)
  }, 50)
}

const onEnter = async () => {
  await search.executeSelected()
}
const onUp = () => search.selectPrev()
const onDown = () => search.selectNext()
const onEscape = () => {
  search.hide()
  if (currentPanel.value !== 'search') {
    goSearch()
  }
}

const onSelect = (item: any) => search.executeItem(item)
const onHover = (idx: number) => {
  search.selectedIndex = idx
}

const onQueryChange = (val: string) => search.setQuery(val)

const onCategorySelect = (cat: any) => search.setCategory(cat)

const tryRegisterHotkey = async () => {
  if (!isTauri) return
  try {
    await hotkeyApi.register(settings.settings.hotkey)
  } catch (err) {
    console.warn('注册热键失败', err)
  }
}

onMounted(async () => {
  await settings.load()
  await tryRegisterHotkey()
  await nextTick()
  if (containerRef.value && typeof ResizeObserver !== 'undefined') {
    resizeObserver = new ResizeObserver(syncWindowHeight)
    resizeObserver.observe(containerRef.value)
    syncWindowHeight()
  }
})

onBeforeUnmount(() => {
  if (resizeObserver) {
    resizeObserver.disconnect()
    resizeObserver = null
  }
})

watch(currentPanel, () => {
  nextTick(syncWindowHeight)
})

watch(
  () => search.filteredResults.length,
  () => nextTick(syncWindowHeight),
)
</script>

<template>
  <div class="app-viewport">
    <!-- ========== 搜索视图 ========== -->
    <Transition name="fade" mode="out-in">
      <div
        v-if="currentPanel === 'search'"
        key="search"
        class="search-view"
      >
        <div
          ref="containerRef"
          class="search-container"
        >
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
                <Search class="empty-icon" :size="26" :stroke-width="1.6" />
                <span v-if="!search.query" class="empty-text">输入关键字开始搜索</span>
                <span v-else class="empty-text">没有找到结果，试试别的关键字</span>
              </div>
            </template>
          </SearchResults>
          <ActionBar />
        </div>
      </div>

      <!-- ========== 功能面板 ========== -->
      <div v-else key="panel" class="panel-view">
        <div class="panel-container" data-tauri-drag-region>
          <div class="panel-header-bar">
            <button class="panel-back-btn" @click="goSearch">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>
              返回搜索
            </button>
            <h2 class="panel-header-title">
              <template v-if="currentPanel === 'settings'">设置</template>
              <template v-else-if="currentPanel === 'commands'">命令管理</template>
            </h2>
          </div>
          <SettingsPanel v-if="currentPanel === 'settings'" />
          <CommandsPanel v-else-if="currentPanel === 'commands'" />
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.app-viewport {
  width: 100%;
  height: 100%;
  position: relative;
  overflow: hidden;
}

/* ========== 搜索视图 ========== */
.search-view {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.search-container {
  width: 100%;
  display: flex;
  flex-direction: column;
  background: var(--surface);
  border: 1px solid var(--hairline);
  border-radius: 0;
  box-shadow:
    0 12px 48px rgba(0, 0, 0, 0.55),
    0 1px 0 rgba(255, 255, 255, 0.02) inset;
  overflow: hidden;
  transition: height var(--duration-slow) var(--ease-out);
  min-height: 200px;
}

/* ========== 功能面板视图 ========== */
.panel-view {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  z-index: 5;
}

.panel-container {
  width: 100%;
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--surface);
  border: 1px solid var(--hairline);
  border-radius: 0;
  box-shadow:
    0 12px 48px rgba(0, 0, 0, 0.55);
  overflow: hidden;
  min-height: 0;
}

.panel-header-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--hairline);
  background: transparent;
  flex-shrink: 0;
}

.panel-back-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  font-size: 12px;
  font-weight: 500;
  color: var(--text-body);
  background: transparent;
  border: 1px solid var(--hairline);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}
.panel-back-btn:hover {
  background: var(--surface-elevated);
  color: var(--on-dark);
  border-color: var(--hairline-strong);
}

.panel-header-title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-ink);
  letter-spacing: 0.005em;
}

/* ========== 空状态 ========== */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 40px 20px;
  flex: 1;
}
.empty-icon {
  color: var(--text-mute);
  opacity: 0.7;
}
.empty-text {
  color: var(--text-ash);
  font-size: 13px;
  letter-spacing: 0.005em;
}

/* ========== 过渡动画 ========== */
.fade-enter-active,
.fade-leave-active {
  transition: opacity var(--duration-normal) var(--ease-out),
    transform var(--duration-normal) var(--ease-out);
}
.fade-enter-from {
  opacity: 0;
  transform: translateY(4px);
}
.fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
