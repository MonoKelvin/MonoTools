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
  { anchor: 'pointer', minWidth: 220 },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: boolean): void
  (e: 'select', item: MtMenuItem): void
}>()

const rootRef = ref<HTMLElement | null>(null)
const translate = ref({ x: 0, y: 0 })

function positionPanel() {
  if (!rootRef.value) return
  const el = rootRef.value
  const vw = window.innerWidth
  const vh = window.innerHeight
  const rect = el.getBoundingClientRect()
  const width = rect.width || props.minWidth
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
  translate.value = { x: left, y: top }
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
    <Transition name="mt-menu" appear>
      <div
        v-if="visible"
        ref="rootRef"
        class="mt-menu"
        :style="{
          transform: `translate(${translate.x}px, ${translate.y}px)`,
          minWidth: `${minWidth}px`,
        }"
        role="menu"
      >
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
    </Transition>
  </Teleport>
</template>

<style scoped>
.mt-menu {
  position: fixed;
  top: 0;
  left: 0;
  z-index: 9999;
  border-radius: var(--radius-lg);
  padding: var(--sp-1);
  background: var(--glass-bg);
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  border: 1px solid var(--glass-border);
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.15),
    0 8px 24px rgba(0, 0, 0, 0.3),
    0 0 48px rgba(0, 0, 0, 0.22);
  user-select: none;
  animation: mt-menu-in var(--dur-fast) var(--ease-out);
  transform-origin: var(--mt-menu-origin, top left);
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
  gap: var(--sp-3);
  padding: var(--sp-2) var(--sp-3);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--text-primary);
  cursor: pointer;
  transition: background var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out);
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

.mt-menu-enter-active,
.mt-menu-leave-active {
  transition: opacity var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
}
.mt-menu-enter-from,
.mt-menu-leave-to {
  opacity: 0;
  transform: translate(v-bind('translate.x + 8')px, v-bind('translate.y + 4')px) scale(0.98);
}

@keyframes mt-menu-in {
  from {
    opacity: 0;
    transform: translateY(-4px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}
</style>