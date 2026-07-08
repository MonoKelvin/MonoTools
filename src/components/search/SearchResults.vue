<script setup lang="ts">
import type { SearchResult } from '@/types/search'

defineProps<{
  results: SearchResult[]
  loading?: boolean
  selectedIndex: number
}>()

defineEmits<{
  (e: 'select', item: SearchResult): void
  (e: 'hover', index: number): void
}>()
</script>

<template>
  <div class="results-wrapper" data-tauri-drag-region>
    <!-- Loading -->
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span class="loading-text">搜索中...</span>
    </div>

    <!-- Results -->
    <div v-else-if="results.length" class="results-list">
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

    <!-- Empty state -->
    <div v-else-if="$slots.empty" class="empty-state" data-tauri-drag-region>
      <slot name="empty" />
    </div>
  </div>
</template>

<style scoped>
.results-wrapper {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow-y: auto;
}

.results-list {
  display: flex;
  flex-direction: column;
  padding: var(--sp-1);
  gap: var(--sp-1);
}

.loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--sp-3);
  padding: var(--sp-8) var(--sp-5);
  flex: 1;
}
.spinner {
  width: 18px;
  height: 18px;
  border: 2.5px solid var(--border-default);
  border-top-color: var(--text-secondary);
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}
.loading-text {
  color: var(--text-tertiary);
  font-size: var(--text-sm);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-4);
  padding: var(--sp-8) var(--sp-5);
  flex: 1;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>