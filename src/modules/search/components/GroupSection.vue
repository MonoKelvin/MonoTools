<script setup lang="ts">
import { computed, ref, watch, nextTick, onMounted } from 'vue'
import type { SearchResult } from '@/modules/search'
import AppResultItem from './AppResultItem.vue'
import ResultItem from '@/modules/search/components/ResultItem.vue'
import MtComboBox from '@/ui/components/MtComboBox.vue'
import type { MtComboBoxOption } from '@/ui/components/MtComboBox.vue'
import type { SortMode } from '@/core/config/sorting'
import { LayoutList, Grid3X3, WrapText, LayoutGrid, Sparkles, Type, Clock, Folder } from '@lucide/vue'

export type LayoutMode = 'list' | 'grid-fixed' | 'grid-auto' | 'icon'

const props = defineProps<{
  id: string
  title: string
  icon: any
  items: SearchResult[]
  collapsed: boolean
  kind: 'pinned' | 'recent' | 'system' | 'commands' | 'apps' | 'files'
  count?: number
  showFilter?: boolean
  defaultLayout?: LayoutMode
  selectedGlobalIndex?: number
  hoveredGlobalIndex?: number
  startIndex?: number
  gridCols?: number
  sortMode?: SortMode
  sortOptions?: MtComboBoxOption[]
  collapsedItems?: SearchResult[]
}>()

const emit = defineEmits<{
  (e: 'toggle-collapse', id: string): void
  (e: 'select', item: SearchResult, globalIndex: number, event: MouseEvent): void
  (e: 'open', item: SearchResult): void
  (e: 'hover', globalIndex: number): void
  (e: 'contextmenu', event: MouseEvent, item: SearchResult, globalIndex: number): void
  (e: 'layout-change', mode: LayoutMode): void
  (e: 'sort-change', mode: SortMode): void
}>()

const DEFAULT_LAYOUT_BY_KIND: Record<string, LayoutMode> = {
  pinned: 'grid-fixed',
  recent: 'grid-fixed',
  apps: 'icon',
  commands: 'list',
  files: 'list',
}

/** 各分组类型的默认排序模式 */
const DEFAULT_SORT_BY_KIND: Record<string, string> = {
  pinned: 'recent',
  recent: 'recent',
  apps: 'smart',
  system: 'name',
  commands: 'name',
  files: 'name',
}

const layoutMode = ref<LayoutMode>(props.defaultLayout || DEFAULT_LAYOUT_BY_KIND[props.kind] || 'list')

/** 排序图标映射 */
const sortIconMap: Record<string, any> = {
  smart: Sparkles,
  name: Type,
  recent: Clock,
  path: Folder,
}

const sortOptions = computed<MtComboBoxOption[]>(() => props.sortOptions || [])

/** 排序 combobox 的显示值（优先使用传入的 sortMode，否则使用默认值） */
const displaySortMode = computed(() => props.sortMode || DEFAULT_SORT_BY_KIND[props.kind] || 'name')

const layoutOptions: MtComboBoxOption[] = [
  { key: 'list', label: '列表模式', icon: LayoutList },
  { key: 'grid-fixed', label: '等宽网格', icon: Grid3X3 },
  { key: 'grid-auto', label: '自适应宽度', icon: WrapText },
  { key: 'icon', label: '图标模式', icon: LayoutGrid },
]

const isAppKind = (kind: string) => kind === 'pinned' || kind === 'recent' || kind === 'apps' || kind === 'system'

const startIdx = computed(() => props.startIndex ?? 0)

const selectedLocalIndex = computed(() => {
  if (props.selectedGlobalIndex == null) return -1
  const local = props.selectedGlobalIndex - startIdx.value
  return local >= 0 && local < props.items.length ? local : -1
})

const hoveredLocalIndex = computed(() => {
  if (props.hoveredGlobalIndex == null) return -1
  const local = props.hoveredGlobalIndex - startIdx.value
  return local >= 0 && local < props.items.length ? local : -1
})

const gridStyle = computed(() => {
  if (layoutMode.value === 'grid-fixed') {
    const cols = props.gridCols || 3
    return { gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }
  }
  return {}
})

const iconGridStyle = computed(() => {
  if (layoutMode.value === 'icon') {
    return { gridTemplateColumns: 'repeat(auto-fill, minmax(90px, 1fr))' }
  }
  return {}
})

/**
 * 空组 (items.length === 0) 不响应点击. store 层 toggleGroupCollapse
 * 也会过滤, 这里再做一道 UI 防御: cursor 不变 pointer, header 不可点击.
 *
 * 满足产品诉求 "下面如果没有内容, 则不支持折叠展开". 视觉上 header 还在,
 * 但点击不会触发任何动作, 鼠标光标也保持默认 (不显示 "可点击" 暗示).
 */
const isInteractive = computed(() => (props.collapsedItems ?? props.items).length > 0)

/** 防止快速重复点击导致状态翻转 */
let toggleDebounceTimer: ReturnType<typeof setTimeout> | null = null

function toggleCollapse() {
  // 空组不响应 (兜底, store 也会过滤). 防止 hover 出 pointer 误导用户.
  if (!isInteractive.value) return
  // 防抖: 50ms 内不重复处理
  if (toggleDebounceTimer) return
  toggleDebounceTimer = setTimeout(() => {
    toggleDebounceTimer = null
  }, 50)
  emit('toggle-collapse', props.id)
}

/** 使用 watch + requestAnimationFrame 实现流畅的折叠/展开动画 */
const isAnimating = ref(false)

/** 是否已完成首次初始化 */
const isInitialized = ref(false)

// 组件挂载后初始化默认展开状态的内联高度
onMounted(async () => {
  await nextTick()
  const wrapper = document.getElementById(`group-content-${props.id}`)
  if (!wrapper) return

  // 如果 props.collapsed 是 false（默认展开），读取真实高度并设置内联样式
  if (!props.collapsed) {
    wrapper.style.transition = 'none'
    wrapper.style.height = 'auto'
    await nextTick()
    const realH = wrapper.scrollHeight
    wrapper.style.height = `${realH}px`
  }
  isInitialized.value = true
})

watch(
  () => props.collapsed,
  (collapsed) => {
    const wrapper = document.getElementById(`group-content-${props.id}`)
    if (!wrapper) return

    if (!isInitialized.value) {
      // 首次 watch 触发时等待初始化完成
      return
    }

    if (collapsed) {
      // 收缩：先渐隐再收拢高度，避免内容直接消失
      isAnimating.value = true

      // 读取当前高度
      wrapper.style.transition = 'none'
      const currentInlineH = parseInt(wrapper.style.height, 10)
      const readH = currentInlineH > 15 ? currentInlineH : wrapper.scrollHeight
      if (readH <= 15) return

      // 重置到展开态基线
      wrapper.style.height = `${readH}px`
      wrapper.style.opacity = '1'
      void wrapper.offsetHeight

      // 第一段：只渐隐，不改变高度
      wrapper.style.transition = `opacity ${DURATION}ms ease-out 0ms`
      requestAnimationFrame(() => {
        wrapper.style.opacity = '0'

        // opacity 动画开始后，再收拢高度
        setTimeout(() => {
          wrapper.style.transition = `height ${DURATION}ms ease-in 0ms`
          requestAnimationFrame(() => {
            wrapper.style.height = '0'
          })
        }, 40)
      })

      setTimeout(() => { isAnimating.value = false }, DURATION + 50)
    } else {
      // 展开：强制显示 → 回到起始状态 → 动画展开
      isAnimating.value = true
      wrapper.style.display = 'block'
      wrapper.style.transition = 'none'
      wrapper.style.height = '0'
      wrapper.style.opacity = '0'
      wrapper.style.transform = 'translateY(6px) scale(0.99)'
      void wrapper.offsetHeight // 强制重排

      // 读取目标高度
      wrapper.style.height = 'auto'
      void wrapper.offsetHeight
      const targetH = wrapper.scrollHeight

      // 回到展开的起始状态
      wrapper.style.height = '0'
      wrapper.style.opacity = '0'
      wrapper.style.transform = 'translateY(6px) scale(0.99)'

      wrapper.style.transition = `height ${DURATION}ms ${EASE}, opacity ${DURATION - 40}ms ease-out 20ms, transform ${DURATION}ms ${EASE} 10ms`
      requestAnimationFrame(() => {
        wrapper.style.height = `${targetH}px`
        wrapper.style.opacity = '1'
        wrapper.style.transform = ''
      })

      setTimeout(() => { isAnimating.value = false }, DURATION + 50)
    }
  },
  { flush: 'post' }
)

function globalIndexOf(localIndex: number): number {
  return startIdx.value + localIndex
}

function onItemClick(item: SearchResult, localIndex: number, event: MouseEvent) {
  emit('select', item, globalIndexOf(localIndex), event)
}

function onItemDblClick(item: SearchResult) {
  emit('open', item)
}

function onItemHover(localIndex: number) {
  emit('hover', globalIndexOf(localIndex))
}

function onItemLeave() {
  emit('hover', -1)
}

function onItemContextMenu(event: MouseEvent, item: SearchResult, localIndex: number) {
  event.preventDefault()
  emit('contextmenu', event, item, globalIndexOf(localIndex))
}

const DURATION = 300
const EASE = 'cubic-bezier(0.25, 0, 0.15, 1)'

function onLayoutChange(key: string) {
  const mode = key as LayoutMode
  layoutMode.value = mode
  emit('layout-change', mode)
}

function onSortChange(key: string) {
  const mode = key as SortMode
  emit('sort-change', mode)
}

function isItemActive(localIndex: number): boolean {
  return selectedLocalIndex.value === localIndex
}

function isItemHovered(localIndex: number): boolean {
  return hoveredLocalIndex.value === localIndex
}
</script>

<template>
  <div class="group-section" :data-kind="kind" :data-interactive="isInteractive ? '1' : '0'">
    <div class="group-header" @click="toggleCollapse">
      <div class="group-header-left">
        <component :is="icon" :size="13" :stroke-width="1.8" class="group-icon" />
        <span class="group-title">{{ title }}</span>
        <span v-if="count != null && count > 0" class="group-count">{{ count.toLocaleString() }}</span>
      </div>
      <div class="group-header-right" @click.stop>
        <div v-if="items.length > 0" class="group-sort-toggle">
          <MtComboBox
            :options="sortOptions"
            :model-value="displaySortMode"
            dropdown-anchor="#search-container"
            :compact="true"
            @update:model-value="onSortChange"
          />
        </div>
        <div v-if="items.length > 0" class="group-layout-toggle">
          <MtComboBox
            :options="layoutOptions"
            :model-value="layoutMode"
            dropdown-anchor="#search-container"
            :compact="true"
            @update:model-value="onLayoutChange"
          />
        </div>
      </div>
    </div>

    <div :id="`group-content-${id}`" class="group-content-wrapper">
      <div class="group-content-inner">
          <Transition name="layout-fade" mode="out-in">
            <div :key="layoutMode" class="group-content" :class="`group-content--${layoutMode}`">
              <!-- list mode -->
              <template v-if="layoutMode === 'list'">
                <div
                  v-for="(item, idx) in items"
                  :key="`${id}-${item.id}-${idx}`"
                  class="gs-item gs-item--list"
                  :class="{
                    'gs-item--active': isItemActive(idx),
                    'gs-item--hover': isItemHovered(idx),
                  }"
                  @click="(e) => onItemClick(item, idx, e)"
                  @dblclick="onItemDblClick(item)"
                  @mouseenter="onItemHover(idx)"
                  @mouseleave="onItemLeave"
                  @contextmenu="(e) => onItemContextMenu(e, item, idx)"
                >
                  <AppResultItem v-if="isAppKind(kind)" :result="item" :index="idx" :active="isItemActive(idx)" badge-size="sm" />
                  <ResultItem v-else :result="item" :index="idx" :active="isItemActive(idx)" />
                </div>
              </template>

              <!-- grid-fixed / grid-auto -->
              <template v-else-if="layoutMode === 'grid-fixed' || layoutMode === 'grid-auto'">
                <div class="gs-grid" :style="gridStyle">
                  <div
                    v-for="(item, idx) in items"
                    :key="`${id}-${item.id}-${idx}`"
                    class="gs-item gs-item--grid"
                    :class="{
                      'gs-item--active': isItemActive(idx),
                      'gs-item--hover': isItemHovered(idx),
                    }"
                    @click="(e) => onItemClick(item, idx, e)"
                    @dblclick="onItemDblClick(item)"
                    @mouseenter="onItemHover(idx)"
                    @mouseleave="onItemLeave"
                    @contextmenu="(e) => onItemContextMenu(e, item, idx)"
                  >
                    <AppResultItem v-if="isAppKind(kind)" :result="item" :index="idx" :active="isItemActive(idx)" badge-size="sm" />
                    <ResultItem v-else :result="item" :index="idx" :active="isItemActive(idx)" />
                  </div>
                </div>
              </template>

              <!-- icon mode -->
              <template v-else-if="layoutMode === 'icon'">
                <div class="gs-grid gs-grid--icon" :style="iconGridStyle">
                  <div
                    v-for="(item, idx) in items"
                    :key="`${id}-${item.id}-${idx}`"
                    class="gs-item gs-item--icon"
                    :class="{
                      'gs-item--active': isItemActive(idx),
                      'gs-item--hover': isItemHovered(idx),
                    }"
                    @click="(e) => onItemClick(item, idx, e)"
                    @dblclick="onItemDblClick(item)"
                    @mouseenter="onItemHover(idx)"
                    @mouseleave="onItemLeave"
                    @contextmenu="(e) => onItemContextMenu(e, item, idx)"
                  >
                    <div class="gs-icon-mode-icon">
                      <AppResultItem v-if="isAppKind(kind)" :result="item" :index="idx" :active="isItemActive(idx)" badge-size="xs" />
                      <ResultItem v-else :result="item" :index="idx" :active="isItemActive(idx)" />
                    </div>
                    <div class="gs-icon-mode-title">{{ item.title }}</div>
                  </div>
                </div>
              </template>
            </div>
          </Transition>
        </div>
      </div>
  </div>
</template>

<style scoped>
.group-section {
  display: flex;
  flex-direction: column;
  width: 100%;
  position: relative;
}

.group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 44px;
  padding: 0 6px;
  gap: 10px;
  box-sizing: border-box;
  flex-shrink: 0;
  background: transparent;
  cursor: pointer;
  user-select: none;
  position: relative;
  /* 移除 border-top, 改用 ::before 伪元素延伸到窗口两边, 解决"分组间
     分割线左右有间距" 的视觉问题. 数值 -8px 对应父容器
     .results-scroll-container 的 padding-left/right (var(--sp-3) = 8px). */
  border-top: none;
}

.group-header::before {
  content: '';
  position: absolute;
  top: 0;
  left: -8px;
  right: -8px;
  height: 1px;
  background: var(--border-subtle);
  pointer-events: none;
}

.group-section[data-kind="pinned"] .group-header::before,
.group-section[data-kind="recent"] .group-header::before {
  /* 固定项目 / 最近访问 是首个分组 (紧贴 SearchInput), 不画顶部分割线 */
  display: none;
}

/* 空组 (无内容) 不可点击. 鼠标光标保持默认, 避免误导用户. */
.group-section[data-interactive="0"] .group-header {
  cursor: default;
}

.group-header-left {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.group-icon {
  color: var(--text-tertiary);
  flex-shrink: 0;
  transition: color var(--dur-fast) var(--ease-out);
}

.group-header:hover .group-icon {
  color: var(--text-secondary);
}

.group-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  letter-spacing: 0.3px;
  flex-shrink: 0;
  transition: color var(--dur-fast) var(--ease-out);
}

.group-header:hover .group-title {
  color: var(--text-primary);
}

.group-count {
  font-size: 11px;
  color: var(--text-tertiary);
  font-weight: 500;
  padding: 2px 6px;
  border-radius: 999px;
  background: var(--bg-tertiary);
  flex-shrink: 0;
}

.group-header:hover .group-count {
  color: var(--text-secondary);
}

.group-header-right {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
  position: relative;
}

.group-sort-toggle {
  display: flex;
  align-items: center;
}

.group-layout-toggle {
  display: flex;
  align-items: center;
}

/* === 折叠过渡动画 === */

.group-content-wrapper {
  overflow: hidden;
  will-change: height, opacity, transform;
}

.group-content-inner {
  width: 100%;
}

.group-content {
  width: 100%;
}

/* === 布局切换过渡动画 === */

.layout-fade-enter-active,
.layout-fade-leave-active {
  transition:
    opacity 180ms var(--ease-out),
    transform 180ms var(--ease-out);
}

.layout-fade-enter-from {
  opacity: 0;
  transform: translateY(4px);
}

.layout-fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

/* === list mode === */

.gs-item--list {
  display: block;
  cursor: pointer;
  border-radius: var(--radius-md);
  outline: none;
  transition: background var(--dur-fast) var(--ease-out);
  margin-bottom: 1px;
}

.gs-item--list:hover,
.gs-item--hover.gs-item--list {
  background: var(--list-hover-bg);
}

.gs-item--active.gs-item--list {
  background: var(--list-selected-bg);
}

/* === grid mode (fixed / auto) === */

.gs-grid {
  display: grid;
  gap: 6px;
  padding: 4px 2px 8px;
}

/* grid-auto 使用 flex 流式布局，保证最小宽度和边距自适应 */
.group-content--grid-auto .gs-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 4px 2px 8px;
}

/* flex 布局下 gs-item--grid 用内容自然宽度，不强制拉伸 */
.group-content--grid-auto .gs-item--grid {
  flex: 0 0 auto;
  width: auto;
  min-width: 140px;
  max-width: 280px;
}

.group-content--grid-auto .gs-item--grid :deep(.app-result-item),
.group-content--grid-auto .gs-item--grid :deep(.result-item) {
  width: auto;
}

.group-content--grid-auto .gs-item--grid :deep(.app-result-item__title),
.group-content--grid-auto .gs-item--grid :deep(.result-item__title),
.group-content--grid-auto .gs-item--grid :deep(.result-item__subtitle) {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.gs-item--grid {
  display: block;
  cursor: pointer;
  border-radius: var(--radius-md);
  background: transparent;
  border: none;
  outline: none;
  transition: background var(--dur-fast) var(--ease-out);
  overflow: visible;
  position: relative;
}

.gs-item--grid:hover,
.gs-item--hover.gs-item--grid {
  background: var(--list-hover-bg);
}

.gs-item--active.gs-item--grid {
  background: var(--list-selected-bg);
}

/* 网格模式下的内层样式：保持与列表模式类似的间距和高度 */
.gs-item--grid :deep(.app-result-item),
.gs-item--grid :deep(.result-item) {
  background: transparent;
  border: none;
  border-radius: 0;
  padding: 7px 12px;
  gap: var(--sp-4);
  height: 100%;
  min-height: 42px;
}

.gs-item--grid :deep(.app-result-item__icon),
.gs-item--grid :deep(.result-item__icon) {
  transform: none !important;
  filter: none !important;
  transition: none !important;
}

/* === icon mode === */

.gs-grid--icon {
  gap: 8px;
  padding: 8px 4px 12px;
}

.gs-item--icon {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0;
  padding: 12px 8px 10px;
  border-radius: var(--radius-md);
  cursor: pointer;
  background: transparent;
  border: none;
  outline: none;
  transition: background var(--dur-fast) var(--ease-out);
}

.gs-item--icon:hover,
.gs-item--hover.gs-item--icon {
  background: var(--list-hover-bg);
}

.gs-item--active.gs-item--icon {
  background: var(--list-selected-bg);
}

.gs-icon-mode-icon {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.gs-icon-mode-icon :deep(.app-result-item) {
  padding: 0;
  border: none;
  background: transparent;
  border-radius: 0;
  gap: 0;
}

.gs-icon-mode-icon :deep(.app-result-item__icon) {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  transform: none !important;
  filter: none !important;
  transition: none !important;
}

.gs-icon-mode-icon :deep(.app-result-item__img) {
  width: 100%;
  height: 100%;
  padding: 2px;
}

.gs-icon-mode-icon :deep(.app-result-item__title),
.gs-icon-mode-icon :deep(.app-result-item__meta) {
  display: none;
}

.gs-icon-mode-icon :deep(.result-item) {
  padding: 0;
  border: none;
  background: transparent;
  border-radius: 0;
  gap: 0;
}

.gs-icon-mode-icon :deep(.result-item__icon) {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  transform: none !important;
  filter: none !important;
  transition: none !important;
}

.gs-icon-mode-icon :deep(.result-item__img) {
  width: 32px;
  height: 32px;
}

.gs-icon-mode-icon :deep(.result-item__content),
.gs-icon-mode-icon :deep(.result-item__meta) {
  display: none;
}

.gs-icon-mode-title {
  font-size: 11px;
  text-align: center;
  line-height: 1.3;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  white-space: normal;
  color: var(--text-primary);
  font-weight: 500;
  overflow: hidden;
  max-width: 86px;
  margin-top: 8px;
}
</style>
