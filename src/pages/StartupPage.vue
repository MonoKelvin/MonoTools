<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useStartupStore } from '@/stores/startup'
import AddStartupModal from '@/components/startup/AddStartupModal.vue'

const store = useStartupStore()

function sourceLabel(s: string): string {
  switch (s) {
    case 'registryRun':
      return 'HKCU Run'
    case 'registryRunOnce':
      return 'RunOnce'
    case 'startupFolder':
      return '启动文件夹'
    case 'scheduledTask':
      return '计划任务'
    case 'custom':
      return '自定义'
    default:
      return s
  }
}

const filter = ref<'all' | 'enabled' | 'disabled'>('all')
const keyword = ref('')
const showAdd = ref(false)

const filteredItems = computed(() => {
  let items = store.items.slice()
  if (filter.value === 'enabled') items = items.filter((i) => i.enabled)
  else if (filter.value === 'disabled') items = items.filter((i) => !i.enabled)
  const k = keyword.value.trim().toLowerCase()
  if (k) items = items.filter((i) => i.name.toLowerCase().includes(k) || i.command.toLowerCase().includes(k))
  return items
})

const headersCount = computed(() => ({
  total: store.items.length,
  enabled: store.items.filter((i) => i.enabled).length,
  disabled: store.items.filter((i) => !i.enabled).length,
}))

const refresh = () => store.refresh()

const onToggle = (item: any) => {
  store.toggle(item.id, !item.enabled)
}

function onAdded() {
  showAdd.value = false
  refresh()
}

onMounted(() => {
  refresh()
})
</script>

<template>
  <div class="startup-page">
    <div class="page-header">
      <div>
        <h1 class="page-title">启动项管理</h1>
        <p class="page-subtitle">
          共 {{ headersCount.total }} 项 · 已启用 {{ headersCount.enabled }} ·
          已禁用 {{ headersCount.disabled }}
        </p>
      </div>
      <div class="page-actions">
        <input
          v-model="keyword"
          class="input"
          placeholder="搜索启动项..."
          style="width: 220px"
        />
        <button class="btn btn-ghost" @click="refresh">↻ 刷新</button>
        <button class="btn btn-primary" @click="showAdd = true">+ 添加</button>
      </div>
    </div>

    <div class="filter-row">
      <button
        :class="['category-tab', { 'is-active': filter === 'all' }]"
        @click="filter = 'all'"
      >
        全部
      </button>
      <button
        :class="['category-tab', { 'is-active': filter === 'enabled' }]"
        @click="filter = 'enabled'"
      >
        已启用
      </button>
      <button
        :class="['category-tab', { 'is-active': filter === 'disabled' }]"
        @click="filter = 'disabled'"
      >
        已禁用
      </button>
    </div>

    <div v-if="store.loading" class="empty">载入中…</div>
    <div v-else-if="filteredItems.length === 0" class="empty">
      没有匹配项。
    </div>
    <ul v-else class="startup-list">
      <li
        v-for="item in filteredItems"
        :key="item.id"
        class="startup-card"
      >
        <span :class="['startup-status', item.enabled ? 'is-enabled' : 'is-disabled']"></span>
        <div style="flex: 1; min-width: 0">
          <div class="startup-name">{{ item.name }}</div>
          <div class="startup-cmd" :title="item.command">{{ item.command }}</div>
          <div class="startup-meta">
            <span class="badge">{{ sourceLabel(item.source) }}</span>
            <span v-if="item.delaySeconds" class="badge">延迟 {{ item.delaySeconds }}s</span>
            <span v-if="item.runAsAdmin" class="badge">管理员</span>
          </div>
        </div>
        <button class="btn btn-ghost" @click="onToggle(item)">
          {{ item.enabled ? '禁用' : '启用' }}
        </button>
        <button class="btn-icon" @click="store.remove(item.id)" title="删除">
          ✕
        </button>
      </li>
    </ul>

    <AddStartupModal v-if="showAdd" @close="showAdd = false" @added="onAdded" />
  </div>
</template>

<style scoped>
.startup-page {
  padding: 32px 40px;
  height: 100%;
  overflow-y: auto;
  background: var(--bg-primary);
  color: var(--text-primary);
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  margin-bottom: 24px;
}
.page-title {
  margin: 0;
  font-size: 28px;
  font-weight: 600;
}
.page-subtitle {
  margin: 4px 0 0 0;
  font-size: 13px;
  color: var(--text-secondary);
}
.page-actions {
  display: flex;
  gap: 8px;
}
.filter-row {
  display: flex;
  gap: 4px;
  margin-bottom: 16px;
}
.filter-row .category-tab {
  padding: 6px 12px;
  font-size: 12px;
  color: var(--text-secondary);
  background: transparent;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.filter-row .category-tab:hover {
  background: var(--border);
  color: var(--text-primary);
}
.filter-row .category-tab.is-active {
  background: var(--accent-subtle);
  color: var(--accent);
  border-color: var(--accent);
}
.empty {
  text-align: center;
  padding: 60px;
  color: var(--text-tertiary);
}
.startup-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.startup-card {
  display: flex;
  align-items: center;
  padding: 14px 18px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  gap: 12px;
  transition: border var(--duration-fast) var(--ease-out);
}
.startup-card:hover {
  border-color: var(--border-hover);
}
.startup-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}
.startup-cmd {
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.startup-meta {
  display: flex;
  gap: 6px;
  margin-top: 8px;
}
.badge {
  display: inline-block;
  padding: 2px 8px;
  font-size: 10px;
  background: var(--bg-tertiary);
  border-radius: 4px;
  color: var(--text-secondary);
}
</style>
