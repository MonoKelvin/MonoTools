<script setup lang="ts">
import ResultItem from '@/components/common/ResultItem.vue'
import type { SearchResult } from '@/types/search'
import { Search } from '@lucide/vue'

defineProps<{
  results: SearchResult[]
  loading?: boolean
  selectedIndex: number
}>()

const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
  (e: 'hover', index: number): void
}>()
</script>

<template>
  <div class="search-results-wrapper" data-tauri-drag-region v-if="results.length || loading">
    <div v-if="loading" class="empty">
      <div class="spinner"></div>
      <span class="dim">搜索中...</span>
    </div>
    <div v-else class="search-results">
      <ResultItem
        v-for="(item, idx) in results"
        :key="item.id"
        :result="item"
        :index="idx"
        :active="idx === selectedIndex"
        @select="emit('select', $event)"
        @mouseover="emit('hover', idx)"
      />
    </div>
  </div>
  <div v-else-if="$slots.empty" class="empty" data-tauri-drag-region>
    <slot name="empty" />
  </div>
</template>

<style scoped>
.search-results-wrapper {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.search-results {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  padding: 4px 6px 6px;
}

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 40px 20px;
  color: var(--text-mute);
  font-size: 13px;
  flex: 1;
}
.spinner {
  width: 18px;
  height: 18px;
  border: 2px solid var(--hairline);
  border-top-color: var(--on-dark);
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
