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
  items.value = (await commandApi.list()) as CustomCommand[]
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
  <div class="panel" data-tauri-drag-region>
    <div class="panel-header">
      <div>
        <h1 class="panel-title">自定义命令</h1>
        <p class="panel-subtitle">通过关键字执行 Shell 命令或程序</p>
      </div>
      <button class="btn btn-primary" @click="showAdd = true">
        <Plus :size="13" :stroke-width="2" />
        添加
      </button>
    </div>

    <ul v-if="items.length" class="command-list">
      <li v-for="cmd in items" :key="cmd.id" class="command-card">
        <div class="command-icon">
          <Zap :size="14" :stroke-width="2" />
        </div>
        <div class="command-text">
          <div class="command-name">{{ cmd.name }}</div>
          <div class="command-keyword mono">{{ cmd.keyword }}</div>
          <div class="command-cmd mono">{{ cmd.command + ' ' + cmd.args.join(' ') }}</div>
        </div>
        <button class="btn btn-ghost btn-sm" @click="runCmd(cmd.id)">
          <Play :size="11" :stroke-width="2.5" />
          运行
        </button>
        <button class="btn-icon btn-sm" @click="remove(cmd.id)" title="删除">
          <Trash2 :size="13" :stroke-width="2" />
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
  padding: 14px 16px;
  overflow-y: auto;
  height: 100%;
  background: var(--canvas);
  color: var(--text-ink);
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  gap: 12px;
}
.panel-title {
  margin: 0;
  font-size: 17px;
  font-weight: 600;
  letter-spacing: -0.005em;
  color: var(--text-ink);
}
.panel-subtitle {
  margin: 3px 0 0 0;
  font-size: 12px;
  color: var(--text-mute);
  font-weight: 400;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 5px 12px;
  border-radius: var(--radius-md);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all var(--duration-fast) var(--ease-out);
}
.btn-sm {
  padding: 4px 10px;
  font-size: 11.5px;
}
.btn-primary {
  background: var(--primary);
  color: var(--on-primary);
  font-weight: 600;
}
.btn-primary:hover {
  background: var(--primary-pressed);
}
.btn-ghost {
  background: transparent;
  color: var(--text-body);
  border-color: var(--hairline);
}
.btn-ghost:hover {
  background: var(--surface-elevated);
  color: var(--on-dark);
  border-color: var(--hairline-strong);
}
.btn-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 5px;
  background: transparent;
  color: var(--text-mute);
  border: 1px solid var(--hairline);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}
.btn-icon:hover {
  background: var(--surface-elevated);
  color: var(--accent-red);
  border-color: var(--hairline-strong);
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
  padding: 10px 12px;
  background: var(--surface);
  border: 1px solid var(--hairline);
  border-radius: var(--radius-lg);
  transition: all var(--duration-fast) var(--ease-out);
}
.command-card:hover {
  border-color: var(--hairline-strong);
}
.command-icon {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  background: var(--surface-elevated);
  border: 1px solid var(--hairline-soft);
  color: var(--text-body);
  flex-shrink: 0;
}
.command-text {
  flex: 1;
  min-width: 0;
}
.command-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-ink);
  line-height: 1.3;
}
.command-keyword {
  font-size: 11px;
  color: var(--text-ash);
  margin-top: 1px;
}
.command-cmd {
  font-size: 11.5px;
  color: var(--text-body);
  margin-top: 2px;
}

.empty {
  padding: 40px;
  text-align: center;
  color: var(--text-ash);
  font-size: 13px;
}

.input {
  width: 100%;
  padding: 8px 12px;
  background: var(--surface);
  border: 1px solid var(--hairline);
  border-radius: var(--radius-md);
  color: var(--text-ink);
  font-family: var(--font-sans);
  font-size: 13px;
  outline: none;
  transition: border-color var(--duration-fast) var(--ease-out);
}
.input:focus {
  border-color: var(--hairline-strong);
}

.form-row {
  margin-bottom: 12px;
}
.form-row label {
  display: block;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-mute);
  margin-bottom: 4px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}
.form-check .check-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  font-size: 13px;
  color: var(--text-ink);
}

.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
}
.modal-card {
  width: 440px;
  max-width: 90vw;
  background: var(--surface-elevated);
  border: 1px solid var(--hairline);
  border-radius: var(--radius-xl);
  padding: 22px 22px 18px;
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.65);
}
.modal-title {
  margin: 0 0 16px 0;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-ink);
  letter-spacing: 0.005em;
}
.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 18px;
}
</style>
