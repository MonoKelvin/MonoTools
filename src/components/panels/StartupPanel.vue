<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useStartupStore } from '@/stores/startup'
import { Power, Trash2, RefreshCw, Plus, Search } from "@lucide/vue"
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
  <div class="panel">
    <div class="panel-header">
      <div>
        <h1 class="panel-title">启动项管理</h1>
        <p class="panel-subtitle">
          共 {{ headersCount.total }} 项 · 已启用 {{ headersCount.enabled }} ·
          已禁用 {{ headersCount.disabled }}
        </p>
      </div>
      <div class="panel-actions">
        <div class="search-mini">
          <Search :size="13" :stroke-width="2" class="search-mini-icon" />
          <input
            v-model="keyword"
            class="input input-mini"
            placeholder="搜索启动项..."
          />
        </div>
        <button class="btn btn-ghost" @click="refresh">
          <RefreshCw :size="13" :stroke-width="2" />
          刷新
        </button>
        <button class="btn btn-primary" @click="showAdd = true">
          <Plus :size="13" :stroke-width="2" />
          添加
        </button>
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
        <button class="btn btn-ghost btn-sm" @click="onToggle(item)">
          {{ item.enabled ? '禁用' : '启用' }}
        </button>
        <button class="btn-icon btn-sm" @click="store.remove(item.id)" title="删除">
          <Trash2 :size="14" :stroke-width="2" />
        </button>
      </li>
    </ul>

    <AddStartupModal v-if="showAdd" @close="showAdd = false" @added="onAdded" />
  </div>
</template>

<style scoped>
.panel {
  padding: 16px 20px;
  overflow-y: auto;
  height: 100%;
  background: var(--bg-primary);
  color: var(--text-primary);
}
.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  margin-bottom: 14px;
  gap: 12px;
}
.panel-title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  letter-spacing: 0.01em;
}
.panel-subtitle {
  margin: 3px 0 0 0;
  font-size: 12px;
  color: var(--text-secondary);
}
.panel-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
.search-mini {
  position: relative;
}
.search-mini-icon {
  position: absolute;
  left: 8px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--text-tertiary);
  pointer-events: none;
}
.input-mini {
  width: 150px !important;
  padding-left: 30px !important;
}
.filter-row {
  display: flex;
  gap: 4px;
  margin-bottom: 10px;
}
.filter-row .category-tab {
  padding: 5px 12px;
  font-size: 12px;
  color: var(--text-secondary);
  background: transparent;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}
.filter-row .category-tab:hover {
  background: rgba(255, 255, 255, 0.05);
  color: var(--text-primary);
}
.filter-row .category-tab.is-active {
  background: var(--accent-subtle);
  color: var(--accent);
  border-color: var(--accent);
  font-weight: 600;
}
.empty {
  text-align: center;
  padding: 40px;
  color: var(--text-tertiary);
  font-size: 13px;
}
.startup-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.startup-card {
  display: flex;
  align-items: center;
  padding: 10px 14px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  gap: 10px;
  transition: all var(--duration-fast) var(--ease-out);
}
.startup-card:hover {
  border-color: var(--border-hover);
  background: rgba(255, 255, 255, 0.02);
}
:global(.theme-light) .startup-card:hover {
  background: rgba(0, 0, 0, 0.01);
}
.startup-status {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.startup-status.is-enabled {
  background: var(--success);
  box-shadow: 0 0 6px rgba(81, 207, 102, 0.4);
}
.startup-status.is-disabled {
  background: var(--text-tertiary);
}
.startup-name {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.3;
}
.startup-cmd {
  font-family: var(--font-mono);
  font-size: 11.5px;
  color: var(--text-secondary);
  margin-top: 3px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.startup-meta {
  display: flex;
  gap: 6px;
  margin-top: 6px;
}
.badge {
  display: inline-block;
  padding: 1px 7px;
  font-size: 10px;
  background: var(--bg-tertiary);
  border-radius: 3px;
  color: var(--text-secondary);
  line-height: 1.6;
}
</style>
