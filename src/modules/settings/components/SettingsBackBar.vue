<script setup lang="ts">
import { ArrowLeft } from '@lucide/vue'
import { useRouter } from 'vue-router'

const router = useRouter()

function goBack() {
  router.push('/')
}
</script>

<template>
  <!-- 顶部 + 与 SearchInput 协同: app-region: drag 实现整行可拖动窗口,
       内部交互控件 (返回按钮) 单独 app-region: no-drag, 避免吃掉窗口拖拽. -->
  <div class="settings-back-bar glass">
    <button class="settings-back-bar__back" @click="goBack" type="button">
      <ArrowLeft :size="16" :stroke-width="1.5" />
      <span>返回</span>
    </button>
  </div>
</template>

<style scoped>
.settings-back-bar {
  display: flex;
  align-items: center;
  height: 44px;
  padding: 0 var(--sp-5);
  flex-shrink: 0;
  border-bottom: 1px solid var(--border-subtle);
  -webkit-app-region: drag;
  app-region: drag;
}

.settings-back-bar__back {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-2);
  color: var(--text-tertiary);
  font-size: var(--text-sm);
  font-weight: 500;
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  padding: var(--sp-1) var(--sp-3);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
  -webkit-app-region: no-drag;
  app-region: no-drag;

  &:hover {
    color: var(--text-primary);
    background: var(--list-hover-bg);
  }
}
</style>
