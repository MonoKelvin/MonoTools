<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useSettingsStore } from '@/stores/settings'
import { useThemeStore } from '@/stores/theme'
import { hotkeyApi } from '@/services'
import { Palette, Keyboard, FolderSearch, Info } from "@lucide/vue"

const settingsStore = useSettingsStore()
const themeStore = useThemeStore()

const newKey = ref('')
const recording = ref(false)
const message = ref('')

async function save() {
  await settingsStore.update({
    hotkey: newKey.value.trim() || settingsStore.settings.hotkey,
    theme: themeStore.mode,
    accentColor: themeStore.accent,
  })
  message.value = '已保存'
  setTimeout(() => (message.value = ''), 1500)
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

function tryRegister() {
  hotkeyApi.register(settingsStore.settings.hotkey).catch(() => {})
}

onMounted(async () => {
  await settingsStore.load()
  newKey.value = settingsStore.settings.hotkey
  document.addEventListener('keydown', onRecordKey)
})
</script>

<template>
  <div class="panel">
    <section class="card">
      <div class="card-header">
        <Palette :size="16" :stroke-width="2" />
        <h2>外观</h2>
      </div>
      <div class="row">
        <label class="row-label">主题模式</label>
        <div class="theme-buttons">
          <button
            v-for="m in (['light', 'dark', 'auto'] as const)"
            :key="m"
            class="theme-pill"
            :class="{ 'is-active': themeStore.mode === m }"
            @click="themeStore.setMode(m)"
          >
            {{ m === 'light' ? '亮色' : m === 'dark' ? '暗色' : '自动' }}
          </button>
        </div>
      </div>
      <div class="row">
        <label class="row-label">强调色</label>
        <input
          type="color"
          :value="themeStore.accent"
          @input="(e) => themeStore.setAccent((e.target as HTMLInputElement).value)"
          class="color-picker"
        />
      </div>
    </section>

    <section class="card">
      <div class="card-header">
        <Keyboard :size="16" :stroke-width="2" />
        <h2>全局快捷键</h2>
      </div>
      <div class="row">
        <label class="row-label">当前快捷键</label>
        <div class="hotkey-display">{{ newKey || settingsStore.settings.hotkey }}</div>
        <button class="btn btn-ghost" @click="startRecord">
          {{ recording ? '按下新的组合键…' : '录制' }}
        </button>
      </div>
      <p v-if="message" class="hint">{{ message }}</p>
      <button class="btn btn-primary" @click="save">保存</button>
    </section>

    <section class="card">
      <div class="card-header">
        <FolderSearch :size="16" :stroke-width="2" />
        <h2>文件搜索</h2>
      </div>
      <div class="row">
        <label class="row-label">
          <input
            type="checkbox"
            :checked="settingsStore.settings.fileSearchEnabled"
            @change="settingsStore.update({ fileSearchEnabled: ($event.target as HTMLInputElement).checked })"
          />
          启用文件搜索
        </label>
      </div>
    </section>

    <section class="card">
      <div class="card-header">
        <Info :size="16" :stroke-width="2" />
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
  padding: 16px 20px;
  overflow-y: auto;
  height: 100%;
  background: var(--bg-primary);
  color: var(--text-primary);
}
.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 14px 18px;
  margin-bottom: 10px;
}
.card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 14px;
  color: var(--text-secondary);
}
.card h2 {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 8px 0;
}
.row-label {
  flex: 1;
  font-size: 13px;
  color: var(--text-primary);
}
.theme-buttons {
  display: flex;
  gap: 4px;
}
.theme-pill {
  padding: 6px 14px;
  font-size: 12px;
  color: var(--text-secondary);
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 999px;
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}
.theme-pill:hover {
  border-color: var(--border-hover);
  color: var(--text-primary);
}
.theme-pill.is-active {
  background: var(--accent);
  color: white;
  border-color: var(--accent);
}
.color-picker {
  width: 48px;
  height: 30px;
  border: 1px solid var(--border);
  background: transparent;
  border-radius: 6px;
  cursor: pointer;
  padding: 2px;
}
.hotkey-display {
  flex: 1;
  text-align: center;
  font-family: var(--font-mono);
  font-size: 14px;
  padding: 8px 12px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
}
.hint {
  font-size: 12px;
  color: var(--accent);
  margin: 4px 0;
}
.about {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.7;
}
</style>
