<script setup lang="ts">
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'close': []
}>()

const activeTab = ref('general')
const config = ref({
  language: 'zh-CN',
  startup: true,
  silentStart: true,
  hotkey: 'Alt+Space',
})

const tabs = [
  { id: 'general', label: '通用', icon: 'Settings' },
  { id: 'search', label: '搜索', icon: 'Search' },
  { id: 'workspace', label: '工作区', icon: 'FolderOpen' },
  { id: 'plugins', label: '插件', icon: 'Puzzle' },
  { id: 'theme', label: '主题', icon: 'Palette' },
  { id: 'shortcuts', label: '快捷键', icon: 'Keyboard' },
  { id: 'about', label: '关于', icon: 'Info' },
]

const currentTab = computed(() => tabs.find(t => t.id === activeTab.value))

async function saveSettings() {
  try {
    await invoke('config:set', {
      key: 'general.language',
      value: config.value.language,
    })
    await invoke('config:set', {
      key: 'general.startup',
      value: config.value.startup,
    })
    await invoke('config:set', {
      key: 'general.silentStart',
      value: config.value.silentStart,
    })

    // 关闭设置面板
    emit('close')
  } catch (error) {
    console.error('Failed to save settings:', error)
  }
}

function cancel() {
  emit('close')
}

function closePanel() {
  emit('close')
}
</script>

<template>
  <div v-if="modelValue" class="settings-overlay" @click="closePanel">
    <div class="settings-panel" @click.stop>
      <div class="settings-header">
        <h2 class="settings-title">设置</h2>
        <button class="settings-close" type="button" @click="closePanel">
          ✕
        </button>
      </div>

      <div class="settings-body">
        <!-- 侧边栏 -->
        <nav class="settings-nav">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            class="nav-item"
            :class="{ 'is-active': activeTab === tab.id }"
            @click="activeTab = tab.id"
          >
            <component :is="tab.icon" :size="16" />
            <span>{{ tab.label }}</span>
          </button>
        </nav>

        <!-- 内容区 -->
        <div class="settings-content">
          <!-- 通用设置 -->
          <div v-if="activeTab === 'general'" class="tab-content">
            <h3>通用</h3>

            <div class="setting-item">
              <label class="setting-label">
                <span>语言</span>
                <span class="setting-desc">选择界面语言</span>
              </label>
              <select v-model="config.language" class="setting-select">
                <option value="zh-CN">简体中文</option>
                <option value="en-US">English</option>
                <option value="system">跟随系统</option>
              </select>
            </div>

            <div class="setting-item">
              <label class="setting-label">
                <span>开机自启</span>
                <span class="setting-desc">登录时自动启动 MonoTools</span>
              </label>
              <input
                v-model="config.startup"
                type="checkbox"
                class="setting-checkbox"
              />
            </div>

            <div class="setting-item">
              <label class="setting-label">
                <span>静默启动</span>
                <span class="setting-desc">启动时不显示窗口</span>
              </label>
              <input
                v-model="config.silentStart"
                type="checkbox"
                class="setting-checkbox"
              />
            </div>
          </div>

          <!-- 搜索设置 -->
          <div v-else-if="activeTab === 'search'" class="tab-content">
            <h3>搜索</h3>
            <p class="placeholder">搜索设置（开发中...）</p>
          </div>

          <!-- 工作区设置 -->
          <div v-else-if="activeTab === 'workspace'" class="tab-content">
            <h3>工作区</h3>
            <p class="placeholder">工作区设置（开发中...）</p>
          </div>

          <!-- 插件设置 -->
          <div v-else-if="activeTab === 'plugins'" class="tab-content">
            <h3>插件</h3>
            <p class="placeholder">插件管理（开发中...）</p>
          </div>

          <!-- 主题设置 -->
          <div v-else-if="activeTab === 'theme'" class="tab-content">
            <h3>主题</h3>
            <p class="placeholder">主题设置（开发中...）</p>
          </div>

          <!-- 快捷键设置 -->
          <div v-else-if="activeTab === 'shortcuts'" class="tab-content">
            <h3>快捷键</h3>
            <p class="placeholder">快捷键设置（开发中...）</p>
          </div>

          <!-- 关于 -->
          <div v-else-if="activeTab === 'about'" class="tab-content">
            <h3>关于</h3>
            <div class="about-info">
              <p><strong>MonoTools</strong></p>
              <p class="version">版本 1.0.0</p>
              <p class="copyright">© 2026 MonoKelvin</p>
              <p class="license">MIT License</p>
            </div>
          </div>
        </div>
      </div>

      <!-- 底部按钮 -->
      <div class="settings-footer">
        <button class="btn-secondary" type="button" @click="cancel">
          取消
        </button>
        <button class="btn-primary" type="button" @click="saveSettings">
          保存
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  animation: fadeIn 0.15s ease;
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.settings-panel {
  width: 800px;
  max-width: 90vw;
  max-height: 80vh;
  background: var(--mt-surface-1);
  border: 1px solid var(--mt-hairline);
  border-radius: var(--mt-radius-xl);
  box-shadow: 0 24px 48px rgba(0, 0, 0, 0.4);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--mt-space-md) var(--mt-space-lg);
  border-bottom: 1px solid var(--mt-hairline);
}

.settings-title {
  font-size: 20px;
  font-weight: 600;
  color: var(--mt-ink);
  margin: 0;
}

.settings-close {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: 1px solid var(--mt-hairline);
  border-radius: var(--mt-radius-md);
  color: var(--mt-ink-subtle);
  cursor: pointer;
  transition: all 0.15s ease;
}

.settings-close:hover {
  background: var(--mt-surface-2);
  color: var(--mt-ink);
}

.settings-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.settings-nav {
  width: 200px;
  padding: var(--mt-space-sm);
  border-right: 1px solid var(--mt-hairline);
  overflow-y: auto;
}

.nav-item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: var(--mt-space-sm);
  padding: var(--mt-space-sm) var(--mt-space-md);
  background: transparent;
  border: none;
  border-radius: var(--mt-radius-md);
  color: var(--mt-ink-muted);
  font-size: 14px;
  cursor: pointer;
  transition: all 0.15s ease;
  text-align: left;
}

.nav-item:hover {
  background: var(--mt-surface-2);
  color: var(--mt-ink);
}

.nav-item.is-active {
  background: var(--mt-surface-2);
  color: var(--mt-ink);
  font-weight: 500;
}

.settings-content {
  flex: 1;
  padding: var(--mt-space-lg);
  overflow-y: auto;
}

.tab-content h3 {
  font-size: 18px;
  font-weight: 600;
  color: var(--mt-ink);
  margin: 0 0 var(--mt-space-lg);
}

.placeholder {
  color: var(--mt-ink-subtle);
  font-size: 14px;
}

.setting-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--mt-space-md) 0;
  border-bottom: 1px solid var(--mt-hairline);
}

.setting-item:last-child {
  border-bottom: none;
}

.setting-label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  color: var(--mt-ink);
  font-size: 14px;
  font-weight: 500;
}

.setting-desc {
  font-size: 12px;
  color: var(--mt-ink-subtle);
  font-weight: 400;
}

.setting-select {
  min-width: 160px;
  padding: var(--mt-space-xs) var(--mt-space-sm);
  background: var(--mt-canvas);
  border: 1px solid var(--mt-hairline);
  border-radius: var(--mt-radius-md);
  color: var(--mt-ink);
  font-size: 14px;
  cursor: pointer;
}

.setting-checkbox {
  width: 40px;
  height: 24px;
  position: relative;
  -webkit-appearance: none;
  appearance: none;
  background: var(--mt-surface-3);
  border-radius: var(--mt-radius-pill);
  cursor: pointer;
  transition: background-color 0.15s ease;
}

.setting-checkbox:checked {
  background: var(--mt-primary);
}

.setting-checkbox::before {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  width: 20px;
  height: 20px;
  background: white;
  border-radius: 50%;
  transition: transform 0.15s ease;
}

.setting-checkbox:checked::before {
  transform: translateX(16px);
}

.about-info {
  display: flex;
  flex-direction: column;
  gap: var(--mt-space-sm);
}

.about-info p {
  margin: 0;
  color: var(--mt-ink-muted);
  font-size: 14px;
}

.about-info .version {
  font-size: 12px;
  color: var(--mt-ink-subtle);
}

.about-info .copyright {
  font-size: 12px;
}

.about-info .license {
  font-size: 12px;
  color: var(--mt-ink-subtle);
}

.settings-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--mt-space-sm);
  padding: var(--mt-space-md) var(--mt-space-lg);
  border-top: 1px solid var(--mt-hairline);
}

.btn-primary,
.btn-secondary {
  padding: var(--mt-space-xs) var(--mt-space-md);
  border-radius: var(--mt-radius-md);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-primary {
  background: var(--mt-primary);
  border: none;
  color: white;
}

.btn-primary:hover {
  background: var(--mt-primary-hover);
}

.btn-secondary {
  background: var(--mt-surface-2);
  border: 1px solid var(--mt-hairline);
  color: var(--mt-ink);
}

.btn-secondary:hover {
  background: var(--mt-surface-3);
}
</style>
