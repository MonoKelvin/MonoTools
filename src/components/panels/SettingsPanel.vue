<script setup lang="ts">
import { onMounted, ref, watch, computed } from 'vue'
import { useSettingsStore } from '@/stores/settings'
import { hotkeyApi, pinTopApi, isTauri } from '@/services'
import { Keyboard, FolderSearch, Info, Pin, PinOff, Monitor } from "@lucide/vue"

const settingsStore = useSettingsStore()

const newKey = ref('')
const recording = ref(false)
const message = ref('')

const pinToTop = ref(true)
const savingPin = ref(false)

const hotkey = computed({
  get: () => newKey.value || settingsStore.settings.hotkey,
  set: (v: string) => (newKey.value = v),
})

async function reloadPin() {
  if (!isTauri) return
  try {
    pinToTop.value = await pinTopApi.get()
  } catch {
    pinToTop.value = true
  }
}

async function togglePin(value: boolean) {
  pinToTop.value = value
  savingPin.value = true
  try {
    await settingsStore.update({ pinToTop: value })
    if (isTauri) await pinTopApi.set(value)
  } finally {
    savingPin.value = false
  }
}

function startRecord() {
  recording.value = true
  message.value = '请按下组合键（Esc 取消）'
}

function onRecordKey(e: KeyboardEvent) {
  if (!recording.value) return
  e.preventDefault()
  e.stopPropagation()
  if (e.key === 'Escape') {
    recording.value = false
    message.value = ''
    return
  }
  const parts: string[] = []
  if (e.ctrlKey) parts.push('Ctrl')
  if (e.altKey) parts.push('Alt')
  if (e.shiftKey) parts.push('Shift')
  if (e.metaKey) parts.push('Meta')
  let key = e.key
  if (key.length === 1) key = key.toUpperCase()
  parts.push(key)
  newKey.value = parts.join('+')
  recording.value = false
  message.value = `已识别：${newKey.value}`
}

async function save() {
  await settingsStore.update({
    hotkey: newKey.value.trim() || settingsStore.settings.hotkey,
  })
  hotkeyApi.register(hotkey.value).catch(() => {})
  message.value = '已保存'
  setTimeout(() => (message.value = ''), 1500)
}

watch(pinToTop, (v) => {
  if (v !== settingsStore.settings.pinToTop) togglePin(v)
})

onMounted(async () => {
  await settingsStore.load()
  newKey.value = settingsStore.settings.hotkey
  pinToTop.value = settingsStore.settings.pinToTop
  await reloadPin()
  document.addEventListener('keydown', onRecordKey)
})
</script>

<template>
  <div class="panel" data-tauri-drag-region>
    <section class="card">
      <div class="card-header">
        <Monitor :size="14" :stroke-width="1.8" />
        <h2>外观</h2>
      </div>
      <p class="card-hint">当前始终使用深色主题（黑/白/灰，Raycast 设计语言）。</p>
    </section>

    <section class="card">
      <div class="card-header">
        <Pin :size="14" :stroke-width="1.8" />
        <h2>窗口行为</h2>
      </div>
      <div class="row">
        <div class="row-info">
          <div class="row-label">窗口置顶</div>
          <div class="row-desc">开启后，搜索窗口会在所有应用之上</div>
        </div>
        <label class="toggle">
          <input
            type="checkbox"
            :checked="pinToTop"
            :disabled="savingPin"
            @change="togglePin(($event.target as HTMLInputElement).checked)"
          />
          <span class="toggle-track">
            <span class="toggle-thumb"></span>
          </span>
        </label>
      </div>
    </section>

    <section class="card">
      <div class="card-header">
        <Keyboard :size="14" :stroke-width="1.8" />
        <h2>全局快捷键</h2>
      </div>
      <div class="row">
        <div class="row-info">
          <div class="row-label">唤起快捷键</div>
          <div class="row-desc">按下此组合键显示 / 隐藏窗口</div>
        </div>
        <div class="row-actions">
          <kbd class="hotkey-chip">{{ hotkey }}</kbd>
          <button class="btn btn-ghost" type="button" @click="startRecord">
            {{ recording ? '按下新组合键…' : '录制' }}
          </button>
        </div>
      </div>
      <p v-if="message" class="hint">{{ message }}</p>
      <div class="row">
        <button class="btn btn-primary" @click="save">保存</button>
      </div>
    </section>

    <section class="card">
      <div class="card-header">
        <FolderSearch :size="14" :stroke-width="1.8" />
        <h2>文件搜索</h2>
      </div>
      <div class="row">
        <div class="row-info">
          <div class="row-label">启用文件搜索</div>
          <div class="row-desc">使用 SQLite FTS5 索引文件名</div>
        </div>
        <label class="toggle">
          <input
            type="checkbox"
            :checked="settingsStore.settings.fileSearchEnabled"
            @change="settingsStore.update({ fileSearchEnabled: ($event.target as HTMLInputElement).checked })"
          />
          <span class="toggle-track">
            <span class="toggle-thumb"></span>
          </span>
        </label>
      </div>
    </section>

    <section class="card">
      <div class="card-header">
        <Info :size="14" :stroke-width="1.8" />
        <h2>关于</h2>
      </div>
      <p class="about">
        MonoTools v0.1.0<br />
        轻量级系统效率工具 · 启动项管理 + 全局搜索
      </p>
    </section>
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
  gap: 10px;
}

.card {
  background: var(--surface);
  border: 1px solid var(--hairline);
  border-radius: var(--radius-lg);
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-mute);
}
.card-header h2 {
  margin: 0;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-mute);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}
.card-hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-body);
  line-height: 1.5;
}

.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.row-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.row-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-ink);
}
.row-desc {
  font-size: 11.5px;
  color: var(--text-ash);
  line-height: 1.5;
}
.row-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.hotkey-chip {
  display: inline-flex;
  align-items: center;
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 500;
  padding: 1px 8px;
  height: 24px;
  color: var(--text-body);
  background: linear-gradient(180deg, var(--surface-card), var(--surface));
  border: 1px solid var(--hairline);
  border-radius: var(--radius-xs);
}

.hint {
  font-size: 12px;
  color: var(--accent-blue);
  margin: 0;
}
.about {
  margin: 0;
  font-size: 12px;
  color: var(--text-body);
  line-height: 1.7;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border-radius: var(--radius-md);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all var(--duration-fast) var(--ease-out);
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

/* Toggle (Raycast flatten style) */
.toggle {
  position: relative;
  display: inline-flex;
  align-items: center;
  cursor: pointer;
  flex-shrink: 0;
}
.toggle input {
  position: absolute;
  opacity: 0;
  pointer-events: none;
}
.toggle-track {
  position: relative;
  width: 32px;
  height: 18px;
  background: var(--surface-card);
  border: 1px solid var(--hairline);
  border-radius: var(--radius-full);
  transition: all var(--duration-fast) var(--ease-out);
}
.toggle-thumb {
  position: absolute;
  top: 1px;
  left: 1px;
  width: 14px;
  height: 14px;
  background: var(--text-ash);
  border-radius: 50%;
  transition: all var(--duration-fast) var(--ease-out);
}
.toggle input:checked + .toggle-track {
  background: var(--primary);
  border-color: var(--primary);
}
.toggle input:checked + .toggle-track .toggle-thumb {
  left: 15px;
  background: var(--on-primary);
}
.toggle:hover .toggle-track {
  border-color: var(--hairline-strong);
}
</style>
