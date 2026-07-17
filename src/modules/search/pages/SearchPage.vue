<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount, nextTick, onUpdated, computed } from 'vue'
import { Pin, Clock, Settings, Terminal, Sparkles, Folder } from '@lucide/vue'
import { useSearchStore, GROUP_ID } from '@/modules/search'
import type { DisplayGroup } from '@/modules/search'
import type { MtComboBoxOption } from '@/ui/components/MtComboBox.vue'
import type { SortMode } from '@/core/config/sorting'
import { useSettingsStore } from '@/core/stores/settings'
import { hotkeyApi, windowApi, shellApi } from '@/services'
import { isTauri } from '@/services/env'
import { useRouter } from 'vue-router'
import { listenEvent } from '@/services/tauri'
import type { SearchResult } from '@/modules/search'
import { WINDOW_DIMENSIONS, UI_DELAYS, SEARCH_LIMITS_VISIBLE } from '@/core/config'
import { LayoutList, Grid3X3, WrapText, LayoutGrid, Type } from '@lucide/vue'

import SearchInput from '@/modules/search/components/SearchInput.vue'
import GroupSection from '@/modules/search/components/GroupSection.vue'
import ActionBar from '@/modules/search/components/ActionBar.vue'
import ContextMenu from '@/modules/search/components/ContextMenu.vue'
import HotkeyModal from '@/ui/widgets/HotkeyModal.vue'
import OverlayPage from '@/ui/pages/OverlayPage.vue'
import { useCommandsStore, dispatchKeyEvent } from '@/core/command'
import { useAppIcon } from '@/ui/widgets/appicon/useAppIcon'
import { useSearchStatusBar } from '@/modules/search/composables/useSearchStatusBar'

const showContextMenu = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const contextMenuItem = ref<SearchResult | null>(null)
const showHotkeyModal = ref(false)

const search = useSearchStore()
const settings = useSettingsStore()
const router = useRouter()
const commandsStore = useCommandsStore()

// 状态栏编排: 业务状态 → 通用 StatusBarMessage, 交给 ActionBar 渲染.
const { message: statusBarMessage } = useSearchStatusBar(search)

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
const onSelect = (item: SearchResult, _globalIndex: number, _event: MouseEvent) => {
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
const onHover = (globalIndex: number) => {
  // hover 也用 selectByIndex 同步 ID 锚点, 保持状态一致.
  if (globalIndex >= 0) {
    search.selectByIndex(globalIndex)
  }
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

// ============================================================================
// 分组渲染: 每个分组使用 GroupSection 组件, 支持独立的显示模式切换
// ============================================================================

/**
 * 分组结构. 单一真源: store.displayGroups
 */
const displayGroups = computed(() => search.displayGroups)

/**
 * 过滤掉 items.length === 0 的分组, 避免渲染空 GroupSection.
 * 固定项目为空时, 不显示"固定项目"分组.
 */
const visibleGroups = computed(() =>
  search.displayGroups.filter((g) => g.items.length > 0),
)

/** 分组图标映射 */
const GROUP_ICONS: Record<DisplayGroup['kind'], any> = {
  pinned: Pin,
  recent: Clock,
  system: Settings,
  commands: Terminal,
  apps: Sparkles,
  files: Folder,
}

/** 支持智能排序的分组类型 */
const SMART_SORT_KINDS = ['commands', 'apps'] as const

/** 各分组类型的排序选项 */
const sortOptionsByKind: Record<string, MtComboBoxOption[]> = {
  pinned: [
    { key: 'recent', label: '最近访问', icon: Clock },
    { key: 'name', label: '名称', icon: Type },
    { key: 'path', label: '路径', icon: Folder },
  ],
  recent: [
    { key: 'recent', label: '最近访问', icon: Clock },
    { key: 'name', label: '名称', icon: Type },
  ],
  apps: [
    { key: 'smart', label: '智能排序', icon: Sparkles },
    { key: 'name', label: '名称', icon: Type },
    { key: 'recent', label: '最近访问', icon: Clock },
    { key: 'path', label: '路径', icon: Folder },
  ],
  system: [
    { key: 'name', label: '名称', icon: Type },
  ],
  commands: [
    { key: 'smart', label: '智能排序', icon: Sparkles },
    { key: 'name', label: '名称', icon: Type },
  ],
  files: [
    { key: 'name', label: '名称', icon: Type },
    { key: 'path', label: '路径', icon: Folder },
  ],
}

/** 各分组类型的默认排序模式 */
const DEFAULT_SORT_BY_KIND: Record<string, SortMode> = {
  pinned: 'recent',
  recent: 'recent',
  apps: 'smart',
  system: 'name',
  commands: 'name',
  files: 'name',
}

/** 滚动容器 ref */
const resultsScrollRef = ref<HTMLElement | null>(null)

/**
 * 计算每个分组的 startIndex (在 displayList 中的起始位置).
 * 用于 GroupSection 的 selectedGlobalIndex / hoveredGlobalIndex 计算.
 * 注意: 只遍历 visibleGroups，保证与渲染的分组一致.
 */
const groupStartIndices = computed(() => {
  const map = new Map<string, number>()
  let idx = 0
  for (const g of visibleGroups.value) {
    map.set(g.id, idx)
    idx += g.visibleItems.length
  }
  return map
})

function onToggleGroup(groupId: string) {
  search.toggleGroupCollapse(groupId)
}

/**
 * 选中项变化时, 滚动到可见区域.
 */
watch(
  () => search.selectedIndex,
  () => {
    nextTick(() => {
      if (!resultsScrollRef.value) return
      const activeEl = resultsScrollRef.value.querySelector('.gs-item--active') as HTMLElement | null
      if (activeEl) {
        activeEl.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
      }
    })
  },
)

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
  <OverlayPage>
    <div class="search-container" id="search-container" ref="containerRef">
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

      <!-- 分组列表: 每个分组使用 GroupSection 组件, 支持独立的显示模式切换 -->
      <div class="results-scroll-container" ref="resultsScrollRef">
        <template v-if="visibleGroups.length > 0">
          <GroupSection
            v-for="group in visibleGroups"
            :key="group.id"
            :id="group.id"
            :title="group.title"
            :icon="GROUP_ICONS[group.kind]"
            :items="group.visibleItems"
            :collapsed-items="group.items"
            :collapsed="group.collapsed"
            :kind="group.kind"
            :count="group.items.length"
            :sort-mode="search.groupSortModes[group.id] || DEFAULT_SORT_BY_KIND[group.kind]"
            :sort-options="sortOptionsByKind[group.kind]"
            :selected-global-index="search.selectedIndex"
            :hovered-global-index="search.selectedIndex"
            :start-index="groupStartIndices.get(group.id) ?? 0"
            @toggle-collapse="onToggleGroup"
            @select="onSelect"
            @open="onOpen"
            @hover="onHover"
            @contextmenu="handleContextMenu"
            @sort-change="(mode) => search.setGroupSortMode(group.id, mode)"
          />
        </template>
        <template v-else>
          <MtEmptyState
            :icon="search.query ? 'no-results' : 'search'"
            :title="search.query ? '没有找到结果' : '输入关键字搜索应用、文件、命令'"
            :hint="search.query ? '尝试使用不同的关键词' : '支持拼音、首字母、模糊匹配'"
            padding="xl"
          />
        </template>
      </div>

      <ActionBar
        :message="statusBarMessage"
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
  </OverlayPage>
</template>

<style scoped>
/* 浮窗容器样式由 ui/pages/OverlayPage.vue 提供, 此处只写 SearchPage 专属的 */

/* 主容器: flex column, 让 SearchInput / results / ActionBar 正确伸缩 */
.search-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0; /* 关键: 让嵌套 flex 子项的 overflow-y: auto 生效 */
}

/* 搜索结果区域 - 高度由外层动态控制 */
.results-scroll-container {
  flex: 1;
  /* overflow-x: visible 让分组头行的 ::before 分割线能穿过 padding 贴到
     窗口两边. overflow-y 保持 auto, 纵向滚动不受影响.
     注: overflow-x 与 overflow-y 一边 visible 时, CSS 规范会强制另一边
     变为 auto. 但这里 overflow-y 本来就是 auto, 所以无副作用. */
  overflow-x: visible;
  overflow-y: auto;
  padding: var(--sp-2) var(--sp-3);
  min-height: 0; /* 关键: 让 flex 子项能正确收缩并出现滚动条 */
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
