<script setup lang="ts">
import { useRouter } from 'vue-router'
import ThemeToggle from '@/components/common/ThemeToggle.vue'
import { hotkeyApi } from '@/services'

const router = useRouter()

const goStartup = () => {
  router.push({ name: 'startup' })
}
const goSettings = () => {
  router.push({ name: 'settings' })
}
const goHelp = () => {
  alert(
    'MonoTools 用法：\n' +
      '• Alt+Space 唤起/隐藏搜索面板\n' +
      '• 上下箭头选择，Enter 执行，Esc 关闭\n' +
      '• 直接输入即可搜索应用/文件/命令',
  )
}
</script>

<template>
  <div class="action-bar">
    <div class="left">
      <span class="action-bar-item">
        <span class="kbd">↑</span>
        <span class="kbd">↓</span>
        Navigate
      </span>
      <span class="action-bar-item" style="margin-left: 16px">
        <span class="kbd">⏎</span>
        Open
      </span>
    </div>
    <div class="right">
      <button class="action-btn" @click="goStartup">⚙ Startup</button>
      <ThemeToggle />
      <button class="action-btn" @click="goHelp">? Help</button>
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
  background: rgba(255, 255, 255, 0.02);
  color: var(--text-tertiary);
  font-size: 11px;
}
.left,
.right {
  display: flex;
  align-items: center;
  gap: 8px;
}
.action-bar-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.action-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  font-size: 11px;
  color: var(--text-secondary);
  background: transparent;
  border-radius: 6px;
  border: none;
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}
.action-btn:hover {
  background: rgba(255, 255, 255, 0.06);
  color: var(--text-primary);
}
</style>
