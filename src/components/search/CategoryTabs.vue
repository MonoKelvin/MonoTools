<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  active: 'all' | 'apps' | 'files' | 'commands'
}>(), {})

const emit = defineEmits<{
  (e: 'select', c: 'all' | 'apps' | 'files' | 'commands'): void
}>()

const tabs = [
  { id: 'all', label: '全部' },
  { id: 'apps', label: '应用' },
  { id: 'files', label: '文件' },
  { id: 'commands', label: '命令' },
] as const
</script>

<template>
  <div class="category-tabs" data-tauri-drag-region>
    <button
      v-for="t in tabs"
      :key="t.id"
      :class="['category-tab', { 'is-active': active === t.id }]"
      @click="emit('select', t.id)"
    >
      {{ t.label }}
    </button>
  </div>
</template>

<style scoped>
.category-tabs {
  display: flex;
  align-items: center;
  gap: var(--sp-1);
  padding: var(--sp-3) var(--sp-4);
  border-top: 1px solid var(--border-subtle);
  border-bottom: 1px solid var(--border-subtle);
  background: var(--surface-raised);
  flex-shrink: 0;
}

.category-tab {
  padding: var(--sp-2) var(--sp-5);
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--text-tertiary);
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
  letter-spacing: 0.02em;
  line-height: 1.5;
  position: relative;
}
.category-tab:hover {
  color: var(--text-secondary);
  background: var(--interactive-hover);
}
.category-tab.is-active {
  color: var(--accent);
  background: var(--interactive-active);
}

/* Active indicator line */
.category-tab.is-active::after {
  content: '';
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--accent);
  border-radius: 2px 2px 0 0;
}
</style>