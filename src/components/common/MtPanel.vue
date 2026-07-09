<script setup lang="ts">
import { X } from 'lucide-vue-next'

interface Props {
  title?: string
  visible?: boolean
  width?: number | string
}

withDefaults(defineProps<Props>(), {
  visible: false,
  width: 360,
})

const emit = defineEmits<{
  (e: 'close'): void
}>()
</script>

<template>
  <Transition name="slide-in-right">
    <div v-if="visible" class="mt-panel-overlay" @click="emit('close')">
      <div class="mt-panel" :style="{ width: typeof width === 'number' ? `${width}px` : width }" @click.stop>
        <div class="mt-panel__header">
          <h2 v-if="title" class="mt-panel__title">{{ title }}</h2>
          <button class="mt-panel__close" @click="emit('close')">
            <X :size="16" :stroke-width="2" />
          </button>
        </div>
        <div class="mt-panel__content">
          <slot />
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.mt-panel-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.3);
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
  display: flex;
  justify-content: flex-end;
  align-items: stretch;
  z-index: 1000;
}

.mt-panel {
  background: var(--surface);
  border-left: 1px solid var(--border-default);
  display: flex;
  flex-direction: column;
  height: 100%;
  box-shadow: var(--shadow-xl);
}

.mt-panel__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-5);
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.mt-panel__title {
  font-size: var(--text-lg);
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}

.mt-panel__close {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-tertiary);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
}

.mt-panel__close:hover {
  background: var(--surface-hover);
  color: var(--text-primary);
}

.mt-panel__content {
  flex: 1;
  overflow-y: auto;
  padding: var(--sp-5);
}

.slide-in-right-enter-active,
.slide-in-right-leave-active {
  transition: all var(--dur-normal) var(--ease-out);
}

.slide-in-right-enter-from,
.slide-in-right-leave-to {
  opacity: 0;
}

.slide-in-right-enter-from .mt-panel,
.slide-in-right-leave-to .mt-panel {
  transform: translateX(100%);
}
</style>
