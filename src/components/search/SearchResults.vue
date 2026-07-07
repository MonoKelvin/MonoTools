<script setup lang="ts">
import ResultItem from '@/components/common/ResultItem.vue'
import type { SearchResult } from '@/types/search'

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
  <div class="search-results" v-if="results.length || loading">
    <div v-if="loading" class="empty">
      <div class="spinner"></div>
      <span>搜索中...</span>
    </div>
    <template v-else>
      <ResultItem
        v-for="(item, idx) in results"
        :key="item.id"
        :result="item"
        :index="idx"
        :active="idx === selectedIndex"
        @select="emit('select', $event)"
        @mouseover="emit('hover', idx)"
      />
    </template>
  </div>
  <div v-else-if="$slots.empty" class="empty">
    <slot name="empty" />
  </div>
</template>

<style scoped>
.search-results {
  flex: 1;
  overflow-y: auto;
  min-height: 200px;
  padding: 6px;
}
.empty {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 60px 20px;
  color: var(--text-tertiary);
  font-size: 13px;
}
.spinner {
  width: 16px;
  height: 16px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
