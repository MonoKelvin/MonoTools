<script setup lang="ts">
import { computed, ref } from 'vue'
import type { SearchResult } from '@/types/search'
import AppResultItem from './AppResultItem.vue'
import ResultItem from '@/components/common/ResultItem.vue'
import MtComboBox from '@/components/common/MtComboBox.vue'
import type { MtComboBoxOption } from '@/components/common/MtComboBox.vue'
import { LayoutList, Grid3X3, WrapText, LayoutGrid } from '@lucide/vue'

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
}>()

const emit = defineEmits<{
  (e: 'toggle-collapse', id: string): void
  (e: 'select', item: SearchResult, globalIndex: number, event: MouseEvent): void
  (e: 'open', item: SearchResult): void
  (e: 'hover', globalIndex: number): void
  (e: 'contextmenu', event: MouseEvent, item: SearchResult, globalIndex: number): void
  (e: 'layout-change', mode: LayoutMode): void
}>()

const DEFAULT_LAYOUT_BY_KIND: Record<string, LayoutMode> = {
  pinned: 'grid-fixed',
  recent: 'grid-fixed',
  system: 'icon',
  apps: 'icon',
  commands: 'list',
  files: 'list',
}

const layoutMode = ref<LayoutMode>(props.defaultLayout || DEFAULT_LAYOUT_BY_KIND[props.kind] || 'list')

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

function toggleCollapse() {
  emit('toggle-collapse', props.id)
}

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

function finishTransition(node: HTMLElement, done: () => void) {
  let finished = false
  const finish = () => {
    if (finished) return
    finished = true
    node.removeEventListener('transitionend', onEnd)
    node.style.height = ''
    node.style.opacity = ''
    node.style.transform = ''
    node.style.transition = ''
    done()
  }
  const onEnd = (event: TransitionEvent) => {
    if (event.target === node && event.propertyName === 'height') finish()
  }

  node.addEventListener('transitionend', onEnd)
  window.setTimeout(finish, 340)
}

function onCollapseBeforeEnter(el: Element) {
  const node = el as HTMLElement
  node.style.height = '0'
  node.style.opacity = '0'
  node.style.transform = 'translateY(-4px)'
}

function onCollapseEnter(el: Element, done: () => void) {
  const node = el as HTMLElement
  node.style.transition = 'height 260ms cubic-bezier(0.22, 1, 0.36, 1), opacity 160ms ease-out, transform 260ms cubic-bezier(0.22, 1, 0.36, 1)'
  requestAnimationFrame(() => {
    node.style.height = `${node.scrollHeight}px`
    node.style.opacity = '1'
    node.style.transform = 'translateY(0)'
  })
  finishTransition(node, done)
}

function onCollapseBeforeLeave(el: Element) {
  const node = el as HTMLElement
  node.style.height = `${node.scrollHeight}px`
  node.style.opacity = '1'
  node.style.transform = 'translateY(0)'
}

function onCollapseLeave(el: Element, done: () => void) {
  const node = el as HTMLElement
  node.style.transition = 'height 180ms cubic-bezier(0.4, 0, 1, 1), opacity 120ms ease-in, transform 180ms cubic-bezier(0.4, 0, 1, 1)'
  requestAnimationFrame(() => {
    node.style.height = '0'
    node.style.opacity = '0'
    node.style.transform = 'translateY(-3px)'
  })
  finishTransition(node, done)
}

function onLayoutChange(key: string) {
  const mode = key as LayoutMode
  layoutMode.value = mode
  emit('layout-change', mode)
}

function isItemActive(localIndex: number): boolean {
  return selectedLocalIndex.value === localIndex
}

function isItemHovered(localIndex: number): boolean {
  return hoveredLocalIndex.value === localIndex
}
</script>

<template>
  <div class="group-section" :data-kind="kind">
    <div class="group-header" @click="toggleCollapse">
      <div class="group-header-left">
        <component :is="icon" :size="13" :stroke-width="1.8" class="group-icon" />
        <span class="group-title">{{ title }}</span>
        <span v-if="count != null" class="group-count">{{ count.toLocaleString() }}</span>
      </div>
      <div class="group-header-right" @click.stop>
        <div v-if="items.length > 0" class="group-layout-toggle">
          <MtComboBox
            :options="layoutOptions"
            :model-value="layoutMode"
            dropdown-anchor="#search-container"
            @update:model-value="onLayoutChange"
          />
        </div>
      </div>
    </div>

    <Transition
      @before-enter="onCollapseBeforeEnter"
      @enter="onCollapseEnter"
      @before-leave="onCollapseBeforeLeave"
      @leave="onCollapseLeave"
    >
      <div v-if="!collapsed" class="group-content-wrapper">
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
                  <AppResultItem v-if="isAppKind(kind)" :result="item" :index="idx" :active="isItemActive(idx)" />
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
                    <AppResultItem v-if="isAppKind(kind)" :result="item" :index="idx" :active="isItemActive(idx)" />
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
                      <AppResultItem v-if="isAppKind(kind)" :result="item" :index="idx" :active="isItemActive(idx)" />
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
    </Transition>
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
  border-top: 1px solid var(--border-subtle);
  box-sizing: border-box;
  flex-shrink: 0;
  background: transparent;
  cursor: pointer;
  user-select: none;
}

.group-section[data-kind="pinned"] .group-header,
.group-section[data-kind="recent"] .group-header {
  border-top: none;
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
  overflow: hidden;
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
