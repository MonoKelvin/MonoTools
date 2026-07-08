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

const runCmd = async (id: string) => await commandApi.run(id)
const remove = async (id: string) => { await commandApi.remove(id); await load() }

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
        <Plus :size="14" :stroke-width="2" />
        添加
      </button>
    </div>

    <ul v-if="items.length" class="command-list">
      <li v-for="cmd in items" :key="cmd.id" class="command-card">
        <div class="command-icon">
          <Zap :size="16" :stroke-width="2" />
        </div>
        <div class="command-text">
          <div class="command-name">{{ cmd.name }}</div>
          <div class="command-keyword mono">{{ cmd.keyword }}</div>
          <div class="command-cmd mono">{{ cmd.command + ' ' + cmd.args.join(' ') }}</div>
        </div>
        <button class="btn btn-ghost btn-sm" @click="runCmd(cmd.id)">
          <Play :size="12" :stroke-width="2.5" />
          运行
        </button>
        <button class="btn-icon" @click="remove(cmd.id)" title="删除">
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
  padding: var(--sp-6);
  overflow-y: auto;
  height: 100%;
  background: var(--canvas);
  color: var(--text-primary);
  display: flex;
  flex-direction: column;
  gap: var(--sp-4);
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  gap: var(--sp-6);
  padding-bottom: var(--sp-3);
}

.panel-title {
  margin: 0;
  font-size: var(--text-lg);
  font-weight: 600;
  letter-spacing: -0.01em;
  color: var(--text-primary);
}

.panel-subtitle {
  margin: var(--sp-2) 0 0 0;
  font-size: var(--text-sm);
  color: var(--text-quaternary);
  font-weight: 400;
}

.command-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
}

.command-card {
  display: flex;
  align-items: center;
  gap: var(--sp-4);
  padding: var(--sp-4) var(--sp-5);
  background: var(--surface);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-lg);
  transition: all var(--dur-fast) var(--ease-out);
}
.command-card:hover {
  border-color: var(--border-default);
  background: var(--surface-overlay);
}

.command-icon {
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  background: var(--surface-overlay);
  border: 1px solid var(--border-subtle);
  color: var(--text-tertiary);
  flex-shrink: 0;
}

.command-text {
  flex: 1;
  min-width: 0;
}

.command-name {
  font-size: var(--text-base);
  font-weight: 500;
  color: var(--text-primary);
  line-height: var(--leading-tight);
}

.command-keyword {
  font-size: var(--text-sm);
  color: var(--text-quaternary);
  margin-top: var(--sp-1);
}

.command-cmd {
  font-size: var(--text-sm);
  color: var(--text-tertiary);
  margin-top: var(--sp-1);
}

.empty {
  padding: var(--sp-12);
  text-align: center;
  color: var(--text-quaternary);
  font-size: var(--text-md);
}

/* Button styles */
.btn {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-3);
  padding: var(--sp-2) var(--sp-6);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-weight: 500;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all var(--dur-fast) var(--ease-out);
  line-height: 1.4;
  font-family: var(--font-sans);
}
.btn-primary {
  background: var(--accent);
  color: var(--canvas);
  font-weight: 600;
  border-color: var(--accent);
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}
.btn-primary:hover {
  background: var(--accent-hover);
  border-color: var(--accent-hover);
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.15);
}
.btn-ghost {
  background: transparent;
  color: var(--text-secondary);
  border-color: var(--border-default);
}
.btn-ghost:hover {
  background: var(--interactive-hover);
  color: var(--text-primary);
  border-color: var(--border-hover);
}
.btn-sm {
  padding: var(--sp-2) var(--sp-5);
  font-size: var(--text-xs);
}

.btn-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: var(--sp-2);
  background: transparent;
  color: var(--text-tertiary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
}
.btn-icon:hover {
  background: var(--color-danger-bg);
  color: var(--color-danger);
  border-color: var(--color-danger);
}

/* Modal */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: var(--surface-overlay);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
}

.modal-card {
  width: 460px;
  max-width: 90vw;
  background: var(--surface-raised);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-xl);
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.5);
  padding: var(--sp-8);
  transition: transform var(--dur-normal) var(--ease-out);
}

.modal-title {
  font-size: var(--text-base);
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 var(--sp-6) 0;
  letter-spacing: -0.01em;
}

.form-row {
  margin-bottom: var(--sp-6);
}
.form-row label {
  display: block;
  font-size: var(--text-xs);
  font-weight: 500;
  color: var(--text-tertiary);
  margin-bottom: var(--sp-3);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.input {
  width: 100%;
  padding: var(--sp-3) var(--sp-5);
  background: var(--surface);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-family: var(--font-sans);
  font-size: var(--text-md);
  outline: none;
  transition: border-color var(--dur-fast) var(--ease-out), box-shadow var(--dur-fast) var(--ease-out);
}
.input:focus {
  border-color: var(--border-active);
  box-shadow: 0 0 0 3px rgba(255, 255, 255, 0.05);
}
.input::placeholder {
  color: var(--text-quaternary);
}

.check-label {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-3);
  cursor: pointer;
  font-size: var(--text-md);
  color: var(--text-secondary);
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-3);
  margin-top: var(--sp-8);
}
</style>