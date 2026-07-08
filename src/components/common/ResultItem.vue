<script setup lang="ts">
import { computed } from 'vue'
import type { SearchResult } from '@/types/search'
import { FolderOpen, FileText, Terminal } from "@lucide/vue"

const props = defineProps<{
  result: SearchResult
  active?: boolean
  index: number
}>()

const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
}>()

const IconComponent = computed(() => {
  const map: Record<string, typeof FolderOpen> = {
    apps: FolderOpen,
    files: FileText,
    commands: Terminal,
  }
  return map[props.result.category] || FileText
})
</script>

<template>
  <div
    :class="['result-item', { 'is-active': active }]"
    @click="emit('select', result)"
    @mouseenter="$emit('mouseover', index)"
  >
    <div class="result-icon">
      <component :is="IconComponent" :size="18" :stroke-width="2" />
    </div>
    <div class="result-text">
      <div class="result-title">{{ result.title }}</div>
      <div v-if="result.subtitle" class="result-subtitle">{{ result.subtitle }}</div>
    </div>
    <div class="result-action">
      <span class="action-key">&#x23CE;</span>
    </div>
  </div>
</template>

<style scoped>
.result-item {
  display: flex;
  align-items: center;
  padding: var(--sp-3) var(--sp-4);
  gap: var(--sp-3);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--dur-fast) var(--ease-out), transform var(--dur-fast) var(--ease-out);
  user-select: none;
  margin: var(--sp-1);
}
.result-item:hover {
  background: var(--interactive-hover);
  transform: translateY(-1px);
}
.result-item.is-active {
  background: var(--interactive-active);
  transform: translateY(-1px);
}

.result-icon {
  flex-shrink: 0;
  width: 36px;
  height: 36px;
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-tertiary);
  background: var(--surface-overlay);
  border: 1px solid var(--border-subtle);
  transition: all var(--dur-fast) var(--ease-out);
}
.result-item.is-active .result-icon {
  color: var(--text-primary);
  border-color: var(--border-default);
  background: var(--surface-raised);
  box-shadow: 0 0 12px rgba(255, 255, 255, 0.05);
}

.result-text {
  flex: 1;
  min-width: 0;
}

.result-title {
  font-size: var(--text-base);
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: var(--leading-tight);
  letter-spacing: -0.01em;
}

.result-subtitle {
  font-size: var(--text-sm);
  color: var(--text-quaternary);
  margin-top: var(--sp-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 400;
}
.result-item.is-active .result-subtitle {
  color: var(--text-tertiary);
}

.result-action {
  flex-shrink: 0;
}

.action-key {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 28px;
  padding: 0 var(--sp-2);
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  line-height: 1.5;
  color: var(--text-quaternary);
  transition: color var(--dur-fast) var(--ease-out);
}
.result-item.is-active .action-key {
  color: var(--text-tertiary);
}
</style>