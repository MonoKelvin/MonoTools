<script setup lang="ts">
import { onMounted, ref, watch, computed } from 'vue'
import { useSettingsStore } from '@/stores/settings'
import { hotkeyApi, pinTopApi, isTauri } from '@/services'
import { Keyboard, FolderSearch, Info, Pin, Monitor } from "@lucide/vue"

const settingsStore = useSettingsStore()

const newKey = ref('')
const recording = ref(false)
const message = ref('')
const savingPin = ref(false)
const pinToTop = ref(true)

const hotkey = computed({
  get: () => newKey.value || settingsStore.settings.hotkey,
  set: (v: string) => (newKey.value = v),
})

async function reloadPin() {
  if (!isTauri) return
  try { pinToTop.value = await pinTopApi.get() }
  catch { pinToTop.value = true }
}

async function togglePin(value: boolean) {
  pinToTop.value = value
  savingPin.value = true
  if (isTauri) {
    try {
      await pinTopApi.set(value)
      if (value) {
        const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow')
        const win = WebviewWindow.getCurrent()
        try { await win.show(); await win.setFocus(); await win.setAlwaysOnTop(true) }
        catch {}
      }
    } catch (err) {
      console.warn('置顶设置失败:', err)
    }
  }
  try { await settingsStore.update({ pinToTop: value }) }
  catch (err) { console.warn('保存设置失败:', err) }
  finally { savingPin.value = false }
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
    <section class="section">
      <div class="section-header">
        <Monitor :size="16" :stroke-width="1.8" />
        <h3>外观</h3>
      </div>
      <p class="section-desc">当前始终使用深色主题（Raycast 设计语言）。</p>
    </section>

    <section class="section">
      <div class="section-header">
        <Pin :size="16" :stroke-width="1.8" />
        <h3>窗口行为</h3>
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

    <section class="section">
      <div class="section-header">
        <Keyboard :size="16" :stroke-width="1.8" />
        <h3>全局快捷键</h3>
      </div>
      <div class="row">
        <div class="row-info">
          <div class="row-label">唤起快捷键</div>
          <div class="row-desc">按下此组合键显示 / 隐藏窗口</div>
        </div>
        <div class="row-actions">
          <kbd class="hotkey-chip">{{ hotkey }}</kbd>
          <button class="btn btn-ghost btn-sm" type="button" @click="startRecord">
            {{ recording ? '按下新组合键…' : '录制' }}
          </button>
        </div>
      </div>
      <p v-if="message" class="hint">{{ message }}</p>
      <div class="row">
        <button class="btn btn-primary btn-sm" @click="save">保存</button>
      </div>
    </section>

    <section class="section">
      <div class="section-header">
        <FolderSearch :size="16" :stroke-width="1.8" />
        <h3>文件搜索</h3>
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

    <section class="section">
      <div class="section-header">
        <Info :size="16" :stroke-width="1.8" />
        <h3>关于</h3>
      </div>
      <p class="about">
        MonoTools v0.1.0<br />
        轻量级系统效率工具 · 全局搜索 + 自定义命令
      </p>
    </section>
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

.section {
  background: var(--surface);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-lg);
  padding: var(--sp-6) var(--sp-8);
  display: flex;
  flex-direction: column;
  gap: var(--sp-5);
  transition: box-shadow var(--dur-fast) var(--ease-out);
}
.section:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.section-header {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  color: var(--text-tertiary);
}

.section-header h3 {
  margin: 0;
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.section-desc {
  margin: 0;
  font-size: var(--text-md);
  color: var(--text-secondary);
  line-height: var(--leading-normal);
}

.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-8);
}

.row-info {
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
  min-width: 0;
}

.row-label {
  font-size: var(--text-base);
  font-weight: 500;
  color: var(--text-primary);
}

.row-desc {
  font-size: var(--text-sm);
  color: var(--text-quaternary);
  line-height: var(--leading-normal);
}

.row-actions {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  flex-shrink: 0;
}

.hint {
  font-size: var(--text-sm);
  color: var(--color-success);
  margin: 0;
}

.about {
  margin: 0;
  font-size: var(--text-md);
  color: var(--text-secondary);
  line-height: 1.7;
}

.hotkey-chip {
  display: inline-flex;
  align-items: center;
  font-family: var(--font-mono);
  font-size: var(--text-md);
  font-weight: 500;
  padding: var(--sp-1) var(--sp-3);
  height: 26px;
  color: var(--text-secondary);
  background: var(--surface-overlay);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-xs);
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

/* Toggle */
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
  width: 38px;
  height: 20px;
  background: var(--surface-overlay);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-full);
  transition: all var(--dur-fast) var(--ease-out);
}
.toggle-thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 14px;
  height: 14px;
  background: var(--text-tertiary);
  border-radius: 50%;
  transition: all var(--dur-fast) var(--ease-out);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}
.toggle input:checked + .toggle-track {
  background: var(--accent);
  border-color: var(--accent);
}
.toggle input:checked + .toggle-track .toggle-thumb {
  left: 20px;
  background: var(--canvas);
}
.toggle:hover .toggle-track {
  border-color: var(--border-hover);
}
</style>