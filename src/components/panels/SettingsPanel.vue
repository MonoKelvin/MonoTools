<script setup lang="ts">
import { onMounted, ref, watch, computed, onUnmounted } from 'vue'
import { useSettingsStore } from '@/stores/settings'
import { hotkeyApi, pinTopApi, isTauri } from '@/services'
import { Keyboard, FolderSearch, Info, Pin, Monitor } from "@lucide/vue"
import MtCard from '@/components/common/MtCard.vue'
import MtButton from '@/components/common/MtButton.vue'
import MtDivider from '@/components/common/MtDivider.vue'

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
    } catch {}
  }
  try { await settingsStore.update({ pinToTop: value }) }
  catch {}
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

onUnmounted(() => {
  document.removeEventListener('keydown', onRecordKey)
})
</script>

<template>
  <div class="settings-panel">
    <MtCard class="settings-panel__card">
      <div class="settings-panel__section-header">
        <Monitor :size="16" :stroke-width="1.5" />
        <h3>外观</h3>
      </div>
      <p class="settings-panel__section-desc">当前始终使用深色主题（Raycast 设计语言）。</p>
    </MtCard>

    <MtCard class="settings-panel__card">
      <div class="settings-panel__section-header">
        <Pin :size="16" :stroke-width="1.5" />
        <h3>窗口行为</h3>
      </div>

      <div class="settings-panel__row">
        <div class="settings-panel__row-info">
          <div class="settings-panel__row-label">窗口置顶</div>
          <div class="settings-panel__row-desc">开启后，搜索窗口会在所有应用之上</div>
        </div>
        <label class="settings-panel__toggle">
          <input
            type="checkbox"
            :checked="pinToTop"
            :disabled="savingPin"
            @change="togglePin(($event.target as HTMLInputElement).checked)"
          />
          <span class="settings-panel__toggle-track">
            <span class="settings-panel__toggle-thumb"></span>
          </span>
        </label>
      </div>
    </MtCard>

    <MtCard class="settings-panel__card">
      <div class="settings-panel__section-header">
        <Keyboard :size="16" :stroke-width="1.5" />
        <h3>全局快捷键</h3>
      </div>

      <div class="settings-panel__row">
        <div class="settings-panel__row-info">
          <div class="settings-panel__row-label">唤起快捷键</div>
          <div class="settings-panel__row-desc">按下此组合键显示 / 隐藏窗口</div>
        </div>
        <div class="settings-panel__row-actions">
          <kbd class="settings-panel__hotkey-chip">{{ hotkey }}</kbd>
          <MtButton variant="ghost" size="sm" @click="startRecord">
            {{ recording ? '按下新组合键…' : '录制' }}
          </MtButton>
        </div>
      </div>

      <p v-if="message" class="settings-panel__hint">{{ message }}</p>

      <MtDivider />

      <div class="settings-panel__row settings-panel__row--justify-end">
        <MtButton variant="primary" size="sm" @click="save">保存</MtButton>
      </div>
    </MtCard>

    <MtCard class="settings-panel__card">
      <div class="settings-panel__section-header">
        <FolderSearch :size="16" :stroke-width="1.5" />
        <h3>文件搜索</h3>
      </div>

      <div class="settings-panel__row">
        <div class="settings-panel__row-info">
          <div class="settings-panel__row-label">启用文件搜索</div>
          <div class="settings-panel__row-desc">使用 SQLite FTS5 索引文件名</div>
        </div>
        <label class="settings-panel__toggle">
          <input
            type="checkbox"
            :checked="settingsStore.settings.fileSearchEnabled"
            @change="settingsStore.update({ fileSearchEnabled: ($event.target as HTMLInputElement).checked })"
          />
          <span class="settings-panel__toggle-track">
            <span class="settings-panel__toggle-thumb"></span>
          </span>
        </label>
      </div>
    </MtCard>

    <MtCard class="settings-panel__card">
      <div class="settings-panel__section-header">
        <Info :size="16" :stroke-width="1.5" />
        <h3>关于</h3>
      </div>
      <p class="settings-panel__about">
        MonoTools v0.1.0<br />
        轻量级系统效率工具 · 全局搜索 + 自定义命令
      </p>
    </MtCard>
  </div>
</template>

<style scoped>
.settings-panel {
  padding: var(--sp-5);
  overflow-y: auto;
  height: 100%;
  background: var(--canvas);
  display: flex;
  flex-direction: column;
  gap: var(--sp-4);
}

.settings-panel__card {
  padding: var(--sp-5);
}

.settings-panel__section-header {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  color: var(--text-tertiary);
  margin-bottom: var(--sp-4);
}

.settings-panel__section-header h3 {
  margin: 0;
  font-size: var(--text-sm);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.settings-panel__section-desc {
  margin: 0;
  font-size: var(--text-base);
  color: var(--text-secondary);
  line-height: var(--leading-normal);
}

.settings-panel__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-5);
}

.settings-panel__row--justify-end {
  justify-content: flex-end;
}

.settings-panel__row-info {
  display: flex;
  flex-direction: column;
  gap: var(--sp-1);
  min-width: 0;
}

.settings-panel__row-label {
  font-size: var(--text-base);
  font-weight: 500;
  color: var(--text-primary);
}

.settings-panel__row-desc {
  font-size: var(--text-sm);
  color: var(--text-quaternary);
  line-height: var(--leading-normal);
}

.settings-panel__row-actions {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  flex-shrink: 0;
}

.settings-panel__hint {
  font-size: var(--text-sm);
  color: var(--color-success);
  margin: var(--sp-2) 0 0;
}

.settings-panel__about {
  margin: 0;
  font-size: var(--text-base);
  color: var(--text-secondary);
  line-height: 1.7;
}

.settings-panel__hotkey-chip {
  display: inline-flex;
  align-items: center;
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  font-weight: 500;
  padding: var(--sp-1) var(--sp-3);
  height: 26px;
  color: var(--text-secondary);
  background: var(--surface-raised);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-xs);
}

.settings-panel__toggle {
  position: relative;
  display: inline-flex;
  align-items: center;
  cursor: pointer;
  flex-shrink: 0;
}

.settings-panel__toggle input {
  position: absolute;
  opacity: 0;
  pointer-events: none;
}

.settings-panel__toggle-track {
  position: relative;
  width: 40px;
  height: 22px;
  background: var(--surface-raised);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-full);
  transition: all var(--dur-fast) var(--ease-out);
}

.settings-panel__toggle-thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  background: var(--text-tertiary);
  border-radius: 50%;
  transition: all var(--dur-fast) var(--ease-out);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}

.settings-panel__toggle input:checked + .settings-panel__toggle-track {
  background: var(--accent);
  border-color: var(--accent);
}

.settings-panel__toggle input:checked + .settings-panel__toggle-track .settings-panel__toggle-thumb {
  left: 20px;
  background: #fff;
}

.settings-panel__toggle:hover .settings-panel__toggle-track {
  border-color: var(--border-hover);
}
</style>
