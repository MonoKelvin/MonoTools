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
  if (props.closeOnOverlayClick) emit('close')
}

const handleClose = () => emit('close')

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
      <div v-if="visible" class="mt-modal-overlay" @click="handleOverlayClick">
        <div class="mt-modal glass-card" :style="modalStyle" @click.stop>
          <div v-if="title || closable || icon" class="mt-modal__header">
            <slot name="header">
              <component v-if="icon" :is="icon" :size="18" class="mt-modal__icon" />
              <span v-if="title" class="mt-modal__title">{{ title }}</span>
            </slot>
            <button v-if="closable" class="mt-modal__close" @click="handleClose" aria-label="关闭">
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
/* === Overlay (遮罩) ====================================================== */
.mt-modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: var(--sp-5);
}

/* === Modal 容器 (高级玻璃 + 微高光) ===================================== */
.mt-modal {
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-xl);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.os-win10 .mt-modal {
  background: rgb(28 28 32 / 0.98);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
  border-color: var(--border-default);
  box-shadow: var(--shadow-lg);
}

/* === Header ============================================================= */
.mt-modal__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-3);
  padding: var(--sp-4) var(--sp-5);
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
  min-height: 56px;
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
  letter-spacing: -0.005em;
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
  transition:
    background var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out);
  flex-shrink: 0;
}
.mt-modal__close:hover {
  background: var(--list-hover-bg);
  color: var(--text-secondary);
}

/* === Body =============================================================== */
.mt-modal__body {
  padding: var(--sp-4) var(--sp-5);
  overflow-y: auto;
  overflow-x: hidden;
}

/* === 进出动画: 容器从 96% 缩放淡入, 简洁有质感 ========================== */
.mt-modal-enter-active,
.mt-modal-leave-active {
  transition: opacity var(--dur-normal) var(--ease-out);
  transition-delay: 0s;
}
.mt-modal-enter-active .mt-modal,
.mt-modal-leave-active .mt-modal {
  transition:
    transform var(--dur-normal) var(--ease-out),
    opacity var(--dur-normal) var(--ease-out);
}
.mt-modal-enter-from,
.mt-modal-leave-to { opacity: 0; }
.mt-modal-enter-from .mt-modal,
.mt-modal-leave-to .mt-modal {
  opacity: 0;
  transform: scale(0.96) translateY(8px);
}

@media (prefers-reduced-motion: reduce) {
  .mt-modal-enter-active,
  .mt-modal-leave-active,
  .mt-modal-enter-active .mt-modal,
  .mt-modal-leave-active .mt-modal {
    transition: none;
  }
}
</style>
