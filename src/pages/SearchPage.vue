<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount, nextTick, onUpdated, computed } from 'vue'
import { Search, Sparkles, HardDrive, FileText } from '@lucide/vue'
import { useSearchStore } from '@/stores/search'
import { useSettingsStore } from '@/stores/settings'
import { hotkeyApi, windowApi, shellApi } from '@/services'
import { isTauri } from '@/services/env'
import { useRouter } from 'vue-router'
import { listenEvent } from '@/services/tauri'
import type { SearchResult } from '@/types/search'

import SearchInput from '@/components/common/SearchInput.vue'
import VirtualGroupedResults from '@/components/search/VirtualGroupedResults.vue'
import ActionBar from '@/components/search/ActionBar.vue'
import ContextMenu from '@/components/search/ContextMenu.vue'
import HotkeyModal from '@/components/common/HotkeyModal.vue'
import { useCommandsStore, dispatchKeyEvent } from '@/commands'

const showContextMenu = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const contextMenuItem = ref<SearchResult | null>(null)
const showHotkeyModal = ref(false)
const useGroupedView = ref(true)

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

const WINDOW_FIXED_WIDTH = 640

/** 窗口最大高度 (不含顶部输入框 + 底部状态栏) */
const CONTENT_AREA_MAX = 460

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
  // 容器原始高度 (不限制) - 用于在容器溢出时计算滚动区
  const naturalH = Math.round(rect.height)
  // 限制为最大高度, 防止搜索结果过多时窗口过高
  const h = Math.min(naturalH, CONTENT_AREA_MAX + 88)
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

const tryRegisterHotkey = async () => {
  if (!isTauri) return
  try {
    await hotkeyApi.register(settings.settings.hotkey)
  } catch {}
}

const handleIndexProgress = (progress: {
  status: string
  message?: string
  files?: number
  phase?: string
}) => {
  search.setIndexProgress(progress)
  // 应用索引就绪后, 若当前是空查询, 自动刷新一次让应用出现在首屏
  if (progress.phase === 'apps' && progress.status === 'completed' && !search.query) {
    search.runSearch().catch(() => undefined)
  }
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
  event.preventDefault()
  await commandsStore.execute(id).catch(() => undefined)
}

// === 未搜索状态数据 (默认推荐内容) ===
// 当 query 为空时, 把整个 results 列表传给分组组件 (内部会按
// 类别/类型拆分到 "系统应用" / "所有应用" / "所有文件" / "命令" 分组里)。
// 固定项目和最近访问分组由 VirtualGroupedResults 内部根据 search.pinned / search.recent 决定。
const defaultResults = computed<SearchResult[]>(() => {
  if (search.query) return []
  // 排除固定项目 + 最近访问(它们已有独立分组), 避免在下方重复展示.
  const excludedIds = new Set([
    ...search.pinned.map((p) => p.id),
    ...search.recent.map((p) => p.id),
  ])
  return search.filteredResults.filter((r) => !excludedIds.has(r.id))
})

onMounted(async () => {
  commandsStore.loadFromBackend().catch(() => undefined)
  window.addEventListener('keydown', handleGlobalKeydown)
  if (containerRef.value) {
    containerRef.value.addEventListener('contextmenu', handleContextMenu)
  }

  unlistenIndexProgress = await listenEvent('index_progress', handleIndexProgress)

  await nextTick()
  try {
    const { call } = await import('@/services/tauri')
    await call('frontend_ready', {})
  } catch {
    // 非 tauri 环境(浏览器 mock)忽略即可.
  }

  search.initialLoad().catch(() => undefined)
  settings.load().catch(() => undefined)
  tryRegisterHotkey().catch(() => undefined)
  fixWindowWidth().catch(() => undefined)
  search.loadIndexStatus().catch(() => undefined)

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

// 计算: 搜索结果区域高度 = 容器高 - 输入框 - 状态栏
// (无分类横幅, 无 status 区域, 仅输入框 + 列表 + 状态栏)
const contentHeight = computed(() => Math.max(240, pendingHeight - 88))
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

      <!-- Raycast 风格统一分组列表 (黑白灰, 简单分隔, 无卡片化) -->
      <VirtualGroupedResults
        v-if="useGroupedView"
        :results="search.query ? search.filteredResults : defaultResults"
        :loading="search.loading"
        :selected-index="search.selectedIndex"
        :height="contentHeight"
        :pinned="search.pinned"
        :recent="search.recent"
        :has-query="!!search.query"
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
      </VirtualGroupedResults>

      <ActionBar
        :results="search.filteredResults"
        :selected-index="search.selectedIndex"
        :index-building="search.indexStatus === 'building'"
        :index-status="search.indexStatus"
        :index-message="search.indexMessage"
        :index-volumes-total="search.indexVolumesTotal"
        :index-volume-index="search.indexVolumeIndex"
        :index-current-volume="search.indexCurrentVolume"
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
  background: transparent;
  overflow: hidden;
  position: relative;
}

/* 容器: 黑白灰 + 极轻玻璃 + 圆角 (上对齐, 无多余装饰) */
.search-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--glass-bg);
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  border-radius: 0;
  border-top: 1px solid var(--glass-border);
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.55);
  overflow: hidden;
  animation: search-container-fade-in 220ms var(--ease-out);
}

/* 当 Mica 不支持 (Win10 及以下) 时, 退化为柔化背景 */
@supports not ((backdrop-filter: blur(40px)) or (-webkit-backdrop-filter: blur(40px))) {
  .search-container {
    background: var(--canvas);
  }
}

@keyframes search-container-fade-in {
  0% {
    opacity: 0;
    transform: translateY(-8px) scale(0.985);
    filter: blur(6px);
  }
  100% {
    opacity: 1;
    transform: translateY(0) scale(1);
    filter: blur(0);
  }
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-3);
  padding: var(--sp-10) var(--sp-5);
}

.empty-state__icon {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-quaternary);
  opacity: 0.35;
  transition: all var(--dur-normal) var(--ease-out);
}

.empty-state:hover .empty-state__icon {
  opacity: 0.55;
  transform: scale(1.05);
}

.empty-state__text {
  color: var(--text-secondary);
  font-size: 14px;
  font-weight: 500;
  letter-spacing: -0.005em;
}

.empty-state__hint {
  color: var(--text-quaternary);
  font-size: 12px;
  font-weight: 400;
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
