<script setup lang="ts">
/**
 * 简洁分组列表 (Raycast 风格).
 *
 * 不管是搜索状态还是未搜索状态, 都以"分组行"的形式展示内容.
 * 分组之间用细分隔线区分, 不使用卡片背景/边框/阴影.
 * 标题是简洁的小号大写字母 + 数字, 类似 Raycast 列表分组.
 *
 * 分组顺序:
 *   1. 固定项目 (Pinned)      - 未搜索
 *   2. 最近访问 (Recent)        - 未搜索
 *   3. 系统应用 (System)      - 全部/应用
 *   4. 命令 (Commands)        - 全部/命令
 *   5. 所有应用 (All Apps)
 *   6. 所有文件 (All Files)   - 含多选分类筛选 chip
 */
import { computed, ref, watch, nextTick, onBeforeUnmount } from 'vue'
import {
  File as FileIcon, FileCode, FileImage, FileText,
  Folder, Settings as SettingsIcon, Terminal, PinIcon, Clock, Cpu,
  Sparkles, FileVideo, FileAudio, FileArchive,
  FileBraces, FileSpreadsheet, Presentation,
  ChevronDown, Filter
} from '@lucide/vue'
import type { SearchResult } from '@/types/search'
import ResultItem from '@/components/common/ResultItem.vue'
import AppResultItem from '@/components/search/AppResultItem.vue'
import CheckButton from '@/components/common/CheckButton.vue'
import { classify, classifyByResultType, FILE_KIND_META, FILE_KIND_DISPLAY_ORDER, type FileKind } from '@/utils/fileKinds'

interface Props {
  results: SearchResult[]
  loading?: boolean
  selectedIndex: number
  height?: number
  itemHeight?: number
  pinned?: SearchResult[]
  recent?: SearchResult[]
  hasQuery?: boolean
  /** 单个分组最多显示多少项, 避免分组过高撑爆窗口. 默认 6. */
  maxPerGroup?: number
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  height: 400,
  itemHeight: 44,
  pinned: () => [],
  recent: () => [],
  hasQuery: false,
  maxPerGroup: 6,
})

/**
 * 文件分组在"未搜索"状态下的最大可见数.
 * 后端 ALL_FILES_EMPTY_QUERY_CAP=500 时, 一次渲染 500 个 DOM 节点会
 * 让滚动卡顿. 默认 80 个 + 用户点击 "显示更多" 每次展开 50 个,
 * 既保留"全部可访问"语义又避免性能塌方.
 */
const FILE_VISIBLE_INITIAL = 80
const FILE_VISIBLE_STEP = 50
const fileVisibleLimit = ref(FILE_VISIBLE_INITIAL)

const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
  (e: 'hover', index: number): void
  (e: 'contextmenu', event: MouseEvent, item: SearchResult): void
}>()

// 默认全选 (搜索时) 或全选 (未搜索时)
const selectedFileKinds = ref<Set<FileKind>>(new Set(FILE_KIND_DISPLAY_ORDER))

interface Group {
  id: string
  title: string
  icon: any
  kinds: FileKind[] | null
  items: SearchResult[]
}

const FILE_GROUP_ID = 'group.files'
const APPS_GROUP_ID = 'group.apps'
const COMMANDS_GROUP_ID = 'group.commands'
const PINNED_GROUP_ID = 'group.pinned'
const SYSTEM_GROUP_ID = 'group.system'
const RECENT_GROUP_ID = 'group.recent'

function isFile(r: SearchResult): boolean { return r.category === 'files' }
function isApp(r: SearchResult): boolean { return r.category === 'apps' }
function isCommand(r: SearchResult): boolean { return r.category === 'commands' }
function isSystemApp(r: SearchResult): boolean {
  return isApp(r) && (r as any).resultType === 'system-app'
}

const groups = computed<Group[]>(() => {
  const out: Group[] = []
  const cap = props.maxPerGroup

  // 1) 固定项目 - 未搜索状态才有
  const pinned = (props.pinned || []).slice(0, cap)
  if (pinned.length && !props.hasQuery) {
    out.push({ id: PINNED_GROUP_ID, title: '固定项目', icon: PinIcon, kinds: null, items: pinned })
  }

  // 2) 最近访问 - 未搜索状态才有
  const recent = (props.recent || []).slice(0, cap)
  if (recent.length && !props.hasQuery) {
    out.push({ id: RECENT_GROUP_ID, title: '最近访问', icon: Clock, kinds: null, items: recent })
  }

  // 3) 系统应用
  const sysApps: SearchResult[] = []
  const userApps: SearchResult[] = []
  for (const r of props.results) {
    if (isSystemApp(r)) sysApps.push(r)
    else if (isApp(r)) userApps.push(r)
  }
  if (sysApps.length) {
    sysApps.sort((a, b) => a.title.localeCompare(b.title))
    // 空查询时: 系统应用也全部展开; 有查询时仍 cap.
    const sysItems = props.hasQuery ? sysApps.slice(0, cap) : sysApps
    out.push({ id: SYSTEM_GROUP_ID, title: '系统应用', icon: SettingsIcon, kinds: null, items: sysItems })
  }

  // 4) 命令
  const commands: SearchResult[] = []
  for (const r of props.results) if (isCommand(r)) commands.push(r)
  if (commands.length) {
    out.push({ id: COMMANDS_GROUP_ID, title: '命令', icon: Terminal, kinds: null, items: commands.slice(0, cap) })
  }

  // 5) 所有应用
  if (userApps.length) {
    // 空查询时: 全部展开 (不再 cap=6 截断), 让首屏能看到所有应用.
    // 有查询时: 仍 cap 保持原有紧凑节奏.
    const appsItems = props.hasQuery ? userApps.slice(0, cap) : userApps
    out.push({ id: APPS_GROUP_ID, title: '所有应用', icon: Sparkles, kinds: null, items: appsItems })
  }

  // 6) 所有文件 - 应用文件分类筛选
  const filesAll: SearchResult[] = []
  for (const r of props.results) {
    if (!isFile(r)) continue
    const byType = classifyByResultType((r as any).resultType)
    const ext = (r.subtitle || r.title || '').split(/[\\/]/).pop() || ''
    const kind = byType ?? classify(ext)
    if (selectedFileKinds.value.has(kind)) filesAll.push(r)
  }
  if (filesAll.length) {
    // 关键: 空查询时文件分组**默认展开** (受 fileVisibleLimit 限制),
    // 保证即使没有输入关键字搜索也能展示所有可搜索/索引的文件.
    // 有查询时仍 cap 保持紧凑节奏, 由窗口滚动承载更多结果.
    if (!props.hasQuery) {
      // 切换 query / 筛选 / 索引就绪时, 把可见数重置回 initial,
      // 避免"展开更多"后的状态在切换条件后造成困惑.
      fileVisibleLimit.value = FILE_VISIBLE_INITIAL
    }
    const sliceEnd = props.hasQuery
      ? Math.min(filesAll.length, cap)
      : Math.min(filesAll.length, fileVisibleLimit.value)
    const filesItems = filesAll.slice(0, sliceEnd)
    out.push({
      id: FILE_GROUP_ID,
      title: '所有文件',
      icon: Folder,
      kinds: FILE_KIND_DISPLAY_ORDER,
      items: filesItems,
      // 让分组头部拿到"是否有更多未展示"的元信息, 渲染"显示更多"按钮
      hiddenCount: props.hasQuery ? 0 : Math.max(0, filesAll.length - sliceEnd),
    } as any)
  }

  return out
})

/** 每个分组的全局起始 offset (用于键盘上下方向键 / 高亮联动) */
const itemOffsetOfGroup = computed(() => {
  const map: Record<string, number> = {}
  let off = 0
  for (const g of groups.value) {
    map[g.id] = off
    off += g.items.length
  }
  return map
})

/** 全局扁平化列表, 给键盘方向键使用 */
const flatItems = computed(() => groups.value.flatMap((g) => g.items))

// === 滚动到指定项 (用于键盘上下方向键) ===
const scrollerRef = ref<HTMLElement | null>(null)

function scrollToGlobalIndex(idx: number) {
  if (!scrollerRef.value) return
  const el = scrollerRef.value.querySelector<HTMLElement>(`[data-global-idx="${idx}"]`)
  if (el) {
    const cTop = scrollerRef.value.scrollTop
    const cBot = cTop + scrollerRef.value.clientHeight
    const eTop = el.offsetTop
    const eBot = eTop + props.itemHeight
    if (eTop < cTop) scrollerRef.value.scrollTop = eTop - 4
    else if (eBot > cBot) scrollerRef.value.scrollTop = eBot - scrollerRef.value.clientHeight + 4
  }
}

watch(() => props.selectedIndex, (v) => {
  if (v >= 0) nextTick(() => scrollToGlobalIndex(v))
})

watch(() => props.results, () => {
  if (scrollerRef.value) scrollerRef.value.scrollTop = 0
})

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
    // 智能判断: 下方空间不足时改为向上展开
    const root = filterDropdownRef.value as any
    if (root && typeof root.getBoundingClientRect === 'function') {
      const rect = root.getBoundingClientRect()
      const spaceBelow = window.innerHeight - rect.bottom
      // 面板大约高度 ~270px, 留 8px 间距
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
  // 至少保留一个, 避免空筛选. 兜底保留 "其他" 类型.
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
  // 防御: ref 可能为 null/undefined 或 v-if 切换过程中被解绑
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

// === 统计每个 kind 的命中数 (仅用于下拉面板的 count 显示) ===
const fileCountByKind = computed(() => {
  const m: Record<string, number> = {}
  for (const r of props.results) {
    if (!isFile(r)) continue
    const byType = classifyByResultType((r as any).resultType)
    const ext = (r.subtitle || r.title || '').split(/[\\/]/).pop() || ''
    const kind = byType ?? classify(ext)
    m[kind] = (m[kind] || 0) + 1
  }
  return m
})

const isLoading = computed(() => props.loading && flatItems.value.length === 0)
const nothingNow = computed(() => !props.loading && flatItems.value.length === 0)

function onPickItem(item: SearchResult) { emit('select', item) }
function onItemHover(idx: number) { emit('hover', idx) }

/** "显示更多": 每次展开 FILE_VISIBLE_STEP 个文件. */
function showMoreFiles() {
  fileVisibleLimit.value = Math.min(
    fileVisibleLimit.value + FILE_VISIBLE_STEP,
    // 硬上限 1000, 防止无限滚动按钮
    1000
  )
}
</script>

<template>
  <div class="vg" :style="{ height: height + 'px' }">
    <div v-if="isLoading" class="vg__loading">
      <div class="vg__spinner"></div>
      <span class="vg__loading-text">搜索中…</span>
    </div>

    <div v-else-if="nothingNow" class="vg__empty">
      <slot name="empty" />
    </div>

    <div v-else ref="scrollerRef" class="vg__scroller">
      <div class="vg__list">
        <template v-for="(g, gi) in groups" :key="g.id">
          <!-- 分组 (无卡片背景, 仅细分隔线) -->
          <section
            class="vg__group"
            :class="{ 'vg__group--first': gi === 0, 'vg__group--files': g.id === FILE_GROUP_ID }"
          >
            <!-- 简洁标题 (Raycast 风格, 进一步加大字号 + 加大图标 + 浅色) -->
            <div class="vg__group-header">
              <div class="vg__group-header-left">
                <component :is="g.icon" :size="15" :stroke-width="1.7" class="vg__group-icon" />
                <span class="vg__group-title">{{ g.title }}</span>
                <span v-if="g.items.length" class="vg__group-count">{{ g.items.length }}</span>
              </div>
              <!-- 所有文件: 标题行右侧的下拉多选 -->
              <div v-if="g.id === FILE_GROUP_ID" ref="filterDropdownRef" class="vg__filter-dropdown">
                <button
                  type="button"
                  class="vg__filter-trigger"
                  :class="{ 'vg__filter-trigger--active': !allKindsActive }"
                  @click="toggleFilterPanel"
                  :aria-expanded="filterOpen"
                >
                  <Filter :size="13" :stroke-width="2" class="vg__filter-trigger-icon" />
                  <span>{{ filterSummary }}</span>
                  <ChevronDown :size="13" :stroke-width="2.2" class="vg__filter-trigger-icon" :style="{ transform: filterOpen ? 'rotate(180deg)' : 'none', transition: 'transform 200ms cubic-bezier(0.16, 1, 0.3, 1)' }" />
                </button>

                <Transition :name="panelAlign === 'up' ? 'filter-pop-up' : 'filter-pop'">
                  <div
                    v-if="filterOpen"
                    class="vg__filter-panel"
                    :class="panelAlign === 'up' ? 'vg__filter-panel--up' : 'vg__filter-panel--down'"
                    role="listbox"
                  >
                    <button
                      v-for="(k, idx) in g.kinds || []"
                      :key="k"
                      type="button"
                      class="vg__filter-option"
                      :class="{ 'vg__filter-option--active': isKindActive(k) }"
                      :style="{ '--i': idx }"
                      @click="onFilterOptionClick(k)"
                    >
                      <CheckButton
                        :model-value="isKindActive(k)"
                        :size="15"
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

            <!-- 项 (直接渲染, 无 wrapper) -->
            <div class="vg__rows">
              <div
                v-for="(it, localIdx) in g.items"
                :key="it.id + ':' + localIdx"
                :data-global-idx="itemOffsetOfGroup[g.id] + localIdx"
                class="vg__row"
                :class="{ 'vg__row--active': (itemOffsetOfGroup[g.id] + localIdx) === selectedIndex }"
                @click="onPickItem(it)"
                @mouseover="onItemHover(itemOffsetOfGroup[g.id] + localIdx)"
                @contextmenu.prevent="(e) => emit('contextmenu', e, it)"
              >
                <AppResultItem
                  v-if="isApp(it)"
                  :result="it"
                  :index="itemOffsetOfGroup[g.id] + localIdx"
                  :active="(itemOffsetOfGroup[g.id] + localIdx) === selectedIndex"
                  @select="onPickItem"
                  @mouseover="onItemHover(itemOffsetOfGroup[g.id] + localIdx)"
                  @contextmenu="(e) => emit('contextmenu', e, it)"
                />
                <ResultItem
                  v-else
                  :result="it"
                  :index="itemOffsetOfGroup[g.id] + localIdx"
                  :active="(itemOffsetOfGroup[g.id] + localIdx) === selectedIndex"
                  @select="onPickItem"
                />
              </div>

              <!-- "所有文件" 分组: 还有更多未展示时, 渲染展开按钮 -->
              <button
                v-if="g.id === FILE_GROUP_ID && (g as any).hiddenCount > 0"
                type="button"
                class="vg__show-more"
                @click="showMoreFiles"
              >
                <ChevronDown :size="13" :stroke-width="2" />
                <span>显示更多 (+{{ (g as any).hiddenCount }})</span>
              </button>
            </div>
          </section>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.vg {
  flex: 1;
  min-height: 0;
  position: relative;
  overflow: hidden;
  padding: 4px 10px 0 10px;
}

.vg__scroller {
  height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
  scrollbar-gutter: stable;
  scroll-behavior: smooth;
}

.vg__list {
  display: flex;
  flex-direction: column;
}

/* === 分组: 无背景/无边框/无圆角, 仅与上下方用 1px 细线分隔 === */
.vg__group {
  display: flex;
  flex-direction: column;
  background: transparent;
  border-top: 1px solid var(--border-subtle);
  padding: 0;
  margin: 0;
}

.vg__group--first {
  border-top: none;
}

/* === 标题行 (加大字号 + 加大图标 + 浅色文字) === */
.vg__group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  height: 38px;
  padding: 18px 6px 10px 6px;
  flex-shrink: 0;
}

.vg__group-header-left {
  display: flex;
  align-items: center;
  gap: 9px;
  flex-shrink: 0;
  min-width: 0;
}

.vg__group-icon {
  color: var(--text-quaternary);
  opacity: 0.8;
  flex-shrink: 0;
  transition: opacity var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out);
}

.vg__group-header:hover .vg__group-icon {
  opacity: 1;
  color: var(--text-tertiary);
}

.vg__group-title {
  font-size: 15px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-quaternary);
  transition: color var(--dur-fast) var(--ease-out);
}

.vg__group-header:hover .vg__group-title {
  color: var(--text-tertiary);
}

.vg__group-count {
  font-size: 10.5px;
  font-weight: 500;
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
  margin-left: 4px;
  padding: 0 7px;
  border-radius: var(--radius-full);
  background: transparent;
  border: 1px solid var(--border-subtle);
  line-height: 17px;
  transition: color var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out);
}

.vg__group-header:hover .vg__group-count {
  color: var(--text-tertiary);
  border-color: var(--border-default);
}

/* === 所有文件: 标题行右侧的下拉多选 === */
.vg__filter-dropdown {
  position: relative;
  flex-shrink: 0;
}

.vg__filter-trigger {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 0 11px;
  height: 26px;
  border-radius: var(--radius-md);
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-tertiary);
  font-size: 12px;
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
  background: rgba(20, 20, 24, 0.75);
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
  backdrop-filter: blur(40px) saturate(180%);
  -webkit-backdrop-filter: blur(40px) saturate(180%);
  overflow: visible;
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

/* === 项容器 === */
.vg__rows {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.vg__row {
  display: block;
  cursor: pointer;
  border-radius: var(--radius-md);
  /* 长列表性能优化: 浏览器自动跳过视口外元素的渲染, 大幅减少 500+
   * 个文件时的 paint / layout 开销. 50px 是单行预估高度的 1.1x 余量. */
  content-visibility: auto;
  contain-intrinsic-size: auto 44px;
}

.vg__row--active {
  background: var(--list-selected-bg);
}

/* "显示更多" 按钮 (文件分组尾部, 当 hiddenCount > 0 时渲染) */
.vg__show-more {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  margin: 4px 8px 8px;
  padding: 7px 10px;
  border: 1px dashed var(--border-subtle);
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-tertiary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition:
    color var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out),
    border-color var(--dur-fast) var(--ease-out);
}

.vg__show-more:hover {
  color: var(--text-primary);
  background: var(--list-hover-bg);
  border-color: var(--border-default);
  border-style: solid;
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
  transform: translateY(-8px) scale(0.92);
}
.filter-pop-leave-to {
  opacity: 0;
  transform: translateY(-4px) scale(0.96);
}
.filter-pop-enter-to,
.filter-pop-leave-from {
  opacity: 1;
  transform: translateY(0) scale(1);
}

/* 向上展开 (用于触发器靠近视口底部时) */
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
  transform: translateY(8px) scale(0.92);
}
.filter-pop-up-leave-to {
  opacity: 0;
  transform: translateY(4px) scale(0.96);
}
.filter-pop-up-enter-to,
.filter-pop-up-leave-from {
  opacity: 1;
  transform: translateY(0) scale(1);
}

.vg__scroller::-webkit-scrollbar { width: 6px; }
.vg__scroller::-webkit-scrollbar-thumb {
  background: var(--border-default);
  border-radius: 999px;
}
.vg__scroller::-webkit-scrollbar-thumb:hover {
  background: var(--border-hover);
}
.vg__scroller::-webkit-scrollbar-track { background: transparent; }
</style>
