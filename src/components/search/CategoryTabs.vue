<script setup lang="ts">
defineProps<{
  active: 'all' | 'apps' | 'files' | 'commands'
}>()

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
  gap: 2px;
  padding: 4px 8px;
  border-top: 1px solid var(--hairline);
  border-bottom: 1px solid var(--hairline);
  background: transparent;
  flex-shrink: 0;
}
.category-tab {
  padding: 4px 10px;
  font-size: 12px;
  font-weight: 500;
  color: var(--text-body);
  background: transparent;
  border: none;
  border-radius: var(--radius-full);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
  letter-spacing: 0.01em;
}
.category-tab:hover {
  color: var(--text-ink);
  background: var(--hairline-soft);
}
.category-tab.is-active {
  background: var(--surface-elevated);
  color: var(--on-dark);
}
</style>
