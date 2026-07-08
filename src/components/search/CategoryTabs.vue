<script setup lang="ts">
defineProps<{
  active: 'all' | 'apps' | 'files' | 'commands' | 'startup'
}>()

const emit = defineEmits<{
  (e: 'select', c: 'all' | 'apps' | 'files' | 'commands' | 'startup'): void
}>()

const tabs = [
  { id: 'all', label: '全部' },
  { id: 'apps', label: '应用' },
  { id: 'files', label: '文件' },
  { id: 'commands', label: '命令' },
  { id: 'startup', label: '启动项' },
] as const
</script>

<template>
  <div class="category-tabs">
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
  gap: 2px;
  padding: 6px 10px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.category-tab {
  padding: 4px 12px;
  font-size: 12px;
  color: var(--text-secondary);
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
  font-weight: 500;
}
.category-tab:hover {
  background: rgba(255, 255, 255, 0.05);
  color: var(--text-primary);
}
:global(.theme-light) .category-tab:hover {
  background: rgba(0, 0, 0, 0.04);
}
.category-tab.is-active {
  background: var(--accent-subtle);
  color: var(--accent);
  font-weight: 600;
}
</style>
