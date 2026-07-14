<script setup lang="ts">
import { onBeforeUnmount, onMounted, computed, ref, nextTick, watch } from 'vue'
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
const activeIndex = ref(-1)
const ignoreTarget = ref<EventTarget | null>(null)
const adjustedX = ref(0)
const adjustedY = ref(0)

const visible = computed(() => props.modelValue)

/**
 * 根据实际测量的菜单尺寸调整位置, 确保不超出视口.
 * 由于菜单宽度是自适应的(max-content + 60vw上限),
 * 必须等 DOM 渲染后才能拿到真实宽高做边界校准.
 *
 * 偏移策略: 鼠标右下偏移 (4, 6) px, 避免菜单正好覆盖在光标上.
 * 如果右侧空间不足, 则自动向左偏移贴边; 底部同理.
 */
function adjustPosition() {
  if (!rootRef.value) return
  const rect = rootRef.value.getBoundingClientRect()
  const vw = window.innerWidth
  const vh = window.innerHeight
  const margin = 8
  const offsetX = 4
  const offsetY = 6

  let left = (props.x ?? 0) + offsetX
  let top = (props.y ?? 0) + offsetY

  if (props.anchor === 'center') {
    left = (vw - rect.width) / 2
    top = (vh - rect.height) / 2
  } else {
    if (left + rect.width > vw - margin) {
      // 右边放不下: 贴右边, 并稍微往左偏移避免贴边
      left = vw - rect.width - margin
    }
    if (top + rect.height > vh - margin) {
      // 下边放不下: 贴底
      top = vh - rect.height - margin
    }
    if (left < margin) left = margin
    if (top < margin) top = margin
  }

  adjustedX.value = left
  adjustedY.value = top
}

const menuStyle = computed(() => ({
  left: `${adjustedX.value}px`,
  top: `${adjustedY.value}px`,
}))

function close() { emit('update:modelValue', false) }

function onItemClick(item: MtMenuItem) {
  if (item.disabled || item.divider) return
  emit('select', item)
  close()
}

function onWindowPointer(event: MouseEvent) {
  if (!rootRef.value) return
  if (rootRef.value.contains(event.target as Node)) return
  if (ignoreTarget.value === event.target) return
  close()
}

function getEnabledIndices(): number[] {
  return props.items
    .map((item, idx) => ({ item, idx }))
    .filter(({ item }) => !item.disabled && !item.divider)
    .map(({ idx }) => idx)
}

function onKey(event: KeyboardEvent) {
  if (!visible.value) return

  if (event.key === 'Escape') {
    event.preventDefault()
    close()
    return
  }

  const enabledIndices = getEnabledIndices()
  if (enabledIndices.length === 0) return

  if (event.key === 'ArrowDown') {
    event.preventDefault()
    const currentIdx = enabledIndices.indexOf(activeIndex.value)
    const nextIdx = currentIdx < 0 ? 0 : (currentIdx + 1) % enabledIndices.length
    activeIndex.value = enabledIndices[nextIdx]
    return
  }

  if (event.key === 'ArrowUp') {
    event.preventDefault()
    const currentIdx = enabledIndices.indexOf(activeIndex.value)
    const prevIdx = currentIdx < 0 ? enabledIndices.length - 1 : (currentIdx - 1 + enabledIndices.length) % enabledIndices.length
    activeIndex.value = enabledIndices[prevIdx]
    return
  }

  if (event.key === 'Enter') {
    event.preventDefault()
    if (activeIndex.value >= 0 && !props.items[activeIndex.value]?.disabled && !props.items[activeIndex.value]?.divider) {
      onItemClick(props.items[activeIndex.value])
    }
    return
  }
}

function onWindowScroll() {
  close()
}

function bindGlobal() {
  window.addEventListener('click', onWindowPointer, true)
  window.addEventListener('contextmenu', onWindowPointer, true)
  window.addEventListener('keydown', onKey)
  window.addEventListener('scroll', onWindowScroll, true)
  window.addEventListener('resize', onWindowScroll)
}

function unbindGlobal() {
  window.removeEventListener('click', onWindowPointer, true)
  window.removeEventListener('contextmenu', onWindowPointer, true)
  window.removeEventListener('keydown', onKey)
  window.removeEventListener('scroll', onWindowScroll, true)
  window.removeEventListener('resize', onWindowScroll)
}

watch(() => props.modelValue, (v) => {
  if (v) {
    activeIndex.value = -1
    ignoreTarget.value = window.event?.target ?? null
    // 先把位置设为初始值(带鼠标偏移), 让菜单先渲染出来
    adjustedX.value = (props.x ?? 0) + 4
    adjustedY.value = (props.y ?? 0) + 6
    nextTick(() => {
      // DOM 渲染后, 测量实际尺寸再校准位置
      adjustPosition()
      bindGlobal()
      setTimeout(() => {
        ignoreTarget.value = null
      }, 100)
    })
  } else {
    unbindGlobal()
    ignoreTarget.value = null
  }
})

onMounted(() => {
})

onBeforeUnmount(() => {
  unbindGlobal()
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
        :style="menuStyle"
      >
        <div class="mt-menu__content">
          <ul class="mt-menu__list">
          <li
            v-for="(item, idx) in items"
            :key="item.key ?? `i${idx}`"
            :class="[
              'mt-menu__row',
              {
                'mt-menu__row--divider': item.divider,
                'mt-menu__row--danger': item.danger,
                'mt-menu__row--disabled': item.disabled,
                'mt-menu__row--active': !item.disabled && !item.divider && activeIndex === idx
              },
            ]"
            role="menuitem"
            tabindex="0"
            @click="onItemClick(item)"
            @mouseenter="!item.disabled && !item.divider && (activeIndex = idx)"
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
  z-index: 9999;
  min-width: v-bind('`${minWidth}px`');
  max-width: 60vw;
  width: max-content;
}

.mt-menu__content {
  --mt-menu-blur: 48px;
  --mt-menu-bg: rgb(17 17 17 / 0.78);

  pointer-events: auto;
  border-radius: var(--radius-lg);
  padding: var(--sp-2);
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

.os-win10 .mt-menu__content {
  --mt-menu-bg: rgb(28 28 32 / 0.7);
  border-color: var(--border-default);
  box-shadow:
    0 8px 24px rgba(0, 0, 0, 0.4),
    0 2px 8px rgba(0, 0, 0, 0.2);
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
  background: transparent;
  transition:
    background 0.15s cubic-bezier(0.4, 0, 0.2, 1),
    color 0.15s cubic-bezier(0.4, 0, 0.2, 1),
    box-shadow 0.15s cubic-bezier(0.4, 0, 0.2, 1);
  min-height: 36px;
}

.mt-menu__row + .mt-menu__row {
  margin-top: 1px;
}

.mt-menu__row + .mt-menu__row--divider {
  margin-top: 4px;
}

.mt-menu__row--divider + .mt-menu__row {
  margin-top: 4px;
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

.mt-menu__row--active,
.mt-menu__row--active:hover {
  background: var(--interactive-hover);
  outline: none;
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
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
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
  flex-shrink: 0;
}

.mt-menu__row--divider {
  display: block;
  min-height: 0;
  height: 1px;
  padding: 0;
  margin: 0 14px;
  cursor: default;
  pointer-events: none;
  background: var(--border-subtle);
  border-radius: 0;
}

.mt-menu__divider {
  display: none;
}

/* ========== 动画 ========== */

.mt-menu__content {
  transform-origin: top center;
}

.mt-menu-enter-active .mt-menu__content {
  transition:
    transform 0.18s cubic-bezier(0.34, 1.12, 0.64, 1),
    opacity 0.18s cubic-bezier(0.34, 1.12, 0.64, 1);
}

.mt-menu-leave-active .mt-menu__content {
  transition:
    transform 0.14s var(--ease-out),
    opacity 0.14s var(--ease-out);
}

.mt-menu-enter-from .mt-menu__content {
  transform: scale(0.92) translateY(-6px);
  opacity: 0;
}

.mt-menu-leave-to .mt-menu__content {
  transform: scale(0.96) translateY(-4px);
  opacity: 0;
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
