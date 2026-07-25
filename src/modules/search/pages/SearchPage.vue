<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount, nextTick, onUpdated, computed } from 'vue'
import { Pin, Clock, Settings, Terminal, Sparkles, Folder, LayoutList, Grid3X3, WrapText, LayoutGrid, Type, CalendarDays, HardDrive, Tag } from '@lucide/vue'
import { useSearchStore, GROUP_ID } from '@/modules/search'
import type { DisplayGroup, GroupId } from '@/modules/search'
import type { MtComboBoxOption } from '@/ui/components/MtComboBox.vue'
import type { SortMode } from '@/core/config/sorting'
import { useSettingsStore } from '@/core/stores/settings'
import { useThemeStore } from '@/core/stores/theme'
import { windowApi, shellApi } from '@/services'
import { isTauri } from '@/services/env'
import { useRouter } from 'vue-router'
import { listenEvent } from '@/services/tauri'
import type { SearchResult } from '@/modules/search'
import { WINDOW_DIMENSIONS, UI_DELAYS, SEARCH_LIMITS_VISIBLE } from '@/core/config'

import SearchInput from '@/modules/search/components/SearchInput.vue'
import GroupSection from '@/modules/search/components/GroupSection.vue'
import ActionBar from '@/modules/search/components/ActionBar.vue'
import ContextMenu from '@/modules/search/components/ContextMenu.vue'
import HotkeyModal from '@/ui/widgets/HotkeyModal.vue'
import OverlayPage from '@/ui/pages/OverlayPage.vue'
import LoadingState from '@/ui/components/LoadingState.vue'
import { MtEmptyState } from '@/ui/components'
import { useCommandsStore, dispatchKeyEvent } from '@/core/command'
import { useAppIcon } from '@/ui/widgets/appicon/useAppIcon'
import { useSearchStatusBar } from '@/modules/search/composables/useSearchStatusBar'
import { buildContextMenuItems } from '@/modules/search/composables/useContextMenuItems'
import { builtinCommandSpecs } from '@/core/command'

const showContextMenu = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const contextMenuItems = ref<import('@/ui/components/MtMenu.vue').MtMenuItem[]>([]) // 预计算好的菜单项
const contextMenuKind = ref('apps') // 右键菜单所在分组 kind, 用于差异化菜单
/** 当前右键菜单对应的搜索结果项，用于 action 处理时获取目标 item */
const contextMenuItemRef = ref<SearchResult | null>(null)
const showHotkeyModal = ref(false)
const showLoading = ref(true)
const hoveredGlobalIndex = ref(-1)

const search = useSearchStore()
const settings = useSettingsStore()
const themeStore = useThemeStore()
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
let unlistenWindowMonitor: (() => void) | null = null
let periodicRecommendTimer: ReturnType<typeof setInterval> | null = null

const WINDOW_FIXED_WIDTH = WINDOW_DIMENSIONS.fixedWidth

/** 窗口最大高度 (不含顶部输入框 + 底部状态栏) */
const CONTENT_AREA_MAX = WINDOW_DIMENSIONS.contentAreaMax

const fixWindowWidth = async () => {
  if (!isTauri) return
  try {
    const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow')
    const { LogicalSize } = await import('@tauri-apps/api/dpi')
    const win = WebviewWindow.getCurrent()
    const currentSize = await win.innerSize()
    if (Math.abs(currentSize.width - WINDOW_FIXED_WIDTH) > 1) {
      await win.setSize(new LogicalSize(WINDOW_FIXED_WIDTH, currentSize.height))
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
 *
 * 支持鼠标多选修饰键 (与键盘保持一致):
 *   - Ctrl      → 切换该项选中状态 (toggle)
 *   - Shift     → 范围选中
 *   - Ctrl+Shift→ 范围反选
 */
const onSelect = (groupId: string, localIndex: number, event: MouseEvent) => {
  // 每个分组独立且互斥, 直接调用 store 的分组选择方法
  search.selectWithModifiers(groupId, localIndex, event.ctrlKey || event.metaKey, event.shiftKey)
}
const onOpen = (item: SearchResult) => {
  search.executeItem(item)
}
const onShowHotkeys = () => {
  showHotkeyModal.value = true
}
const onHover = (globalIndex: number) => {
  hoveredGlobalIndex.value = globalIndex
}
const onQueryChange = (val: string) => search.setQuery(val)

const handleIndexProgress = (progress: {
  status: string
  message?: string
  files?: number
  phase?: string
}) => {
  search.setIndexProgress(progress)
  // 增量刷新由 store.triggerIncrementalRefresh() 内部防抖处理,
  // 此处不再额外调用 runSearch(), 避免与防抖定时器竞速造成 UI 抖动.
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

const handleContextMenu = (e: MouseEvent | CustomEvent, item?: SearchResult, kind?: string) => {
  // Logo 菜单导航: 当前由 SearchInput 内部直接 router.push 路由跳转,
  // 此分支仅作兜底 (向后兼容旧调用方), 如果收到 nav-to-* 自定义事件,
  // 静默忽略, 避免重复跳转或 throw 在 CustomEvent 上调用 preventDefault.
  if (typeof e === 'object' && 'type' in e && typeof e.type === 'string' && e.type.startsWith('nav-to-')) {
    return
  }

  // 搜索结果右键菜单
  e.preventDefault()
  if (!item) return
  contextMenuX.value = e.clientX
  contextMenuY.value = e.clientY
  contextMenuKind.value = kind ?? 'apps'

  // 保存当前 item 引用，用于 action 处理
  contextMenuItemRef.value = item
  // 动态构建菜单项：根据分组 kind 和 item 状态
  const isPinned = search.isPinned(item.id)
  // 命令分组：判断是否为内置命令
  const isBuiltin = (kind === 'commands' && builtinCommandSpecs.some((s) => s.id === item.id))
  contextMenuItems.value = buildContextMenuItems({ item, kind: kind ?? '', isPinned, isBuiltin })

  showContextMenu.value = true
}

const closeContextMenu = () => {
  showContextMenu.value = false
  contextMenuItems.value = []
  contextMenuItemRef.value = null
}

/** 右键菜单 action 统一分发: pin-toggle / edit-command / delete-command */
const handleContextMenuAction = (menuItem: import('@/ui/components/MtMenu.vue').MtMenuItem) => {
  const item = contextMenuItemRef.value
  if (!item) return
  const key = menuItem.key
  if (!key) return

  switch (key) {
    case 'pin-toggle':
      search.togglePin(item.id).catch(() => undefined)
      break
    case 'edit-command':
      // TODO: 打开命令编辑弹窗 (CommandsPanel 已有 CRUD 逻辑)
      console.info('[ContextMenu] edit command:', item.id)
      break
    case 'delete-command':
      commandsStore.deleteCommand?.(item.id).catch(() => undefined)
      break
  }
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
  ],
  recent: [
    { key: 'recent', label: '最近访问', icon: Clock },
    { key: 'name', label: '名称', icon: Type },
    { key: 'path', label: '路径', icon: Folder },
  ],
  apps: [
    { key: 'smart', label: '智能排序', icon: Sparkles },
    { key: 'name', label: '名称', icon: Type },
    { key: 'recent', label: '最近访问', icon: Clock },
    { key: 'path', label: '路径', icon: Folder },
    { key: 'type', label: '类型', icon: Tag },
  ],
  system: [
    { key: 'name', label: '名称', icon: Type },
  ],
  commands: [
    { key: 'smart', label: '智能排序', icon: Sparkles },
    { key: 'name', label: '名称', icon: Type },
    { key: 'recent', label: '最近访问', icon: Clock },
  ],
  files: [
    { key: 'name', label: '名称', icon: Type },
    { key: 'path', label: '路径', icon: Folder },
    { key: 'modified', label: '修改时间', icon: CalendarDays },
    { key: 'size', label: '大小', icon: HardDrive },
    { key: 'type', label: '类型', icon: Tag },
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

function onToggleGroup(groupId: GroupId) {
  search.toggleGroupCollapse(groupId)
}

/**
 * 选中项变化时, 仅在元素不在可视区域内时才滚动.
 * 避免跨 GroupSection 时误触发滚动导致列表跳到顶部.
 */
watch(
  () => search.selectedIndex,
  () => {
    nextTick(() => {
      if (!resultsScrollRef.value) return
      const activeEl = resultsScrollRef.value.querySelector('.gs-item--active') as HTMLElement | null
      if (!activeEl) return
      const container = resultsScrollRef.value
      const containerRect = container.getBoundingClientRect()
      const elRect = activeEl.getBoundingClientRect()
      // 元素已在可视区域内则跳过，避免不必要的滚动
      const isVisible = elRect.top >= containerRect.top - 1 && elRect.bottom <= containerRect.bottom + 1
      if (!isVisible) {
        activeEl.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
      }
    })
  },
)

onMounted(async () => {
  window.addEventListener('keydown', handleGlobalKeydown)
  if (containerRef.value) {
    containerRef.value.addEventListener('contextmenu', handleContextMenu)
  }

  unlistenIndexProgress = await listenEvent('index_progress', handleIndexProgress)

  // 启动 WindowMonitor 订阅: 切换激活应用时, store 立刻刷新推荐.
  if (!unlistenWindowMonitor) {
      unlistenWindowMonitor = await search.listenWindowMonitor()
  }

  // 定时同步 (兜底, 即便事件流异常也保持推荐新鲜)
  if (!periodicRecommendTimer) {
      periodicRecommendTimer = setInterval(() => {
          search.syncWindowMonitor().catch(() => undefined)
      }, 60_000)
  }

  // 并行执行独立的启动任务，提升启动速度
  // 这些任务之间没有依赖关系，可以同时进行
  await Promise.all([
    search.loadPinned().catch(() => undefined),
    settings.load().catch(() => undefined),
    fixWindowWidth().catch(() => undefined),
  ])

  if (containerRef.value && typeof ResizeObserver !== 'undefined') {
    resizeObserver = new ResizeObserver(syncWindowHeight)
    resizeObserver.observe(containerRef.value)
    syncWindowHeight()
  }

  // 等一帧确保 DOM 布局稳定后显示窗口 (一帧足够).
  await nextTick()
  try {
    const { call } = await import('@/services/tauri')
    await call('frontend_ready', {})
  } catch {
    // 非 tauri 环境(浏览器 mock)忽略即可.
  }

  // 后端已显示窗口, 淡出加载状态
  showLoading.value = false

  // frontend_ready 后并行执行: 命令 spec + 索引状态 + 初始搜索
  // 命令 spec 原来是启动阶段第二大 IPC 阻塞点, 挪到窗口已显示之后.
  // 三者都是 IPC 调用, 互不依赖.
  await Promise.all([
    commandsStore.loadFromBackend().catch(() => undefined),
    search.loadIndexStatus().catch(() => undefined),
    search.initialLoad().catch(() => undefined),
  ])
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
    if (unlistenIndexProgress) {
        unlistenIndexProgress()
        unlistenIndexProgress = null
    }
    if (unlistenWindowMonitor) {
        unlistenWindowMonitor()
        unlistenWindowMonitor = null
    }
    if (periodicRecommendTimer) {
        clearInterval(periodicRecommendTimer)
        periodicRecommendTimer = null
    }
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
        @contextmenu="handleContextMenu"
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
            :hovered-global-index="hoveredGlobalIndex"
            :start-index="groupStartIndices.get(group.id) ?? 0"
            :selected-indexes="search.activeSelectionGroupId === group.id ? search.selectedIndexes : undefined"
            @toggle-collapse="onToggleGroup"
            @select="onSelect"
            @open="onOpen"
            @hover="onHover"
            @contextmenu="handleContextMenu"
            @sort-change="(mode) => search.setGroupSortMode(group.id, mode)"
            @layout-transition-end="syncWindowHeight"
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

    <!-- 加载状态: frontend_ready 调用前显示不透明背景, 避免透明窗口闪烁 -->
    <LoadingState v-if="showLoading" message="正在初始化..." />

    <ContextMenu
      :visible="showContextMenu"
      :x="contextMenuX"
      :y="contextMenuY"
      :items="contextMenuItems"
      @close="closeContextMenu"
      @select="handleContextMenuAction"
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
