<script setup lang="ts">
import { computed } from 'vue'
import type { SearchResult } from '@/types/search'
import { FileText, FolderOpen, Terminal } from "@lucide/vue"

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
  >
    <div class="result-icon">
      <component :is="IconComponent" :size="16" :stroke-width="2" />
    </div>
    <div class="result-text">
      <div class="result-title">{{ result.title }}</div>
      <div class="result-subtitle">{{ result.subtitle }}</div>
    </div>
    <div class="result-action">
      <span class="action-hint">⏎</span>
    </div>
  </div>
</template>

<style scoped>
.result-item {
  display: flex;
  align-items: center;
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  gap: 10px;
  cursor: pointer;
  transition: background var(--duration-fast) var(--ease-out);
  user-select: none;
}
.result-item:hover {
  background: var(--hairline-soft);
}
.result-item.is-active {
  background: var(--surface-card);
}

.result-icon {
  flex-shrink: 0;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-body);
  background: var(--surface-elevated);
  border: 1px solid var(--hairline-soft);
  transition: all var(--duration-fast) var(--ease-out);
}
.result-item.is-active .result-icon {
  background: var(--surface);
  color: var(--on-dark);
  border-color: var(--hairline);
}

.result-text {
  flex: 1;
  min-width: 0;
}

.result-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-ink);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.3;
  letter-spacing: 0.01em;
}

.result-subtitle {
  font-size: 11px;
  color: var(--text-ash);
  margin-top: 1px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 400;
}
.result-item.is-active .result-subtitle {
  color: var(--text-mute);
}

.result-action {
  margin-left: auto;
  flex-shrink: 0;
}

.action-hint {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 22px;
  padding: 0 5px;
  font-family: var(--font-mono);
  font-size: 11px;
  line-height: 20px;
  height: 20px;
  color: var(--text-mute);
  background: linear-gradient(180deg, var(--surface-card), var(--surface));
  border: 1px solid var(--hairline);
  border-radius: var(--radius-xs);
  transition: all var(--duration-fast) var(--ease-out);
}
.result-item.is-active .action-hint {
  color: var(--text-body);
}
</style>
