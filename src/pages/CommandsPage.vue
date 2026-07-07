<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { commandApi } from '@/services'
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
  <div class="commands-page">
    <header class="page-header">
      <div>
        <h1 class="page-title">自定义命令</h1>
        <p class="page-subtitle">通过关键字执行 Shell 命令或程序</p>
      </div>
      <button class="btn btn-primary" @click="showAdd = true">+ 添加</button>
    </header>

    <ul v-if="items.length" class="command-list">
      <li v-for="cmd in items" :key="cmd.id" class="command-card">
        <span class="command-icon">⚡</span>
        <div style="flex:1; min-width:0">
          <div class="command-name">{{ cmd.name }}</div>
          <div class="command-keyword mono">{{ cmd.keyword }}</div>
          <div class="command-cmd mono">
            {{ cmd.command + ' ' + cmd.args.join(' ') }}
          </div>
        </div>
        <button class="btn btn-ghost" @click="runCmd(cmd.id)">Run</button>
        <button class="btn-icon" @click="remove(cmd.id)" title="删除">✕</button>
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
          <label>
            <input v-model="form.runAsAdmin" type="checkbox" />
            以管理员权限运行
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
.commands-page {
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
}
.page-subtitle {
  margin: 4px 0 0 0;
  font-size: 13px;
  color: var(--text-secondary);
}
.command-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.command-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
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
  border-radius: 8px;
  background: rgba(255, 200, 0, 0.15);
  color: #fcc419;
  font-size: 18px;
}
.command-name {
  font-size: 14px;
  font-weight: 600;
}
.command-keyword {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 2px;
}
.command-cmd {
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 4px;
}
.empty {
  padding: 60px;
  text-align: center;
  color: var(--text-tertiary);
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
.form-check label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 24px;
}
.modal-title {
  margin: 0 0 16px 0;
  font-size: 20px;
}
</style>
