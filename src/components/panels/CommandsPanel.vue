<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { commandApi } from '@/services'
import { Zap, Trash2, Plus, Play } from "@lucide/vue"
import type { CustomCommand } from '@/types/command'

const items = ref<CustomCommand[]>([])
const showAdd = ref(false)
const form = ref({
  name: '',
  keyword: '',
  command: '',
  args: '',
  runAsAdmin: false,
})

const load = async () => {
  items.value = await commandApi.list() as CustomCommand[]
}

const submit = async () => {
  if (!form.value.name.trim() || !form.value.command.trim()) return
  await commandApi.add({
    id: crypto.randomUUID(),
    name: form.value.name.trim(),
    description: null,
    keyword: form.value.keyword.trim() || form.value.name.trim().toLowerCase(),
    command: form.value.command.trim(),
    args: form.value.args.split(/\s+/).filter(Boolean),
    workingDir: null,
    icon: null,
    category: 'Custom',
    enabled: true,
    runAsAdmin: form.value.runAsAdmin,
    createdAt: Math.floor(Date.now() / 1000),
    lastUsedAt: null,
  })
  form.value = { name: '', keyword: '', command: '', args: '', runAsAdmin: false }
  showAdd.value = false
  await load()
}

const runCmd = async (id: string) => {
  await commandApi.run(id)
}

const remove = async (id: string) => {
  await commandApi.remove(id)
  await load()
}

onMounted(load)
</script>

<template>
  <div class="panel">
    <div class="panel-header">
      <div>
        <h1 class="panel-title">自定义命令</h1>
        <p class="panel-subtitle">通过关键字执行 Shell 命令或程序</p>
      </div>
      <button class="btn btn-primary" @click="showAdd = true">
        <Plus :size="14" :stroke-width="2" />
        添加
      </button>
    </div>

    <ul v-if="items.length" class="command-list">
      <li v-for="cmd in items" :key="cmd.id" class="command-card">
        <div class="command-icon">
          <Zap :size="16" :stroke-width="2.5" />
        </div>
        <div style="flex:1; min-width:0">
          <div class="command-name">{{ cmd.name }}</div>
          <div class="command-keyword mono">{{ cmd.keyword }}</div>
          <div class="command-cmd mono">{{ cmd.command + ' ' + cmd.args.join(' ') }}</div>
        </div>
        <button class="btn btn-ghost btn-sm" @click="runCmd(cmd.id)">
          <Play :size="12" :stroke-width="2.5" />
          运行
        </button>
        <button class="btn-icon btn-sm" @click="remove(cmd.id)" title="删除">
          <Trash2 :size="14" :stroke-width="2" />
        </button>
      </li>
    </ul>

    <div v-else class="empty">还没有自定义命令</div>

    <div v-if="showAdd" class="modal-overlay" @click.self="showAdd = false">
      <div class="modal-card">
        <h2 class="modal-title">添加命令</h2>
        <div class="form-row">
          <label>显示名称</label>
          <input v-model="form.name" class="input" placeholder="git status" />
        </div>
        <div class="form-row">
          <label>搜索关键字</label>
          <input v-model="form.keyword" class="input" placeholder="git" />
        </div>
        <div class="form-row">
          <label>命令</label>
          <input v-model="form.command" class="input" placeholder="git" />
        </div>
        <div class="form-row">
          <label>参数</label>
          <input v-model="form.args" class="input" placeholder="status" />
        </div>
        <div class="form-row form-check">
          <label class="check-label">
            <input v-model="form.runAsAdmin" type="checkbox" />
            <span>以管理员权限运行</span>
          </label>
        </div>
        <div class="modal-actions">
          <button class="btn btn-ghost" @click="showAdd = false">取消</button>
          <button class="btn btn-primary" @click="submit">添加</button>
        </div>
      </div>
    </div>
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
.command-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.command-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  transition: all var(--duration-fast) var(--ease-out);
}
.command-card:hover {
  border-color: var(--border-hover);
}
.command-icon {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  background: rgba(252, 196, 25, 0.12);
  color: #fcc419;
  flex-shrink: 0;
}
.command-name {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.3;
}
.command-keyword {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 1px;
}
.command-cmd {
  font-size: 11.5px;
  color: var(--text-secondary);
  margin-top: 2px;
}
.empty {
  padding: 40px;
  text-align: center;
  color: var(--text-tertiary);
  font-size: 13px;
}
.form-row {
  margin-bottom: 12px;
}
.form-row label {
  display: block;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}
.form-check .check-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  font-size: 13px;
  color: var(--text-primary);
}
.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 20px;
}
.modal-title {
  margin: 0 0 14px 0;
  font-size: 16px;
  font-weight: 600;
}
</style>
