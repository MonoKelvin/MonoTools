<script setup lang="ts">
/**
 * 简洁分组列表 (Raycast 风格).
 *
 * 组件定位: **纯展示** —— 分组结构、折叠状态、可见项全部由 store 提供.
 * VGR 不再持有任何"业务状态", 只负责:
 *   - 接收 `groups: DisplayGroup[]` 渲染每个 section.
 *   - 单击行: emit 'select' (父级更新 selectedIndex).
 *   - 双击行: emit 'open'   (父级 executeItem 真正打开).
 *   - 点击折叠箭头: emit 'toggle-group' (父级 → store.toggleGroupCollapse).
 *   - 点击 "显示更多": emit 'show-more-files'.
 *
 * 分组顺序 (来自 store):
 *   1. 固定项目 (Pinned)      - 未搜索
 *   2. 最近访问 (Recent)        - 未搜索
 *   3. 系统应用 (System)
 *   4. 命令 (Commands)
 *   5. 所有应用 (All Apps)
 *   6. 所有文件 (All Files)   - 含多选分类筛选 chip + 增量展开
 *
 * 动画策略 (重要):
 *   - 行 .vg__rows 用 <Transition name="rows"> 包裹, 折叠时高度/透明度
 *     平滑过渡而不是瞬间消失, 修复"点击箭头后内容立即消失"问题.
 *   - 折叠箭头用 CSS rotate 平滑旋转.
 *   - 选中行始终有 2px accent 进度条 + 缩放图标 + accent 文字色.
 */
import { computed, ref, watch, nextTick, onBeforeUnmount, onMounted } from 'vue'
import {
  Folder, Settings as SettingsIcon, Terminal, PinIcon, Clock,
  Sparkles, ChevronDown, Filter
} from '@lucide/vue'
import type { SearchResult } from '@/types/search'
import type { DisplayGroup, GroupId } from '@/stores/search'
import ResultItem from '@/components/common/ResultItem.vue'
import AppResultItem from '@/components/search/AppResultItem.vue'
import CheckButton from '@/components/common/CheckButton.vue'
import { classify, classifyByResultType, FILE_KIND_META, FILE_KIND_DISPLAY_ORDER, type FileKind } from '@/utils/fileKinds'
import { useSearchStore } from '@/stores/search'

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
  itemHeight?: number
  hasQuery?: boolean
  /** 当前查询关键字, 用于"搜索 X 中…"提示文案. */
  query?: string
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  height: 400,
  itemHeight: 44,
  hasQuery: false,
  query: '',
})

const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
  (e: 'open', item: SearchResult): void
  (e: 'hover', index: number): void
  (e: 'contextmenu', event: MouseEvent, item: SearchResult): void
  (e: 'toggle-group', id: GroupId): void
  (e: 'show-more-files'): void
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

/**
 * 文件类型过滤已委托到 store.search.setFileKindFilter().
 * 此处保留此函数仅为文件类型下拉面板的 count 统计使用,
 * visibleGroups 直接使用 store 过滤后的 displayGroups.
 */
function applyFileKindFilter(items: SearchResult[]): SearchResult[] {
  return items
}

/** 真正渲染到 DOM 的分组 —— 应用文件类型过滤后. */
const visibleGroups = computed<DisplayGroup[]>(() => {
  // 防御: props.groups 在组件挂载早期可能是 undefined, 直接 .map() 会白屏.
  return (props.groups ?? [])
    .map((g) => {
      if (g.kind !== 'files') return g
      const filtered = applyFileKindFilter(g.visibleItems)
      return { ...g, visibleItems: filtered }
    })
    .filter((g) => g.items.length > 0 || g.collapsed)
})

/** 每个分组的全局起始 offset (用于键盘上下方向键 / 高亮联动). */
const itemOffsetOfGroup = computed(() => {
  const map: Record<string, number> = {}
  let off = 0
  for (const g of visibleGroups.value) {
    map[g.id] = off
    off += g.collapsed ? 0 : g.visibleItems.length
  }
  return map
})

/** 全局扁平化列表 —— 与 store.displayList 严格一致. */
const flatItems = computed<SearchResult[]>(() => {
  const out: SearchResult[] = []
  for (const g of visibleGroups.value) {
    if (!g.collapsed) {
      for (const it of g.visibleItems) out.push(it)
    }
  }
  return out
})

// === 滚动到指定项 ===
const scrollerRef = ref<HTMLElement | null>(null)

/**
 * 滚动到指定全局 index 对应的行. 优先用 scrollIntoView (现代浏览器),
 * 失败时回退到手工 scrollTop 计算.
 *
 * 修复: 旧版仅用 offsetTop 计算, 但 collapse/expand 动画期间
 * offsetTop 处于中间值, 导致滚动位置抖动. scrollIntoView 的
 * { block: 'nearest' } 行为会自动跳过已在视口内的项, 视觉更稳定.
 */
function scrollToGlobalIndex(idx: number) {
  if (!scrollerRef.value) return
  if (idx < 0) return
  const el = scrollerRef.value.querySelector<HTMLElement>(`[data-global-idx="${idx}"]`)
  if (!el) {
    // 元素可能还在 transition 中, 等下一帧再试
    requestAnimationFrame(() => {
      const retry = scrollerRef.value?.querySelector<HTMLElement>(`[data-global-idx="${idx}"]`)
      retry?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
    })
    return
  }
  try {
    el.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
  } catch {
    // 回退: 手工计算 scrollTop
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

/**
 * 折叠状态变化时: 等待 max-height 过渡 (280ms) 完成后再滚动,
 * 避免在 transition 中读到错误的 offsetTop.
 *
 * 关键防护: props.groups 在组件挂载早期可能是 undefined (store 还没
 * 产生 DisplayGroup). 直接 `.map()` 会抛 "Cannot read properties of
 * undefined (reading 'map')" 让整个 setup watch 链路死掉, 出现白屏.
 * 用 `?` + `?? ''` 把 undefined 折成空字符串, 让 join 返回 '' (稳定)
 * 而 watcher 第一次的 (undefined→'') 不被错误地当成"折叠状态变化".
 */
watch(() => (props.groups ?? []).map((g) => g.collapsed).join(','), () => {
  nextTick(() => {
    setTimeout(() => scrollToGlobalIndex(props.selectedIndex), 320)
  })
})

watch(() => props.groups, () => {
  if (scrollerRef.value) scrollerRef.value.scrollTop = 0
}, { deep: true })

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
  for (const r of props.groups.find((g) => g.kind === 'files')?.items ?? []) {
    const byType = classifyByResultType((r as any).resultType)
    const ext = (r.subtitle || r.title || '').split(/[\\/]/).pop() || ''
    const kind = byType ?? classify(ext)
    m[kind] = (m[kind] || 0) + 1
  }
  return m
})

const isLoading = computed(() => props.loading && props.hasQuery)
const nothingNow = computed(() => !props.loading && flatItems.value.length === 0)

/** 单击 → 只更新 selectedIndex (无副作用). 双击 → 真正打开. */
function onPickItem(item: SearchResult) {
  emit('select', item)
}

function onOpenItem(item: SearchResult) {
  emit('open', item)
}

function onItemHover(idx: number) { emit('hover', idx) }

/** "显示更多": 通知 store 增加可见文件数. */
function onShowMoreFiles() {
  emit('show-more-files')
}

/** 切换分组折叠: 通知 store. */
function onToggleGroup(id: GroupId) {
  emit('toggle-group', id)
}

/** 分组是否处于展开状态 (供模板 aria-expanded 使用). */
function isCollapsed(g: DisplayGroup): boolean {
  return g.collapsed
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

    <div v-else ref="scrollerRef" class="vg__scroller">
      <div class="vg__list">
        <template v-for="(g, gi) in visibleGroups" :key="g.id">
          <!-- 分组 (无卡片背景, 仅细分隔线) -->
          <section
            class="vg__group"
            :class="{ 'vg__group--first': gi === 0, 'vg__group--files': g.kind === 'files' }"
          >
            <!-- 标题行: 左侧图标 + 标题 + 计数; 右侧 [筛选] [折叠箭头] -->
            <div class="vg__group-header">
              <div class="vg__group-header-left">
                <component :is="GROUP_ICONS[g.kind]" :size="15" :stroke-width="1.7" class="vg__group-icon" />
                <span class="vg__group-title">{{ g.title }}</span>
                <span v-if="g.items.length" class="vg__group-count">{{ g.items.length }}</span>
              </div>

              <div class="vg__group-header-right">
                <!-- 所有文件: 标题行右侧的下拉多选 -->
                <div v-if="g.kind === 'files'" ref="filterDropdownRef" class="vg__filter-dropdown">
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

                <!-- 折叠/展开箭头: 位于分组标题行最右侧 -->
                <button
                  type="button"
                  class="vg__group-toggle"
                  :class="{ 'vg__group-toggle--collapsed': isCollapsed(g) }"
                  @click="onToggleGroup(g.id)"
                  :aria-expanded="!isCollapsed(g)"
                  :aria-label="isCollapsed(g) ? '展开分组' : '折叠分组'"
                  :title="isCollapsed(g) ? '展开分组' : '折叠分组'"
                >
                  <ChevronDown :size="14" :stroke-width="2.2" />
                </button>
              </div>
            </div>

            <!-- 行容器: v-show + max-height transition 让折叠/展开平滑过渡.
                 修复"元素瞬间消失"问题: 旧版用 TransitionGroup + position:absolute
                 让行脱离文档流, 父容器高度瞬间坍缩, 视觉上看不到过渡.
                 现在由 wrapper 的 max-height 控制整段高度, 行内再叠加淡入淡出. -->
            <Transition
              name="group-collapse"
              appear
            >
              <div
                v-show="!isCollapsed(g)"
                class="vg__rows-wrapper"
              >
                <TransitionGroup
                  tag="div"
                  name="rows"
                  class="vg__rows"
                  appear
                >
                  <div
                    v-for="(it, localIdx) in g.visibleItems"
                    :key="it.id + ':' + localIdx"
                    :data-global-idx="itemOffsetOfGroup[g.id] + localIdx"
                    class="vg__row"
                    :class="{ 'vg__row--active': (itemOffsetOfGroup[g.id] + localIdx) === selectedIndex }"
                    @click="onPickItem(it)"
                    @dblclick="onOpenItem(it)"
                    @mouseover="onItemHover(itemOffsetOfGroup[g.id] + localIdx)"
                    @contextmenu.prevent="(e) => emit('contextmenu', e, it)"
                  >
                    <AppResultItem
                      v-if="g.kind === 'apps' || g.kind === 'pinned' || g.kind === 'recent' || g.kind === 'system'"
                      :result="it"
                      :index="itemOffsetOfGroup[g.id] + localIdx"
                      :active="(itemOffsetOfGroup[g.id] + localIdx) === selectedIndex"
                    />
                    <ResultItem
                      v-else
                      :result="it"
                      :index="itemOffsetOfGroup[g.id] + localIdx"
                      :active="(itemOffsetOfGroup[g.id] + localIdx) === selectedIndex"
                    />
                  </div>

                  <!-- "所有文件" 分组: 还有更多未展示时, 渲染展开按钮 -->
                  <button
                    v-if="g.kind === 'files' && g.hiddenCount && g.hiddenCount > 0"
                    :key="`show-more-${g.id}`"
                    type="button"
                    class="vg__show-more"
                    @click="onShowMoreFiles"
                  >
                    <ChevronDown :size="13" :stroke-width="2" />
                    <span>显示更多 (+{{ g.hiddenCount }})</span>
                  </button>
                </TransitionGroup>
              </div>
            </Transition>
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

/* === 折叠/展开箭头按钮 === */
.vg__group-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
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
/* wrapper: v-show + max-height transition, 让整段行的高度平滑收放.
 * 旧版问题: TransitionGroup 直接放在 v-if 上, 行一离开 (position:absolute)
 * 父容器就瞬间坍缩, 用户看不到过渡. 现在高度由 wrapper 控制. */
.vg__rows-wrapper {
  overflow: hidden;
  /* max-height 大到能容纳约 50 行 (50 * 36px ≈ 1800px), 超出部分
   * 由 v-show 隐藏, 但 transition 期间仍可见动画. */
  max-height: 2000px;
  transition:
    max-height 280ms cubic-bezier(0.16, 1, 0.3, 1),
    opacity 200ms cubic-bezier(0.16, 1, 0.3, 1);
  opacity: 1;
}

.vg__rows {
  display: flex;
  flex-direction: column;
  gap: 1px;
  /* 关键: overflow:hidden 让 TransitionGroup 的高度/透明度过渡生效,
   * 否则子项脱离时仍会"瞬间消失" (因为父容器没有限制). */
  overflow: hidden;
  transition: max-height 320ms cubic-bezier(0.16, 1, 0.3, 1);
}

/* === group-collapse Transition: v-show 时的 max-height 动画 === */
.group-collapse-enter-active,
.group-collapse-leave-active {
  transition:
    max-height 280ms cubic-bezier(0.16, 1, 0.3, 1),
    opacity 200ms cubic-bezier(0.16, 1, 0.3, 1);
  overflow: hidden;
  /* 折叠时给 leave 一个稍快一点的节奏, 视觉上"先收高再消". */
  will-change: max-height, opacity;
}
.group-collapse-enter-from,
.group-collapse-leave-to {
  max-height: 0;
  opacity: 0;
}
.group-collapse-enter-to,
.group-collapse-leave-from {
  max-height: 2000px;
  opacity: 1;
}

.vg__row {
  display: block;
  cursor: pointer;
  border-radius: var(--radius-md);
  transition:
    background var(--dur-fast) var(--ease-out),
    opacity 220ms cubic-bezier(0.16, 1, 0.3, 1),
    transform 220ms cubic-bezier(0.16, 1, 0.3, 1);
}

.vg__row:hover {
  background: var(--list-hover-bg);
}

.vg__row--active {
  background: var(--list-selected-bg);
}

.vg__row--active:hover {
  background: var(--list-selected-bg);
  filter: brightness(1.1);
}

/* === TransitionGroup: 折叠/展开/插入/删除 行 时平滑过渡 === */
.rows-enter-active {
  transition:
    opacity 240ms cubic-bezier(0.16, 1, 0.3, 1),
    transform 240ms cubic-bezier(0.16, 1, 0.3, 1);
}
/* 修复: 旧版 position:absolute 让行脱离文档流, 父容器瞬间坍缩, 看不到过渡.
 * 现在高度由 .vg__rows-wrapper 的 max-height 控制, 行只做淡出. */
.rows-leave-active {
  transition:
    opacity 160ms cubic-bezier(0.4, 0, 1, 1),
    transform 160ms cubic-bezier(0.4, 0, 1, 1);
}
.rows-enter-from {
  opacity: 0;
  transform: translateY(-6px);
}
.rows-leave-to {
  opacity: 0;
  transform: translateY(-4px) scale(0.98);
}
.rows-move {
  transition: transform 320ms cubic-bezier(0.16, 1, 0.3, 1);
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
