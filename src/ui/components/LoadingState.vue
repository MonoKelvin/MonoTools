<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Loader2 } from '@lucide/vue'

defineProps<{
  message?: string
}>()

const visible = ref(true)

onMounted(() => {
  // 组件挂载后自动淡出
  requestAnimationFrame(() => {
    setTimeout(() => {
      visible.value = false
    }, 300)
  })
})
</script>

<template>
  <Transition name="fade">
    <div v-if="visible" class="loading-state" role="status" aria-label="加载中">
      <div class="loading-state__spinner">
        <Loader2 :size="28" :stroke-width="2" class="loading-state__icon" />
      </div>
      <p class="loading-state__message">{{ message || '正在初始化...' }}</p>
    </div>
  </Transition>
</template>

<style scoped>
.loading-state {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  background: var(--canvas);
  z-index: 100;
  border-radius: var(--radius-lg);
}

.loading-state__spinner {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: var(--bg-secondary);
  box-shadow: var(--shadow-md);
}

.loading-state__icon {
  color: var(--accent);
  animation: spin 1s linear infinite;
}

.loading-state__message {
  font-size: 13px;
  color: var(--text-secondary);
  font-weight: 500;
  letter-spacing: 0.2px;
  margin: 0;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 250ms var(--ease-out);
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
