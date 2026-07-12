<script setup lang="ts">
/**
 * 极简虚拟滚动列表 —— 不依赖第三方 (支持百万条数据).
 *
 * 策略:
 *  - 容器固定高度 (height prop).
 *  - 内层轨道总高度 = items.length * itemHeight.
 *  - 计算可视区窗口 [startIdx, endIdx),仅渲染 (overscan + windowSize) 个实际节点.
 *  - 选中项不在可视窗时通过 `scrollToIndex` 平滑滚动到位.
 */
import { computed, nextTick, onMounted, onUpdated, ref, watch } from 'vue'
import type { SearchResult } from '@/types/search'
import ResultItem from '@/components/common/ResultItem.vue'

interface Props {
  results: SearchResult[]
  loading?: boolean
  selectedIndex: number
  height?: number
  itemHeight?: number
  overscan?: number
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  height: 360,
  itemHeight: 44,
  overscan: 8,
})

const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
  (e: 'hover', index: number): void
  (e: 'contextmenu', event: MouseEvent, item: SearchResult): void
}>()

const scrollerRef = ref<HTMLElement | null>(null)
const scrollTop = ref(0)

const total = computed(() => props.results.length)
const totalH = computed(() => total.value * props.itemHeight)
const windowRows = computed(() => Math.ceil(props.height / props.itemHeight) + props.overscan * 2)
const startIdx = computed(() => {
  const raw = Math.floor(scrollTop.value / props.itemHeight) - props.overscan
  return Math.max(0, raw)
})
const endIdx = computed(() => {
  return Math.min(total.value, startIdx.value + windowRows.value)
})
const offsetY = computed(() => startIdx.value * props.itemHeight)

/** 切片 + 关键行映射: 仅渲染可见区. */
const visibleItems = computed<Array<{ item: SearchResult; idx: number }>>(() => {
  const list = props.results
  const s = startIdx.value
  const e = endIdx.value
  const out: Array<{ item: SearchResult; idx: number }> = []
  for (let i = s; i < e; i++) {
    const it = list[i]
    if (it) out.push({ item: it, idx: i })
  }
  return out
})

const onScroll = (e: Event) => {
  scrollTop.value = (e.target as HTMLElement).scrollTop
}

/** 选中行不在可视区 → 平滑滚动到位. */
const scrollToIndex = async (idx: number) => {
  await nextTick()
  if (!scrollerRef.value) return
  if (idx < 0 || idx >= total.value) return
  const itemTop = idx * props.itemHeight
  const itemBottom = itemTop + props.itemHeight
  const viewTop = scrollerRef.value.scrollTop
  const viewBottom = viewTop + props.height
  if (itemTop < viewTop) {
    scrollerRef.value.scrollTo({ top: itemTop, behavior: 'smooth' })
  } else if (itemBottom > viewBottom) {
    scrollerRef.value.scrollTo({ top: itemBottom - props.height, behavior: 'smooth' })
  }
}

watch(() => props.selectedIndex, (v) => scrollToIndex(v))
watch(() => props.results, () => {
  // 数据集变化时, 滚动到顶部 (因为用户在重新搜索)
  if (scrollerRef.value) scrollerRef.value.scrollTop = 0
  scrollTop.value = 0
})

onMounted(() => {
  if (props.selectedIndex >= 0) scrollToIndex(props.selectedIndex)
})
onUpdated(() => {
  if (props.selectedIndex >= 0) scrollToIndex(props.selectedIndex)
})
</script>

<template>
  <div class="vlist" :style="{ height: height + 'px' }">
    <div v-if="loading" class="vlist__loading">
      <div class="vlist__spinner"></div>
      <span class="vlist__loading-text">搜索中...</span>
    </div>

    <div
      v-else-if="results.length"
      ref="scrollerRef"
      class="vlist__scroller"
      @scroll.passive="onScroll"
    >
      <!-- 整高轨道: 让浏览器 scrollTop 计算能对上 index. -->
      <div class="vlist__track" :style="{ height: totalH + 'px' }">
        <!-- 可见窗口: 偏移到 startIdx * itemHeight 处. -->
        <div
          class="vlist__window"
          :style="{ transform: `translateY(${offsetY}px)` }"
        >
          <ResultItem
            v-for="row in visibleItems"
            :key="row.item.id + ':' + row.idx"
            :result="row.item"
            :index="row.idx"
            :active="row.idx === selectedIndex"
            @select="emit('select', $event)"
            @mouseover="emit('hover', row.idx)"
            @contextmenu="(e, item) => emit('contextmenu', e, item)"
          />
        </div>
      </div>
    </div>

    <div v-else-if="$slots.empty" class="vlist__empty">
      <slot name="empty" />
    </div>
  </div>
</template>

<style scoped>
.vlist {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  position: relative;
  overflow: hidden;
}

.vlist__scroller {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  scroll-behavior: smooth;
}

.vlist__track {
  position: relative;
  width: 100%;
}

.vlist__window {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  display: flex;
  flex-direction: column;
  padding: var(--sp-2);
  gap: 2px;
  will-change: transform;
}

.vlist__loading,
.vlist__empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-4);
  padding: var(--sp-10) var(--sp-5);
  flex: 1;
  color: var(--text-tertiary);
}

.vlist__loading {
  flex-direction: row;
}

.vlist__spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--border-default);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: vlist-spin 0.8s linear infinite;
}

.vlist__loading-text {
  color: var(--text-tertiary);
  font-size: var(--text-sm);
}

@keyframes vlist-spin {
  to { transform: rotate(360deg); }
}

/* webkit 玻璃感滚动条 */
.vlist__scroller::-webkit-scrollbar {
  width: 8px;
}
.vlist__scroller::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.14);
  border-radius: 999px;
}
.vlist__scroller::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.24);
}
.vlist__scroller::-webkit-scrollbar-track {
  background: transparent;
}
</style>
