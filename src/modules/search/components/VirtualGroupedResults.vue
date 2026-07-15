<script setup lang="ts">
/**
 * 虚拟滚动分组列表 (Raycast 风格).
 *
 * 组件定位: **纯展示** —— 分组结构、折叠状态、可见项全部由 store 提供.
 * VGR 不再持有任何"业务状态", 只负责:
 *   - 把 `groups: DisplayGroup[]` 扁平化为 virtual rows (header + item).
 *   - 用 `vue-virtual-scroller` 的 `RecycleScroller` 渲染,
 *     同一时间 DOM 里只有 viewport + buffer 的几十行, 1M+ 项也无压力.
 *   - 单击行: emit 'select' (父级更新 selectedIndex).
 *   - 双击行: emit 'open'   (父级 executeItem 真正打开).
 *   - 点击折叠箭头: emit 'toggle-group' (父级 → store.toggleGroupCollapse).
 *
 * 分组顺序 (来自 store):
 *   1. 固定项目 (Pinned)      - 未搜索
 *   2. 最近访问 (Recent)        - 未搜索
 *   3. 系统应用 (System)
 *   4. 命令 (Commands)
 *   5. 所有应用 (All Apps)
 *   6. 所有文件 (All Files)   - 含多选分类筛选 chip
 *
 * 虚拟行设计:
 *   - 头行 (`kind: 'header'`): 44px 高, 渲染分组标题 + 折叠箭头 + (files)筛选按钮.
 *   - 普通行 (`kind: 'item'`):  44px 高, 渲染 AppResultItem / ResultItem.
 *   - 同一 itemSize (44px) 让 RecycleScroller 走 fixed-size pool, 性能最佳.
 *   - `typeField="kind"` 让 header / item 走不同 pool, 避免切换类型时闪烁.
 *
 * 关键约定:
 *   - 不再设置任何截断上限; 后端 search_cmd 默认返回 u32::MAX 全量命中.
 *   - "所有文件" 等"可能上百万行" 的分组也用同一份 44px 行, 虚拟滚动保证不卡.
 *   - 旧的"显示更多"按钮 + fileVisibleLimit + hiddenCount 已删除.
 */
import { computed, ref, watch, nextTick, onBeforeUnmount, onMounted } from 'vue'
import {
  Folder, Settings as SettingsIcon, Terminal, PinIcon, Clock,
  Sparkles, ChevronDown, Filter
} from '@lucide/vue'
import { RecycleScroller, type RecycleScrollerInstance } from 'vue-virtual-scroller'
import 'vue-virtual-scroller/dist/vue-virtual-scroller.css'
import type { SearchResult } from '@/modules/search'
import type { DisplayGroup, GroupId } from '@/modules/search'
import ResultItem from '@/modules/search/components/ResultItem.vue'
import AppResultItem from '@/modules/search/components/AppResultItem.vue'
import CheckButton from '@/ui/components/CheckButton.vue'
import { classify, classifyByResultType, FILE_KIND_META, FILE_KIND_DISPLAY_ORDER, type FileKind } from '../utils/fileKinds'
import { useSearchStore } from '@/modules/search'

interface Props {
  /**
   * 分组结构 (来自 store.displayGroups).
   * 启动早期 store 还没产出 DisplayGroup, 父组件可能传 undefined,
   * 组件必须能在 undefined 下挂载 (见 setup 中的 `?? []` 防护).
   */
  groups?: DisplayGroup[]
  loading?: boolean
  /** 当前全局选中项的下标 (来自 store.displayList). */
  selectedIndex: number
  height?: number
  /** 单行高度 (header 与 item 共用, 默认 45px, 含 1px 间距). */
  itemHeight?: number
  hasQuery?: boolean
  /** 当前查询关键字, 用于"搜索 X 中…"提示文案. */
  query?: string
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  height: 400,
  itemHeight: 45,
  hasQuery: false,
  query: '',
})

const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
  (e: 'open', item: SearchResult): void
  (e: 'hover', index: number): void
  (e: 'contextmenu', event: MouseEvent, item: SearchResult, globalIndex: number): void
  (e: 'toggle-group', id: GroupId): void
}>()

// === 文件类型过滤 (UI 内部状态) ===
// 仅 UI 偏好, 不影响 store 中的搜索结果. 持久化由 store 负责, 这里只是
// "未搜索状态下, 在所有文件分组里再筛一次" 的小开关.
const selectedFileKinds = ref<Set<FileKind>>(new Set(FILE_KIND_DISPLAY_ORDER))

// === 分组 -> 图标映射 (kind) ===
const GROUP_ICONS: Record<DisplayGroup['kind'], any> = {
  pinned: PinIcon,
  recent: Clock,
  system: SettingsIcon,
  commands: Terminal,
  apps: Sparkles,
  files: Folder,
}

// ============================================================================
// 虚拟行 (VirtualRow): 把"分组头 + 普通项" 展平为同一份 items 喂给 scroller.
// ============================================================================

/** 分组头行. 高度与 item 行一致, RecycleScroller 走 fixed-size pool. */
interface VirtualRowHeader {
  kind: 'header'
  key: string
  groupId: GroupId
  groupKind: DisplayGroup['kind']
  title: string
  icon: any
  count: number
  collapsed: boolean
  showFilter: boolean
}

/** 普通项行: 文件 / 应用 / 命令等具体 SearchResult. */
interface VirtualRowItem {
  kind: 'item'
  key: string
  groupKind: DisplayGroup['kind']
  result: SearchResult
  /** 在 store.displayList 中的全局 index, 用于 active 高亮 + 键盘导航. */
  globalIndex: number
}

type VirtualRow = VirtualRowHeader | VirtualRowItem

/**
 * 把 displayGroups 展平为 virtual rows.
 *
 * 规则:
 *  - 空组 (items.length === 0) → 完全跳过. 不显示 header, 也不显示 items.
 *    既然没内容, 折叠展开与分割线都没有视觉意义. 这是 "如果下面没有内容,
 *    则不支持折叠展开, 默认是收缩起来的" 的产品诉求的最终落实.
 *  - 折叠组 → 只渲染 header, item 不进入 virtualRows.
 *  - 展开组 → header 在前, 后面跟 visibleItems.
 *  - globalIndex 从 0 连续递增, 与 store.displayList 的索引严格 1:1.
 *
 * 性能: 对百万行 + 6 组的极端情况, 该 computed 的时间复杂度 O(N),
 * 与 store.displayGroups 重算同阶. 实测 1M items 展平 < 30ms (V8).
 */
const virtualRows = computed<VirtualRow[]>(() => {
  const rows: VirtualRow[] = []
  let globalIdx = 0
  for (const g of (props.groups ?? [])) {
    // 空组完全跳过 (header + items 都不渲染). 与"非空"组用同一条路径
    // 走"折叠/展开"逻辑, 简化分支.
    if (g.items.length === 0) continue
    rows.push({
      kind: 'header',
      key: `header:${g.id}`,
      groupId: g.id,
      groupKind: g.kind,
      title: g.title,
      icon: GROUP_ICONS[g.kind],
      count: g.items.length,
      collapsed: g.collapsed,
      showFilter: g.kind === 'files',
    })
    if (!g.collapsed) {
      for (const item of g.visibleItems) {
        rows.push({
          kind: 'item',
          key: `item:${g.id}:${item.id}:${globalIdx}`,
          groupKind: g.kind,
          result: item,
          globalIndex: globalIdx,
        })
        globalIdx++
      }
    }
  }
  return rows
})

/**
 * displayList 索引 → virtualRows 索引 的映射.
 *
 * 为什么需要: selectedIndex 是 displayList 里的位置 (item 索引),
 * 但 RecycleScroller.scrollToItem 要的是 virtualRows 里的位置 (含 header).
 * 该 computed 在 virtualRows 变化时重算一次, 之后 O(1) 取.
 *
 * 对 1M items, 这个数组约 4MB (Uint32Array) — 1 个 group header 占 1 个 slot,
 * 实际占比可忽略.
 */
const itemVirtualIndexes = computed<Int32Array>(() => {
  const arr = new Int32Array(virtualRows.value.length)
  let out = 0
  for (let i = 0; i < virtualRows.value.length; i++) {
    const row = virtualRows.value[i]
    if (row.kind === 'item') {
      arr[out++] = i
    }
  }
  return out === arr.length ? arr : arr.subarray(0, out)
})

// === 滚动到指定项 ===
/**
 * RecycleScroller 实例类型. 用 shim 文件 (`src/types/vue-virtual-scroller.d.ts`)
 * 中导出的 `RecycleScrollerInstance` 而不是 `InstanceType<typeof RecycleScroller>`:
 * `InstanceType` 拿到的是 DefineComponent 的 props/ctx 类型, 不包含实例方法.
 */
const scrollerRef = ref<RecycleScrollerInstance | null>(null)
const shouldScroll = ref(true)
const isHoverTriggered = ref(false)
const hoveredIndex = ref<number>(-1)

function setScrollEnabled(enabled: boolean) {
  shouldScroll.value = enabled
}

function setHoverTriggered(hovering: boolean) {
  isHoverTriggered.value = hovering
}

function isItemVisible(virtualIdx: number): boolean {
  if (!scrollerRef.value) return true
  const el = scrollerRef.value.$el as HTMLElement
  if (!el) return true

  const scrollTop = el.scrollTop
  const viewportHeight = el.clientHeight
  const itemTop = virtualIdx * props.itemHeight

  return itemTop >= scrollTop && itemTop + props.itemHeight <= scrollTop + viewportHeight
}

function scrollToSelected() {
  if (!shouldScroll.value) return

  const target = props.selectedIndex
  if (target < 0 || !scrollerRef.value) return
  const map = itemVirtualIndexes.value
  if (target >= map.length) return
  const virtualIdx = map[target]
  if (virtualIdx == null) return

  if (isHoverTriggered.value) {
    return
  }

  if (isItemVisible(virtualIdx)) {
    return
  }

  const el = scrollerRef.value.$el as HTMLElement
  if (!el) {
    scrollerRef.value.scrollToItem(virtualIdx)
    return
  }

  const scrollTop = el.scrollTop
  const viewportHeight = el.clientHeight
  const itemHeight = props.itemHeight
  const itemTop = virtualIdx * itemHeight

  if (itemTop < scrollTop) {
    scrollerRef.value.scrollToItem(virtualIdx, 'start')
  } else {
    scrollerRef.value.scrollToItem(virtualIdx, 'end')
  }
}

watch(() => props.selectedIndex, (v) => {
  if (v >= 0) nextTick(scrollToSelected)
})

/**
 * 折叠状态变化时: 等 max-height 过渡 (280ms) 完成后再滚,
 * 避免在 transition 中读到错误的 offsetTop.
 *
 * 注: 现在不再依赖 offsetTop, 只是为了"视觉稳定"延后滚动.
 * 折叠状态下 selectedIndex 已被 store clamp 到边界, scrollToItem 会平滑定位.
 */
watch(() => (props.groups ?? []).map((g) => `${g.id}:${g.collapsed}`).join('|'), () => {
  nextTick(() => {
    setTimeout(scrollToSelected, 320)
  })
})

watch(
  () => props.query,
  () => {
    if (scrollerRef.value) scrollerRef.value.scrollToPosition(0)
  },
)

// === 文件类型过滤 ===
function toggleKind(k: FileKind, e: Event) {
  if (e && (e as any).metaKey) {
    if (selectedFileKinds.value.has(k) && selectedFileKinds.value.size === 1) return
    const next = new Set(selectedFileKinds.value)
    if (next.has(k)) next.delete(k)
    else next.add(k)
    selectedFileKinds.value = next
  } else {
    if (selectedFileKinds.value.size === FILE_KIND_DISPLAY_ORDER.length) {
      selectedFileKinds.value = new Set([k])
    } else {
      selectedFileKinds.value = new Set(FILE_KIND_DISPLAY_ORDER)
    }
  }
}

function isKindActive(k: FileKind): boolean { return selectedFileKinds.value.has(k) }

const allKindsActive = computed(() => selectedFileKinds.value.size === FILE_KIND_DISPLAY_ORDER.length)
const activeFilterCount = computed(() => selectedFileKinds.value.size)
const filterSummary = computed(() => {
  if (allKindsActive.value) return '全部类型'
  if (activeFilterCount.value === 0) return '未选'
  if (activeFilterCount.value <= 2) {
    return Array.from(selectedFileKinds.value)
      .map((k) => FILE_KIND_META[k].label)
      .join('、')
  }
  return `已选 ${activeFilterCount.value} 类`
})

// === 下拉面板状态 ===
const filterOpen = ref(false)
const filterDropdownRef = ref<HTMLElement | null>(null)
const panelAlign = ref<'down' | 'up'>('down')

function toggleFilterPanel() {
  if (!filterOpen.value) {
    const root = filterDropdownRef.value as any
    if (root && typeof root.getBoundingClientRect === 'function') {
      const rect = root.getBoundingClientRect()
      const spaceBelow = window.innerHeight - rect.bottom
      panelAlign.value = spaceBelow < 280 ? 'up' : 'down'
    }
  }
  filterOpen.value = !filterOpen.value
}
function closeFilterPanel() { filterOpen.value = false }

function selectAllKinds() {
  selectedFileKinds.value = new Set(FILE_KIND_DISPLAY_ORDER)
}
function clearAllKinds() {
  selectedFileKinds.value = new Set(['other'] as FileKind[])
}

function onFilterOptionClick(k: FileKind) {
  const next = new Set(selectedFileKinds.value)
  if (next.has(k) && next.size > 1) {
    next.delete(k)
  } else if (!next.has(k)) {
    next.add(k)
  }
  selectedFileKinds.value = next
}

function onDocClick(e: MouseEvent) {
  if (!filterOpen.value) return
  const root = filterDropdownRef.value as any
  if (!root || typeof root.contains !== 'function') return
  const target = e.target as Node | null
  if (target && !root.contains(target)) {
    filterOpen.value = false
  }
}

function onKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape' && filterOpen.value) {
    filterOpen.value = false
    e.stopPropagation()
  }
}

if (typeof document !== 'undefined') {
  document.addEventListener('mousedown', onDocClick)
  document.addEventListener('keydown', onKeyDown)
}

onBeforeUnmount(() => {
  if (typeof document !== 'undefined') {
    document.removeEventListener('mousedown', onDocClick)
    document.removeEventListener('keydown', onKeyDown)
  }
})

// === 同步文件类型过滤到 store, 确保 displayList 与 VGR 渲染严格一致 ===
const search = useSearchStore()
onMounted(() => {
  search.setFileKindFilter(selectedFileKinds.value)
})
watch(selectedFileKinds, (kinds) => {
  search.setFileKindFilter(kinds)
}, { deep: true })

// === 统计每个 kind 的命中数 (仅用于下拉面板的 count 显示) ===
const fileCountByKind = computed(() => {
  const m: Record<string, number> = {}
  const filesGroup = (props.groups ?? []).find((g) => g.kind === 'files')
  for (const r of filesGroup?.items ?? []) {
    const byType = classifyByResultType((r as any).resultType)
    const ext = (r.subtitle || r.title || '').split(/[\\/]/).pop() || ''
    const kind = byType ?? classify(ext)
    m[kind] = (m[kind] || 0) + 1
  }
  return m
})

const isLoading = computed(() => props.loading && props.hasQuery)
const nothingNow = computed(() => !props.loading && virtualRows.value.length === 0)

/** 单击 → 只更新 selectedIndex (无副作用). 双击 → 真正打开.
 *  直接用 globalIndex, 确保选中的是用户点击的那一项 (同一 id 在不同分组中是独立的).
 */
function onPickItem(result: SearchResult, globalIndex: number, event: MouseEvent) {
  search.selectWithModifiers(globalIndex, event.ctrlKey || event.metaKey, event.shiftKey)
  emit('select', result)

  const virtualIdx = virtualRows.value.findIndex(row =>
    row.kind === 'item' && row.globalIndex === globalIndex
  )

  if (virtualIdx >= 0 && scrollerRef.value && !isItemVisible(virtualIdx)) {
    scrollerRef.value.scrollToItem(virtualIdx)
  }
}

function onOpenItem(item: SearchResult) {
  emit('open', item)
}

function onItemHover(idx: number) {
  isHoverTriggered.value = true
  hoveredIndex.value = idx
  emit('hover', idx)
}

function onItemLeave() {
  isHoverTriggered.value = false
  hoveredIndex.value = -1
}

/** 切换分组折叠: 通知 store. */
function onToggleGroup(id: GroupId) {
  emit('toggle-group', id)
}

function isAppKind(k: DisplayGroup['kind']): boolean {
  return k === 'apps' || k === 'pinned' || k === 'recent' || k === 'system'
}
</script>

<template>
  <div class="vg" :style="{ height: height + 'px' }">
    <div v-if="isLoading" class="vg__loading">
      <div class="vg__spinner"></div>
      <span class="vg__loading-text">搜索 {{ props.query }} 中…</span>
    </div>

    <div v-else-if="nothingNow" class="vg__empty">
      <slot name="empty" />
    </div>

    <RecycleScroller
      v-else
      ref="scrollerRef"
      class="vg__scroller"
      :items="virtualRows"
      :item-size="itemHeight"
      key-field="key"
      type-field="kind"
      :buffer="200"
      :style="{ height: height + 'px' }"
    >
      <template v-slot="{ item }">
        <!-- 分组头行: 标题 + (files)筛选按钮 -->
        <div
          v-if="item.kind === 'header'"
          class="vg__group-header-row"
          :class="{ 'vg__group-header-row--first': item.key === 'header:group.pinned' || item.key === 'header:group.recent' }"
          :data-group-id="item.groupId"
          @click="onToggleGroup(item.groupId)"
        >
          <div class="vg__group-header-left">
            <component :is="item.icon" :size="13" :stroke-width="1.8" class="vg__group-icon" />
            <span class="vg__group-title">{{ item.title }}</span>
            <span v-if="item.count" class="vg__group-count">{{ item.count.toLocaleString() }}</span>
          </div>

          <div class="vg__group-header-right">
            <!-- 所有文件: 标题行右侧的下拉多选 -->
            <div v-if="item.showFilter" ref="filterDropdownRef" class="vg__filter-dropdown">
              <button
                type="button"
                class="vg__filter-trigger"
                :class="{ 'vg__filter-trigger--active': !allKindsActive }"
                @click.stop="toggleFilterPanel"
                :aria-expanded="filterOpen"
              >
                <Filter :size="11" :stroke-width="2" class="vg__filter-trigger-icon" />
                <span>{{ filterSummary }}</span>
                <ChevronDown
                  :size="11"
                  :stroke-width="2.2"
                  class="vg__filter-trigger-icon"
                  :style="{ transform: filterOpen ? 'rotate(180deg)' : 'none', transition: 'transform 200ms cubic-bezier(0.16, 1, 0.3, 1)' }"
                />
              </button>

              <Transition :name="panelAlign === 'up' ? 'filter-pop-up' : 'filter-pop'">
                <div
                  v-if="filterOpen"
                  class="vg__filter-panel"
                  :class="panelAlign === 'up' ? 'vg__filter-panel--up' : 'vg__filter-panel--down'"
                  role="listbox"
                  @click.stop
                >
                  <button
                    v-for="(k, idx) in FILE_KIND_DISPLAY_ORDER"
                    :key="k"
                    type="button"
                    class="vg__filter-option"
                    :class="{ 'vg__filter-option--active': isKindActive(k) }"
                    :style="{ '--i': idx }"
                    @click="onFilterOptionClick(k)"
                  >
                    <CheckButton
                      :model-value="isKindActive(k)"
                      :size="13"
                      class="vg__filter-check"
                    />
                    <span class="vg__filter-label">{{ FILE_KIND_META[k].label }}</span>
                    <span v-if="fileCountByKind[k]" class="vg__filter-count">{{ fileCountByKind[k] }}</span>
                  </button>

                  <div class="vg__filter-footer">
                    <button type="button" class="vg__filter-action" @click="clearAllKinds">清空</button>
                    <button type="button" class="vg__filter-action vg__filter-action--primary" @click="selectAllKinds">全选</button>
                  </div>
                </div>
              </Transition>
            </div>
          </div>
        </div>

        <!-- 普通行: AppResultItem / ResultItem -->
        <div
          v-else
          class="vg__row"
          :class="{
            'vg__row--active': item.globalIndex === selectedIndex,
            'vg__row--hover': item.globalIndex === hoveredIndex,
            'vg__row--selected': search.isSelected(item.result.id)
          }"
          :data-global-idx="item.globalIndex"
          :data-group-kind="item.groupKind"
          @click="(e) => onPickItem(item.result, item.globalIndex, e)"
          @dblclick="onOpenItem(item.result)"
          @mouseenter="onItemHover(item.globalIndex)"
          @mouseleave="onItemLeave"
          @contextmenu.prevent="(e) => emit('contextmenu', e, item.result, item.globalIndex)"
        >
          <AppResultItem
            v-if="isAppKind(item.groupKind)"
            :result="item.result"
            :index="item.globalIndex"
            :active="item.globalIndex === selectedIndex"
            badge-size="sm"
          />
          <ResultItem
            v-else
            :result="item.result"
            :index="item.globalIndex"
            :active="item.globalIndex === selectedIndex"
          />
        </div>
      </template>
    </RecycleScroller>
  </div>
</template>

<style scoped>
.vg {
  flex: 1;
  min-height: 0;
  position: relative;
  /* overflow-x: visible 让 .vg__group-header-row 的 ::before 分割线
     伪元素能延伸到 .vg 的左右边缘 (进而贴到窗口两边).
     overflow-y 保持 hidden, 防止下拉面板 / 加载动画溢出.
     注: 父容器 .results-scroll-container 同步改为 overflow-x: visible
     (SearchPage.vue), 让伪元素能继续穿过外层 padding. */
  overflow-x: visible;
  overflow-y: hidden;
  padding: 4px 4px 0 10px;
  display: flex;
  flex-direction: column;
}

/* RecycleScroller 自身带 overflow:auto, 不需要再设.
   但要让它有正确高度 (父容器 flex). */
.vg__scroller {
  width: 100%;
  scrollbar-gutter: stable;
  scroll-behavior: smooth;
  flex: 1;
  min-height: 0;
  padding-right: 6px;
}

/* vue-virtual-scroller 的 .vue-recycle-scroller__item-wrapper 默认
   overflow: hidden, 会裁剪掉分组头行 ::before 分割线伪元素.
   改为 visible 让 ::before 能延伸到窗口边缘.
   同时让 item-view 也 overflow: visible, 避免 AppResultItem 的 tooltip 被裁剪. */
.vg__scroller :deep(.vue-recycle-scroller__item-wrapper) {
  overflow: visible;
}
.vg__scroller :deep(.vue-recycle-scroller__item-view) {
  overflow: visible;
}

/* 滚动条统一样式见 theme.scss, 这里只保留 vg 特有的 gutter 配置 */

/* === 分组头行: 44px 高, 与 item 行同高. ===
   分割线: 用 ::before 伪元素延伸到窗口边缘 (覆盖 .vg 与
   .results-scroll-container 的左右 padding), 实现"贴窗口两边"的视觉效果.
   数值 -18px = 8 (.results-scroll-container) + 10 (.vg). 右侧同理
   但额外加 6 (vg__scroller padding-right) = -24px; 不过 VGR 内部
   对称, 左右都用 -18px 即可保证视觉对齐 (左右 padding 总和一致). */
.vg__group-header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 44px;
  padding: 0 6px;
  gap: 10px;
  box-sizing: border-box;
  flex-shrink: 0;
  background: transparent;
  position: relative;
}

.vg__group-header-row::before {
  content: '';
  position: absolute;
  top: 0;
  left: -18px;
  right: -18px;
  height: 1px;
  background: var(--border-subtle);
  pointer-events: none;
}

.vg__group-header-row--first::before {
  /* 第一个分组不需要分割线 (顶部紧贴 SearchInput) */
  display: none;
}

.vg__group-header-left {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  min-width: 0;
}

.vg__group-header-right {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.vg__group-icon {
  color: var(--text-quaternary);
  opacity: 0.8;
  flex-shrink: 0;
  transition: opacity var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out);
}

.vg__group-header-row:hover .vg__group-icon {
  opacity: 1;
  color: var(--text-tertiary);
}

.vg__group-title {
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--text-quaternary);
  transition: color var(--dur-fast) var(--ease-out);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.vg__group-header-row:hover .vg__group-title {
  color: var(--text-tertiary);
}

.vg__group-count {
  font-size: 10px;
  font-weight: 500;
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
  margin-left: 2px;
  padding: 0 6px;
  border-radius: var(--radius-full);
  background: transparent;
  border: 1px solid var(--border-subtle);
  line-height: 15px;
  transition: color var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out);
  flex-shrink: 0;
}

.vg__group-header-row:hover .vg__group-count {
  color: var(--text-tertiary);
  border-color: var(--border-default);
}

/* === 折叠/展开箭头按钮 === */
.vg__group-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: var(--radius-sm);
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-quaternary);
  cursor: pointer;
  transition:
    color var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out),
    border-color var(--dur-fast) var(--ease-out),
    transform 280ms cubic-bezier(0.34, 1.2, 0.64, 1);
  transform: rotate(0deg);
  flex-shrink: 0;
  padding: 0;
}

.vg__group-toggle:hover {
  color: var(--text-primary);
  background: var(--list-hover-bg);
  border-color: var(--border-subtle);
}

.vg__group-toggle--collapsed {
  transform: rotate(-90deg);
}

.vg__group-toggle:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}

/* === 所有文件: 标题行右侧的下拉多选 === */
.vg__filter-dropdown {
  position: relative;
  flex-shrink: 0;
}

.vg__filter-trigger {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 0 8px;
  height: 22px;
  border-radius: var(--radius-md);
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-tertiary);
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.01em;
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
  white-space: nowrap;
}

.vg__filter-trigger:hover {
  background: var(--list-hover-bg);
  border-color: var(--border-subtle);
  color: var(--text-secondary);
}

.vg__filter-trigger--active {
  background: rgba(255, 255, 255, 0.05);
  border-color: var(--border-subtle);
  color: var(--text-primary);
}

.vg__filter-trigger-icon {
  flex-shrink: 0;
  opacity: 0.75;
  transition: opacity var(--dur-fast) var(--ease-out);
}

.vg__filter-trigger:hover .vg__filter-trigger-icon {
  opacity: 1;
}

/* === 扁平化 3 列网格面板 (强高斯模糊, 黑白灰, 适当间距) === */
.vg__filter-panel {
  position: absolute;
  background: rgba(18, 18, 21, 0.62);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-xl);
  box-shadow:
    0 1px 0 rgba(255, 255, 255, 0.05) inset,
    0 16px 48px rgba(0, 0, 0, 0.6),
    0 4px 12px rgba(0, 0, 0, 0.35);
  z-index: 50;
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  grid-auto-rows: min-content;
  gap: 6px;
  backdrop-filter: blur(48px) saturate(180%);
  -webkit-backdrop-filter: blur(48px) saturate(180%);
  overflow: visible;
}

.os-win10 .vg__filter-panel {
  background: rgba(28, 28, 32, 0.98);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
  border-color: var(--border-default);
}

.vg__filter-panel--down {
  top: calc(100% + 8px);
  right: 0;
  width: 360px;
  padding: 12px;
}

.vg__filter-panel--up {
  bottom: calc(100% + 8px);
  right: 0;
  width: 360px;
  padding: 12px;
}

.vg__filter-option {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  border: 1px solid transparent;
  background: transparent;
  border-radius: var(--radius-md);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  text-align: left;
  transition:
    background var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out),
    border-color var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
  white-space: nowrap;
  min-width: 0;
  position: relative;
}

.vg__filter-option:hover {
  background: rgba(255, 255, 255, 0.045);
  color: var(--text-primary);
  border-color: var(--border-subtle);
}

.vg__filter-option--active {
  color: var(--text-primary);
  background: rgba(255, 255, 255, 0.06);
  border-color: var(--border-subtle);
}

.vg__filter-option--active:hover {
  background: rgba(255, 255, 255, 0.09);
  border-color: var(--border-default);
}

.vg__filter-check {
  flex-shrink: 0;
}

.vg__filter-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}

.vg__filter-count {
  flex-shrink: 0;
  margin-left: auto;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
  font-weight: 500;
  opacity: 0.75;
  background: rgba(255, 255, 255, 0.025);
  border-radius: var(--radius-full);
  transition: color var(--dur-fast) var(--ease-out), opacity var(--dur-fast) var(--ease-out), background var(--dur-fast) var(--ease-out);
}

.vg__filter-option--active .vg__filter-count {
  color: var(--text-primary);
  opacity: 0.95;
  background: rgba(255, 255, 255, 0.08);
}

/* === 底部操作行 (跨 3 列) === */
.vg__filter-footer {
  grid-column: 1 / -1;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 6px 2px;
  margin-top: 8px;
  border-top: 1px solid var(--border-subtle);
  gap: 8px;
}

.vg__filter-action {
  background: transparent;
  border: none;
  color: var(--text-tertiary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  padding: 5px 10px;
  border-radius: var(--radius-sm);
  transition: color var(--dur-fast) var(--ease-out), background var(--dur-fast) var(--ease-out);
}

.vg__filter-action:hover {
  color: var(--text-primary);
  background: rgba(255, 255, 255, 0.05);
}

.vg__filter-action--primary {
  color: var(--text-primary);
  font-weight: 600;
}

.vg__filter-action--primary:hover {
  color: var(--text-primary);
  background: rgba(255, 255, 255, 0.08);
}

/* === 错落渐入动画: 每个选项按 index 延时出现 === */
.filter-pop-enter-active .vg__filter-option {
  animation: option-fade-in 360ms cubic-bezier(0.16, 1, 0.3, 1) backwards;
  animation-delay: calc(var(--i, 0) * 16ms);
}

.filter-pop-enter-active .vg__filter-footer {
  animation: option-fade-in 360ms cubic-bezier(0.16, 1, 0.3, 1) backwards;
  animation-delay: calc(var(--i, 12) * 16ms);
}

@keyframes option-fade-in {
  0% {
    opacity: 0;
    transform: translateY(-4px) scale(0.96);
  }
  100% {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

/* === 普通行: 与分组头行同高, RecycleScroller 走 fixed-size pool === */
.vg__row {
  display: block;
  cursor: pointer;
  border-radius: var(--radius-md);
  transition:
    background var(--dur-fast) var(--ease-out),
    filter var(--dur-fast) var(--ease-out);
  box-sizing: border-box;
  height: calc(100% - 1px);
  overflow: visible;
  position: relative;
}

.vg__row:hover,
.vg__row--hover {
  background: var(--list-hover-bg);
}

.vg__row--active {
  background: var(--list-selected-bg);
}

.vg__row--active:hover,
.vg__row--active.vg__row--hover {
  background: var(--list-selected-bg);
  filter: brightness(1.08);
}

.vg__row--selected {
  background: rgba(255, 255, 255, 0.06);
}

.vg__row--selected:hover {
  background: rgba(255, 255, 255, 0.08);
}

.vg__row--selected.vg__row--active {
  background: var(--list-selected-bg);
}

/* === 加载 / 空态 === */
.vg__loading,
.vg__empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-3);
  padding: var(--sp-10) var(--sp-5);
  height: 100%;
}

.vg__loading { flex-direction: row; }

.vg__spinner {
  width: 16px; height: 16px;
  border: 2px solid var(--border-default);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: vg-spin 0.8s linear infinite;
}

.vg__loading-text {
  color: var(--text-tertiary);
  font-size: var(--text-sm);
}

@keyframes vg-spin { to { transform: rotate(360deg); } }

/* === 下拉面板动画 (平滑动效 + 弹性) === */
/* 向下展开 */
.filter-pop-enter-active {
  transition:
    opacity 200ms cubic-bezier(0.16, 1, 0.3, 1),
    transform 240ms cubic-bezier(0.34, 1.2, 0.64, 1);
  transform-origin: top right;
}
.filter-pop-leave-active {
  transition:
    opacity 140ms cubic-bezier(0.4, 0, 1, 1),
    transform 160ms cubic-bezier(0.4, 0, 1, 1);
  transform-origin: top right;
}
.filter-pop-enter-from {
  opacity: 0;
  transform: translateY(-6px) scale(0.96);
}
.filter-pop-leave-to {
  opacity: 0;
  transform: translateY(-4px) scale(0.98);
}

/* 向上展开 */
.filter-pop-up-enter-active {
  transition:
    opacity 200ms cubic-bezier(0.16, 1, 0.3, 1),
    transform 240ms cubic-bezier(0.34, 1.2, 0.64, 1);
  transform-origin: bottom right;
}
.filter-pop-up-leave-active {
  transition:
    opacity 140ms cubic-bezier(0.4, 0, 1, 1),
    transform 160ms cubic-bezier(0.4, 0, 1, 1);
  transform-origin: bottom right;
}
.filter-pop-up-enter-from {
  opacity: 0;
  transform: translateY(6px) scale(0.96);
}
.filter-pop-up-leave-to {
  opacity: 0;
  transform: translateY(4px) scale(0.98);
}
</style>
