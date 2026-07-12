<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { commandApi } from '@/services'
import { useCommandsStore } from '@/commands'
import { Zap, Trash2, Plus, Play } from '@lucide/vue'
import type { CustomCommand } from '@/types/command'
import MtCard from '@/components/common/MtCard.vue'
import MtButton from '@/components/common/MtButton.vue'
import CheckButton from '@/components/common/CheckButton.vue'

const items = ref<CustomCommand[]>([])
const showAdd = ref(false)
const form = ref({
  name: '',
  keyword: '',
  command: '',
  args: '',
  runAsAdmin: false,
})

const commandsStore = useCommandsStore()

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
const remove = async (id: string) => {
  await commandApi.remove(id)
  await load()
}

onMounted(async () => {
  // 后端命令由 commands store 单独拉取，这里只管用户自定义命令
  await commandsStore.loadFromBackend().catch(() => undefined)
  await load()
})
</script>

<template>
  <div class="commands-panel">
    <div class="commands-panel__header">
      <div>
        <h1 class="commands-panel__title">自定义命令</h1>
        <p class="commands-panel__subtitle">通过关键字执行 Shell 命令或程序</p>
      </div>
      <MtButton variant="primary" @click="showAdd = true">
        <Plus :size="14" :stroke-width="2" />
        添加
      </MtButton>
    </div>

    <div v-if="items.length" class="commands-panel__list">
      <MtCard v-for="cmd in items" :key="cmd.id" class="commands-panel__card">
        <div class="commands-panel__card-icon">
          <Zap :size="16" :stroke-width="2" />
        </div>
        <div class="commands-panel__card-content">
          <div class="commands-panel__card-name">{{ cmd.name }}</div>
          <div class="commands-panel__card-keyword mono">{{ cmd.keyword }}</div>
          <div class="commands-panel__card-cmd mono">{{ cmd.command + ' ' + cmd.args.join(' ') }}</div>
        </div>
        <div class="commands-panel__card-actions">
          <MtButton variant="ghost" size="sm" @click="runCmd(cmd.id)">
            <Play :size="12" :stroke-width="2.5" />
            运行
          </MtButton>
          <button
            class="commands-panel__delete-btn"
            @click="remove(cmd.id)"
            v-tooltip="{ value: '删除此命令', showDelay: 280, position: 'top' }"
          >
            <Trash2 :size="14" :stroke-width="2" />
          </button>
        </div>
      </MtCard>
    </div>

    <div v-else class="commands-panel__empty">还没有自定义命令</div>

    <Transition name="scale">
      <div v-if="showAdd" class="commands-panel__modal-overlay" @click.self="showAdd = false">
        <div class="commands-panel__modal-card">
          <h2 class="commands-panel__modal-title">添加命令</h2>

          <div class="commands-panel__form-row">
            <label>显示名称</label>
            <input v-model="form.name" class="commands-panel__input" placeholder="git status" />
          </div>

          <div class="commands-panel__form-row">
            <label>搜索关键字</label>
            <input v-model="form.keyword" class="commands-panel__input" placeholder="git" />
          </div>

          <div class="commands-panel__form-row">
            <label>命令</label>
            <input v-model="form.command" class="commands-panel__input" placeholder="git" />
          </div>

          <div class="commands-panel__form-row">
            <label>参数</label>
            <input v-model="form.args" class="commands-panel__input" placeholder="status" />
          </div>

          <div class="commands-panel__form-row commands-panel__form-row--check">
            <label class="commands-panel__check-label">
              <CheckButton
                v-model="form.runAsAdmin"
                :size="16"
                class="commands-panel__check-btn"
              />
              <span>以管理员权限运行</span>
            </label>
          </div>

          <div class="commands-panel__modal-actions">
            <MtButton variant="ghost" @click="showAdd = false">取消</MtButton>
            <MtButton variant="primary" @click="submit">添加</MtButton>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.commands-panel {
  padding: var(--sp-5);
  overflow-y: auto;
  height: 100%;
  background: var(--canvas);
  display: flex;
  flex-direction: column;
  gap: var(--sp-4);
}

.commands-panel__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  gap: var(--sp-5);
  padding-bottom: var(--sp-2);
}

.commands-panel__title {
  margin: 0;
  font-size: var(--text-lg);
  font-weight: 600;
  letter-spacing: -0.01em;
  color: var(--text-primary);
}

.commands-panel__subtitle {
  margin: var(--sp-1) 0 0 0;
  font-size: var(--text-sm);
  color: var(--text-quaternary);
}

.commands-panel__list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
}

.commands-panel__card {
  display: flex;
  align-items: center;
  gap: var(--sp-4);
  padding: var(--sp-4);
}

.commands-panel__card-icon {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  background: var(--surface-raised);
  color: var(--text-tertiary);
  flex-shrink: 0;
}

.commands-panel__card-content {
  flex: 1;
  min-width: 0;
}

.commands-panel__card-name {
  font-size: var(--text-base);
  font-weight: 500;
  color: var(--text-primary);
  line-height: var(--leading-tight);
}

.commands-panel__card-keyword {
  font-size: var(--text-xs);
  color: var(--text-quaternary);
  margin-top: 2px;
}

.commands-panel__card-cmd {
  font-size: var(--text-sm);
  color: var(--text-tertiary);
  margin-top: 2px;
}

.commands-panel__card-actions {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  flex-shrink: 0;
}

.commands-panel__delete-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: var(--text-tertiary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
}

.commands-panel__delete-btn:hover {
  background: var(--color-danger-bg);
  color: var(--color-danger);
  border-color: var(--color-danger);
}

.commands-panel__empty {
  padding: var(--sp-10);
  text-align: center;
  color: var(--text-quaternary);
  font-size: var(--text-base);
}

.commands-panel__modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
}

.commands-panel__modal-card {
  width: 460px;
  max-width: 90vw;
  background: var(--surface);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-xl);
  padding: var(--sp-6);
}

.commands-panel__modal-title {
  font-size: var(--text-base);
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 var(--sp-5) 0;
  letter-spacing: -0.01em;
}

.commands-panel__form-row {
  margin-bottom: var(--sp-5);
}

.commands-panel__form-row label {
  display: block;
  font-size: var(--text-xs);
  font-weight: 500;
  color: var(--text-tertiary);
  margin-bottom: var(--sp-2);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.commands-panel__input {
  width: 100%;
  padding: var(--sp-3) var(--sp-4);
  background: var(--surface-raised);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-family: var(--font-sans);
  font-size: var(--text-base);
  outline: none;
  transition: all var(--dur-fast) var(--ease-out);
}

.commands-panel__input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-glow);
}

.commands-panel__input::placeholder {
  color: var(--text-quaternary);
}

.commands-panel__form-row--check {
  margin-top: var(--sp-2);
}

.commands-panel__check-label {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-3);
  cursor: pointer;
  font-size: var(--text-base);
  color: var(--text-secondary);
}

.commands-panel__check-btn {
  flex-shrink: 0;
}

.commands-panel__modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-3);
  margin-top: var(--sp-6);
}

.scale-enter-active,
.scale-leave-active {
  transition: all var(--dur-fast) var(--ease-out);
}

.scale-enter-from,
.scale-leave-to {
  opacity: 0;
}

.scale-enter-from .commands-panel__modal-card,
.scale-leave-to .commands-panel__modal-card {
  transform: scale(0.95);
}
</style>
