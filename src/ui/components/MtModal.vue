<script setup lang="ts">
import { computed, type Component } from 'vue'
import { X } from '@lucide/vue'

const props = withDefaults(
  defineProps<{
    visible: boolean
    title?: string
    icon?: Component
    width?: string | number
    maxHeight?: string | number
    closable?: boolean
    closeOnOverlayClick?: boolean
    disableAnimation?: boolean
  }>(),
  {
    title: '',
    width: 480,
    maxHeight: 580,
    closable: true,
    closeOnOverlayClick: true,
    disableAnimation: false,
  },
)

const emit = defineEmits<{
  (e: 'close'): void
}>()

const handleOverlayClick = () => {
  if (props.closeOnOverlayClick) {
    emit('close')
  }
}

const handleClose = () => {
  emit('close')
}

const modalStyle = computed(() => ({
  width: typeof props.width === 'number' ? `${props.width}px` : props.width,
  maxHeight: typeof props.maxHeight === 'number' ? `${props.maxHeight}px` : props.maxHeight,
}))

const bodyMaxHeight = computed(() => {
  const maxHeight = typeof props.maxHeight === 'number' ? props.maxHeight : 580
  return `calc(${maxHeight}px - 60px)`
})
</script>

<template>
  <Teleport to="body">
    <Transition :name="disableAnimation ? '' : 'mt-modal'">
      <div
        v-if="visible"
        class="mt-modal-overlay"
        @click="handleOverlayClick"
      >
        <div
          class="mt-modal"
          :style="modalStyle"
          @click.stop
        >
          <div v-if="title || closable || icon" class="mt-modal__header">
            <slot name="header">
              <component v-if="icon" :is="icon" :size="18" class="mt-modal__icon" />
              <span v-if="title" class="mt-modal__title">{{ title }}</span>
            </slot>
            <button
              v-if="closable"
              class="mt-modal__close"
              @click="handleClose"
            >
              <X :size="16" />
            </button>
          </div>

          <div class="mt-modal__body" :style="{ maxHeight: bodyMaxHeight, overflowY: 'auto' }">
            <slot></slot>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.mt-modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  pointer-events: auto;
}

.mt-modal {
  --mt-modal-blur: 48px;
  --mt-modal-bg: rgb(17 17 17 / 0.78);

  background: var(--mt-modal-bg);
  backdrop-filter: blur(var(--mt-modal-blur)) saturate(180%);
  -webkit-backdrop-filter: blur(var(--mt-modal-blur)) saturate(180%);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow:
    0 4px 8px rgba(0, 0, 0, 0.24),
    0 8px 16px rgba(0, 0, 0, 0.20),
    0 16px 32px rgba(0, 0, 0, 0.16),
    0 24px 48px rgba(0, 0, 0, 0.10),
    0 32px 64px rgba(0, 0, 0, 0.05);
  background-image:
    linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.04) 0%,
      rgba(255, 255, 255, 0.0) 40%,
      rgba(0, 0, 0, 0.02) 100%
    );
  overflow: hidden;
}

.os-win10 .mt-modal {
  --mt-modal-bg: rgb(28 28 32 / 0.98);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
  border-color: var(--border-default);
  box-shadow:
    0 12px 32px rgba(0, 0, 0, 0.4),
    0 4px 12px rgba(0, 0, 0, 0.2);
}

.mt-modal__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-3);
  padding: var(--sp-4) var(--sp-5);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  flex-shrink: 0;
}

.mt-modal__icon {
  color: var(--accent);
  flex-shrink: 0;
}

.mt-modal__title {
  flex: 1;
  font-size: var(--text-base);
  font-weight: 600;
  color: var(--text-primary);
}

.mt-modal__close {
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
  flex-shrink: 0;
}

.mt-modal__close:hover {
  background: rgba(255, 255, 255, 0.06);
  color: var(--text-secondary);
}

.mt-modal__body {
  padding: var(--sp-4) var(--sp-5);
  overflow-y: auto;
  overflow-x: hidden;
}

/* 滚动条统一样式见 theme.scss, 模态框沿用全局样式 */

.mt-modal-enter-active,
.mt-modal-leave-active {
  transition: opacity 0.16s var(--ease-out);
}

.mt-modal-enter-from,
.mt-modal-leave-to {
  opacity: 0;
}

@media (prefers-reduced-motion: reduce) {
  .mt-modal-enter-active,
  .mt-modal-leave-active {
    transition: none;
  }
}
</style>
