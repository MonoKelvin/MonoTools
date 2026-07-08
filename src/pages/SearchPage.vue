<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Search } from "@lucide/vue"
import { useSearchStore } from '@/stores/search'
import { useSettingsStore } from '@/stores/settings'
import { hotkeyApi } from '@/services'
import { isTauri } from '@/services/env'

import SearchInput from '@/components/common/SearchInput.vue'
import CategoryTabs from '@/components/search/CategoryTabs.vue'
import SearchResults from '@/components/search/SearchResults.vue'
import ActionBar from '@/components/search/ActionBar.vue'
import SettingsPanel from '@/components/panels/SettingsPanel.vue'
import StartupPanel from '@/components/panels/StartupPanel.vue'
import CommandsPanel from '@/components/panels/CommandsPanel.vue'

const search = useSearchStore()
const settings = useSettingsStore()

type Panel = 'search' | 'settings' | 'startup' | 'commands'
const currentPanel = ref<Panel>('search')

const showPanel = (panel: Panel) => {
  currentPanel.value = panel
}

const goSearch = () => {
  currentPanel.value = 'search'
}

const inputRef = ref<InstanceType<typeof SearchInput> | null>(null)

const onEnter = async () => {
  const item = await search.executeSelected()
  if (item) {
    // close handled by store
  }
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
})
</script>

<template>
  <div class="app-viewport">
    <!-- ========== 搜索视图 ========== -->
    <Transition name="fade" mode="out-in">
      <div v-if="currentPanel === 'search'" key="search" class="search-view">
        <div class="search-container">
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
                <Search class="empty-icon" :size="28" :stroke-width="1.5" />
                <span v-if="!search.query" class="empty-text">输入关键字开始搜索...</span>
                <span v-else class="empty-text">没有找到结果，试试别的关键字</span>
              </div>
            </template>
          </SearchResults>
          <ActionBar @go-panel="showPanel" />
        </div>
      </div>

      <!-- ========== 功能面板 ========== -->
      <div v-else key="panel" class="panel-view">
        <div class="panel-container">
          <div class="panel-header-bar">
            <button class="panel-back-btn" @click="goSearch">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>
              返回搜索
            </button>
            <h2 class="panel-header-title">
              <template v-if="currentPanel === 'settings'">设置</template>
              <template v-else-if="currentPanel === 'startup'">启动项管理</template>
              <template v-else-if="currentPanel === 'commands'">命令管理</template>
            </h2>
          </div>
          <SettingsPanel v-if="currentPanel === 'settings'" />
          <StartupPanel v-else-if="currentPanel === 'startup'" />
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

/* ========== 搜索视图 - 窗口即搜索区域 ========== */
.search-view {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.search-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  overflow: hidden;
}

/* ========== 功能面板视图 ========== */
.panel-view {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  z-index: 5;
  background: var(--bg-primary);
}

.panel-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.panel-header-bar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 16px;
  border-bottom: 1px solid var(--border);
  background: rgba(255, 255, 255, 0.015);
  flex-shrink: 0;
}
:global(.theme-light) .panel-header-bar {
  background: rgba(0, 0, 0, 0.01);
}

.panel-back-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 5px 10px;
  font-size: 12px;
  color: var(--text-secondary);
  background: transparent;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}
.panel-back-btn:hover {
  background: rgba(255, 255, 255, 0.06);
  color: var(--text-primary);
  border-color: var(--border-hover);
}
:global(.theme-light) .panel-back-btn:hover {
  background: rgba(0, 0, 0, 0.04);
}

.panel-header-title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: 0.02em;
}

/* ========== 空状态 ========== */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 50px 20px;
  flex: 1;
}
.empty-icon {
  color: var(--text-tertiary);
  opacity: 0.6;
}
.empty-text {
  color: var(--text-tertiary);
  font-size: 13px;
}

/* ========== 过渡动画 ========== */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s var(--ease-out), transform 0.15s var(--ease-out);
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
