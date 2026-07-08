<script setup lang="ts">
import { Settings, Terminal, Rocket, HelpCircle, ChevronUp, ChevronDown, CornerDownLeft } from "@lucide/vue"
import ThemeToggle from '@/components/common/ThemeToggle.vue'
import { hotkeyApi } from '@/services'

const emit = defineEmits<{
  (e: 'goPanel', panel: 'settings' | 'startup' | 'commands'): void
}>()

const goStartup = () => emit('goPanel', 'startup')
const goSettings = () => emit('goPanel', 'settings')
const goCommands = () => emit('goPanel', 'commands')
const goHelp = () => {
  alert(
    'MonoTools 用法：\n' +
      '• Alt+Space 唤起/隐藏搜索\n' +
      '• ↑ ↓ 选择，Enter 打开，Esc 关闭\n' +
      '• 直接输入搜索应用 / 文件 / 命令',
  )
}
</script>

<template>
  <div class="action-bar">
    <div class="left">
      <span class="action-bar-item">
        <span class="kbd-group">
          <span class="kbd"><ChevronUp :size="11" :stroke-width="2.5" /></span>
          <span class="kbd"><ChevronDown :size="11" :stroke-width="2.5" /></span>
        </span>
        <span class="action-label">导航</span>
      </span>
      <span class="action-bar-item">
        <span class="kbd"><CornerDownLeft :size="11" :stroke-width="2.5" /></span>
        <span class="action-label">打开</span>
      </span>
    </div>
    <div class="right">
      <button class="action-btn" @click="goStartup" title="启动项管理">
        <Rocket :size="13" :stroke-width="2" />
        <span>启动项</span>
      </button>
      <button class="action-btn" @click="goCommands" title="命令管理">
        <Terminal :size="13" :stroke-width="2" />
        <span>命令</span>
      </button>
      <ThemeToggle />
      <button class="action-btn" @click="goSettings" title="设置">
        <Settings :size="13" :stroke-width="2" />
        <span>设置</span>
      </button>
      <button class="action-btn" @click="goHelp" title="帮助">
        <HelpCircle :size="13" :stroke-width="2" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.action-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
  border-top: 1px solid var(--border);
  background: rgba(255, 255, 255, 0.015);
  color: var(--text-tertiary);
  font-size: 11px;
  flex-shrink: 0;
}
.left,
.right {
  display: flex;
  align-items: center;
  gap: 6px;
}
.action-bar-item {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.kbd-group {
  display: inline-flex;
  align-items: center;
}
.action-label {
  color: var(--text-tertiary);
  font-size: 11px;
}
.action-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  font-size: 11px;
  color: var(--text-secondary);
  background: transparent;
  border-radius: var(--radius-sm);
  border: none;
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
  white-space: nowrap;
}
.action-btn:hover {
  background: rgba(255, 255, 255, 0.06);
  color: var(--text-primary);
}
:global(.theme-light) .action-btn:hover {
  background: rgba(0, 0, 0, 0.04);
}
</style>
