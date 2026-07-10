<script setup lang="ts">
import { ref, watch, nextTick, onUpdated } from 'vue'
import type { SearchResult } from '@/types/search'
import ResultItem from '@/components/common/ResultItem.vue'

interface Props {
  results: SearchResult[]
  loading?: boolean
  selectedIndex: number
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
})

defineEmits<{
  (e: 'select', item: SearchResult): void
  (e: 'hover', index: number): void
  (e: 'contextmenu', event: MouseEvent, item: SearchResult): void
}>()

const listRef = ref<HTMLElement | null>(null)

const scrollToActive = async () => {
  await nextTick()
  if (!listRef.value) return

  const items = listRef.value.querySelectorAll('.result-item')
  const activeItem = items[props.selectedIndex] as HTMLElement | undefined
  if (!activeItem) return

  activeItem.scrollIntoView({
    behavior: 'smooth',
    block: 'nearest',
  })
}

watch(() => props.selectedIndex, scrollToActive)

onUpdated(() => {
  if (props.selectedIndex >= 0) {
    scrollToActive()
  }
})
</script>

<template>
  <div class="search-results">
    <div v-if="loading" class="search-results__loading">
      <div class="search-results__spinner"></div>
      <span class="search-results__loading-text">搜索中...</span>
    </div>

    <TransitionGroup v-else-if="results.length" name="list" tag="div" class="search-results__list" ref="listRef">
      <ResultItem
        v-for="(item, idx) in results"
        :key="item.id"
        :result="item"
        :index="idx"
        :active="idx === selectedIndex"
        @select="emit('select', $event)"
        @mouseover="emit('hover', idx)"
        @contextmenu="(e, item) => emit('contextmenu', e, item)"
      />
    </TransitionGroup>

    <div v-else-if="$slots.empty" class="search-results__empty">
      <slot name="empty" />
    </div>
  </div>
</template>

<style scoped>
.search-results {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow-y: auto;
}

.search-results__list {
  display: flex;
  flex-direction: column;
  padding: var(--sp-2);
  gap: 2px;
}

.search-results__loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--sp-4);
  padding: var(--sp-10) var(--sp-5);
  flex: 1;
}

.search-results__spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--border-default);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: search-results-spin 0.8s linear infinite;
}

.search-results__loading-text {
  color: var(--text-tertiary);
  font-size: var(--text-sm);
}

.search-results__empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-4);
  padding: var(--sp-10) var(--sp-5);
  flex: 1;
}

.list-enter-active,
.list-leave-active {
  transition: all var(--dur-normal) var(--ease-out);
}

.list-enter-from {
  opacity: 0;
  transform: translateX(-12px);
}

.list-leave-to {
  opacity: 0;
  transform: translateX(12px);
}

.list-move {
  transition: transform var(--dur-normal) var(--ease-out);
}

@keyframes search-results-spin {
  to { transform: rotate(360deg); }
}
</style>
