<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount, nextTick, onUpdated, computed } from 'vue'
import { Search } from '@lucide/vue'
import { useSearchStore } from '@/stores/search'
import { useSettingsStore } from '@/stores/settings'
import { hotkeyApi, windowApi, shellApi } from '@/services'
import { isTauri } from '@/services/env'
import { useRouter } from 'vue-router'
import { listenEvent } from '@/services/tauri'
import type { SearchResult } from '@/types/search'
import { WINDOW_DIMENSIONS, UI_DELAYS, SEARCH_LIMITS_VISIBLE } from '@/config'

import SearchInput from '@/components/common/SearchInput.vue'
import VirtualGroupedResults from '@/components/search/VirtualGroupedResults.vue'
import ActionBar from '@/components/search/ActionBar.vue'
import ContextMenu from '@/components/search/ContextMenu.vue'
import HotkeyModal from '@/components/common/HotkeyModal.vue'
import { useCommandsStore, dispatchKeyEvent } from '@/commands'
import { useAppIcon } from '@/composables/useAppIcon'

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

const WINDOW_FIXED_WIDTH = WINDOW_DIMENSIONS.fixedWidth

/** 窗口最大高度 (不含顶部输入框 + 底部状态栏) */
const CONTENT_AREA_MAX = WINDOW_DIMENSIONS.contentAreaMax

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

/**
 * 单击 → 只更新 selectedIndex, 不打开.
 * 双击 / Enter / 业务触发 → executeItem (打开).
 * 这样避免"单击"与"双击"产生双重 IPC 调用, 也避免事件冒泡导致打开两次.
 */
const onSelect = (item: SearchResult) => {
  // 通过 store.selectByIndex 选中, 自动同步 selectedGlobalId 锚点,
  // 避免列表重排时 selectedIndex 指向错误位置.
  search.selectByIndex(search.displayList.findIndex((r) => r.id === item.id))
}
const onOpen = (item: SearchResult) => {
  search.executeItem(item)
}
const onShowHotkeys = () => {
  showHotkeyModal.value = true
}
const onHover = (idx: number) => {
  // hover 也用 selectByIndex 同步 ID 锚点, 保持状态一致.
  search.selectByIndex(idx)
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

// ============================================================================
// 图标批量预取
// ============================================================================
//
// 修复"首屏 200 个结果 = 200 次 IPC"问题: 监听 displayList 变化,
// 防抖 200ms 后调用 useAppIcon.loadIconsBatch 一次性拉满首屏可见项.
// 防抖避免连打 "chrome" 6 个字符触发 6 次 batch.
const { loadIconsBatch } = useAppIcon()
let iconBatchTimer: ReturnType<typeof setTimeout> | null = null
watch(
  () => search.displayList.slice(0, SEARCH_LIMITS_VISIBLE.iconBatchPrefetch),
  (items) => {
    if (!isTauri || items.length === 0) return
    if (iconBatchTimer) clearTimeout(iconBatchTimer)
    iconBatchTimer = setTimeout(() => {
      loadIconsBatch(items).catch(() => undefined)
    }, UI_DELAYS.iconBatchDebounceMs)
  },
  { deep: false },
)

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

/** 右键菜单"固定 / 取消固定" → 调用 store 切换 pin 状态. */
const handleContextMenuPinToggle = (item: SearchResult) => {
  search.togglePin(item.id).catch(() => undefined)
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
  const item = search.displayList[search.selectedIndex]
  if (!item) return
  try {
    await navigator.clipboard.writeText(item.subtitle || item.title)
  } catch {}
}

const revealSelected = async () => {
  const item = search.displayList[search.selectedIndex]
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

// ============================================================================
// 废弃: displayResults / defaultResults 已被移除.
//
// 原因: store.displayList 已经是"折叠 + 分类 + 文件类型过滤"后的
// 最终可见列表. SearchPage 再次做一层过滤 (排除 pinned/recent) 造成
// 长度不一致, 键盘上下方向键可以指向当前不可见的项.
//
// 现在所有选择逻辑统一使用 store.displayList, 确保键盘导航
// 与 VGR 渲染严格 1:1.
// ============================================================================

/**
 * 给 VirtualGroupedResults 的分组结构.
 * 单一真源: store.displayGroups 已经包含"折叠 + 分类 + 文件类型过滤"
 * 的全部计算. SearchPage 只做绑定, 不再二次加工.
 *
 * 注意: 这里必须用 `search.displayGroups` (而非 `displayList`) ,
 * VGR 期望的是"按组分类的二维结构", 不是扁平数组.
 */
const displayGroups = computed(() => search.displayGroups)

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
  // 启动后从后端拉取已 pin 的 id 列表, 让"固定项目"分组即时显示.
  search.loadPinned().catch(() => undefined)
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
        :groups="displayGroups"
        :loading="search.loading"
        :selected-index="search.selectedIndex"
        :height="contentHeight"
        :has-query="!!search.query"
        :query="search.query"
        @select="onSelect"
        @open="onOpen"
        @hover="onHover"
        @contextmenu="handleContextMenu"
        @toggle-group="(id) => search.toggleGroupCollapse(id)"
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
        :results="search.displayList"
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
      @pin-toggle="handleContextMenuPinToggle"
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
  /* 220ms 对齐 ICON_CONFIG.fadeInMs (见 @/config/icon.ts). 改 SCSS 同步改 TS. */
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
