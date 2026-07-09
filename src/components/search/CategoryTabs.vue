<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  active: 'all' | 'apps' | 'files' | 'commands'
}

const props = withDefaults(defineProps<Props>(), {})

const emit = defineEmits<{
  (e: 'select', c: 'all' | 'apps' | 'files' | 'commands'): void
}>()

const tabs = [
  { id: 'all', label: '全部', icon: '' },
  { id: 'apps', label: '应用', icon: '' },
  { id: 'files', label: '文件', icon: '' },
  { id: 'commands', label: '命令', icon: '' },
] as const

const activeIndex = computed(() => tabs.findIndex(t => t.id === props.active))
</script>

<template>
  <div class="category-tabs" data-tauri-drag-region>
    <div class="category-tabs__list">
      <button
        v-for="t in tabs"
        :key="t.id"
        :class="['category-tabs__tab', { 'category-tabs__tab--active': active === t.id }]"
        @click="emit('select', t.id)"
      >
        {{ t.label }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.category-tabs {
  display: flex;
  align-items: center;
  padding: var(--sp-3) var(--sp-5);
  height: 48px;
  background: var(--surface);
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.category-tabs__list {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

.category-tabs__tab {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--sp-2);
  padding: var(--sp-2) var(--sp-4);
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--text-tertiary);
  background: transparent;
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
  letter-spacing: 0.02em;
  white-space: nowrap;
}

.category-tabs__tab:hover {
  color: var(--text-secondary);
  background: var(--surface-hover);
}

.category-tabs__tab--active {
  color: var(--text-primary);
  background: var(--surface-overlay);
}
</style>
