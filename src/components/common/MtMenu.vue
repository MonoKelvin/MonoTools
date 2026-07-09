<script setup lang="ts">
import { onMounted, onBeforeUnmount, computed, ref, nextTick } from 'vue'
import type { Component } from 'vue'

export interface MtMenuItem {
  key?: string
  label?: string
  icon?: Component
  shortcut?: string
  danger?: boolean
  disabled?: boolean
  divider?: boolean
  custom?: string
}

const props = withDefaults(
  defineProps<{
    items: MtMenuItem[]
    modelValue: boolean
    x?: number
    y?: number
    anchor?: 'pointer' | 'top-left' | 'center'
    minWidth?: number
  }>(),
  { anchor: 'pointer', minWidth: 150 },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: boolean): void
  (e: 'select', item: MtMenuItem): void
}>()

const rootRef = ref<HTMLElement | null>(null)

function positionPanel() {
  if (!rootRef.value) return
  const el = rootRef.value
  const vw = window.innerWidth
  const vh = window.innerHeight
  const rect = el.getBoundingClientRect()
  const width = rect.width || props.minWidth
  const height = rect.height
  const anchorX = props.x ?? 0
  const anchorY = props.y ?? 0
  let left = anchorX
  let top = anchorY

  if (props.anchor === 'center') {
    left = (vw - width) / 2
    top = (vh - height) / 2
  }
  if (left + width > vw - 8) left = vw - width - 8
  if (top + height > vh - 8) top = vh - height - 8
  if (left < 8) left = 8
  if (top < 8) top = 8

  // 使用 CSS 变量传递位置，避免与动画的 transform 冲突
  el.style.setProperty('--menu-x', `${left}px`)
  el.style.setProperty('--menu-y', `${top}px`)
}

function close() { emit('update:modelValue', false) }

function onItemClick(item: MtMenuItem) {
  if (item.disabled || item.divider) return
  emit('select', item)
  close()
}

const visible = computed(() => props.modelValue)

function onWindowPointer(event: MouseEvent) {
  if (!rootRef.value) return
  if (rootRef.value.contains(event.target as Node)) return
  close()
}

function onKey(event: KeyboardEvent) {
  if (!visible.value) return
  if (event.key === 'Escape') { event.preventDefault(); close() }
}

onMounted(async () => {
  // 延迟定位，确保 DOM 完全渲染
  await nextTick()
  await nextTick()
  positionPanel()
  window.addEventListener('mousedown', onWindowPointer, true)
  window.addEventListener('contextmenu', onWindowPointer, true)
  window.addEventListener('keydown', onKey)
})

onBeforeUnmount(() => {
  window.removeEventListener('mousedown', onWindowPointer, true)
  window.removeEventListener('contextmenu', onWindowPointer, true)
  window.removeEventListener('keydown', onKey)
})
</script>

<template>
  <Teleport to="body">
    <Transition name="mt-menu">
      <div
        v-if="visible"
        ref="rootRef"
        class="mt-menu"
        role="menu"
      >
        <div class="mt-menu__content">
          <ul class="mt-menu__list">
          <li
            v-for="(item, idx) in items"
            :key="item.key ?? `i${idx}`"
            :class="[
              'mt-menu__row',
              { 'mt-menu__row--divider': item.divider, 'mt-menu__row--danger': item.danger, 'mt-menu__row--disabled': item.disabled },
            ]"
            role="menuitem"
            @click="onItemClick(item)"
          >
            <template v-if="item.divider">
              <span class="mt-menu__divider" />
            </template>
            <template v-else>
              <component :is="item.icon" v-if="item.icon" class="mt-menu__icon" :size="14" :stroke-width="2" />
              <span class="mt-menu__label">{{ item.label }}</span>
              <span v-if="item.shortcut" class="mt-menu__shortcut">{{ item.shortcut }}</span>
              <slot v-if="item.custom" :name="item.custom" />
            </template>
          </li>
        </ul>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.mt-menu {
  position: fixed;
  left: var(--menu-x, 0);
  top: var(--menu-y, 0);
  z-index: 9999;
  min-width: v-bind('`${minWidth}px`');
}

.mt-menu__content {
  --mt-menu-blur: 48px;
  --mt-menu-bg: rgb(17 17 17 / 0.78);

  pointer-events: auto;
  border-radius: var(--radius-lg);
  padding: var(--sp-1);
  background: var(--mt-menu-bg);
  backdrop-filter: blur(var(--mt-menu-blur)) saturate(180%);
  -webkit-backdrop-filter: blur(var(--mt-menu-blur)) saturate(180%);
  border: 1px solid var(--glass-border);
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
  user-select: none;
}

.mt-menu__list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
}

.mt-menu__row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 14px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  cursor: pointer;
  transition: background var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out);
  min-height: 36px;
}

.mt-menu__row + .mt-menu__row {
  margin-top: 1px;
}

.mt-menu__row:hover:not(.mt-menu__row--divider):not(.mt-menu__row--disabled) {
  background: var(--interactive-hover);
}

.mt-menu__row--disabled {
  color: var(--text-quaternary);
  cursor: default;
}

.mt-menu__row--danger {
  color: var(--color-danger);
}
.mt-menu__row--danger:hover {
  background: var(--color-danger-bg);
}

.mt-menu__icon {
  color: var(--text-secondary);
  flex-shrink: 0;
}
.mt-menu__row--danger .mt-menu__icon {
  color: inherit;
}

.mt-menu__label {
  flex: 1;
  min-width: 0;
  letter-spacing: 0.005em;
}

.mt-menu__shortcut {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  background: var(--surface-overlay);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-xs);
  padding: 1px 6px;
  line-height: 1.6;
  height: 18px;
}

.mt-menu__divider {
  display: block;
  height: 1px;
  background: var(--border-subtle);
  margin: var(--sp-1) var(--sp-2);
}

/* ========== 动画 ========== */

.mt-menu-enter-active .mt-menu__content,
.mt-menu-leave-active .mt-menu__content {
  transition: transform var(--dur-fast) cubic-bezier(0.34, 1.12, 0.64, 1);
}

.mt-menu-leave-active .mt-menu__content {
  transition-duration: 0.16s;
  transition-timing-function: var(--ease-out);
}

.mt-menu-enter-from .mt-menu__content,
.mt-menu-leave-to .mt-menu__content {
  transform: scale(0.94) translateY(-0.25rem);
}

@media (prefers-reduced-motion: reduce) {
  .mt-menu-enter-active .mt-menu__content,
  .mt-menu-leave-active .mt-menu__content {
    transition: none;
  }

  .mt-menu-enter-from .mt-menu__content,
  .mt-menu-leave-to .mt-menu__content {
    transform: none;
  }
}

</style>
