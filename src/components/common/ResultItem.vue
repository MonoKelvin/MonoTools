<script setup lang="ts">
import { computed } from 'vue'
import type { SearchResult } from '@/types/search'
import { FileText } from 'lucide-vue-next'

const props = defineProps<{
  result: SearchResult
  active?: boolean
  index: number
}>()

const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
}>()

const IconComponent = computed(() => {
  const map: Record<string, typeof FileText> = {
    apps: FileText,
    files: FileText,
    commands: FileText,
    startup: FileText,
  }
  return map[props.result.category] || FileText
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
      <component :is="IconComponent" :size="16" :stroke-width="2.5" />
    </div>
    <div style="flex: 1; min-width: 0">
      <div class="result-title">{{ result.title }}</div>
      <div class="result-subtitle">{{ result.subtitle }}</div>
    </div>
    <div class="result-action">
      <span class="action-hint">Enter</span>
    </div>
  </div>
</template>

<style scoped>
.result-item {
  display: flex;
  align-items: center;
  padding: 9px 12px;
  border-radius: var(--radius-sm);
  gap: 12px;
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
  user-select: none;
}
.result-item:hover {
  background: rgba(255, 255, 255, 0.04);
}
:global(.theme-light) .result-item:hover {
  background: rgba(0, 0, 0, 0.03);
}
.result-item.is-active {
  background: var(--accent);
  color: white;
}
.result-item.is-active .result-subtitle {
  color: rgba(255, 255, 255, 0.8);
}
.result-item.is-active .result-icon {
  background: rgba(255, 255, 255, 0.2);
}
.result-item.is-active .action-hint {
  background: rgba(255, 255, 255, 0.2);
  color: rgba(255, 255, 255, 0.9);
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
  transition: background var(--duration-fast) var(--ease-out);
}
.result-title {
  font-size: 13.5px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.3;
}
.result-subtitle {
  font-size: 11px;
  color: var(--text-secondary);
  margin-top: 1px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.result-action {
  margin-left: auto;
  flex-shrink: 0;
}
.action-hint {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--text-tertiary);
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid var(--border);
  border-radius: 4px;
  transition: all var(--duration-fast) var(--ease-out);
}
:global(.theme-light) .action-hint {
  background: rgba(0, 0, 0, 0.03);
}
</style>
