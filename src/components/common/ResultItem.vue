<script setup lang="ts">
import { computed } from 'vue'
import type { SearchResult } from '@/types/search'

const props = defineProps<{
  result: SearchResult
  active?: boolean
  index: number
}>()

const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
}>()

const iconChar = computed(() => {
  const m = props.result.title?.[0] || '?'
  return m.toUpperCase()
})

const actionLabel = computed(() => {
  const a = props.result.action
  switch (a.type) {
    case 'launch':
      return '⏎ Open'
    case 'open':
      return '⏎ Open File'
    case 'run':
      return '⏎ Run'
    case 'navigate':
      return '⏎ Go'
    default:
      return ''
  }
})

const color = computed(() => {
  switch (props.result.category) {
    case 'apps':
      return '#339af0'
    case 'files':
      return '#51cf66'
    case 'commands':
      return '#fcc419'
    case 'startup':
      return '#ff6b6b'
    default:
      return '#6c6c7e'
  }
})
</script>

<template>
  <div
    :class="['result-item', { 'is-active': active }]"
    @click="emit('select', result)"
  >
    <div class="result-icon" :style="{ background: color }">
      {{ iconChar }}
    </div>
    <div style="flex: 1; min-width: 0">
      <div class="result-title">{{ result.title }}</div>
      <div class="result-subtitle">{{ result.subtitle }}</div>
    </div>
    <div class="result-action">{{ actionLabel }}</div>
  </div>
</template>

<style scoped>
.result-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  gap: 12px;
  cursor: pointer;
  transition: background var(--duration-fast) var(--ease-out);
  user-select: none;
}
.result-item:hover {
  background: rgba(255, 255, 255, 0.05);
}
.result-item.is-active {
  background: var(--accent-subtle);
  border-left: 2px solid var(--accent);
  padding-left: 10px;
}
.result-icon {
  flex-shrink: 0;
  width: 36px;
  height: 36px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  font-size: 16px;
  font-weight: 600;
}
.result-title {
  font-size: 14px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.result-subtitle {
  font-size: 11px;
  color: var(--text-secondary);
  margin-top: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.result-action {
  margin-left: auto;
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: var(--font-mono);
}
</style>
