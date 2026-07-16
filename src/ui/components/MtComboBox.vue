<script setup lang="ts">
import { onBeforeUnmount, onMounted, computed, ref, nextTick, watch } from 'vue'
import type { Component } from 'vue'
import { ChevronDown } from '@lucide/vue'

export interface MtComboBoxOption {
  key?: string
  label?: string
  icon?: Component
  disabled?: boolean
  divider?: boolean
  custom?: string
}

const props = withDefaults(
  defineProps<{
    options: MtComboBoxOption[]
    modelValue?: string
    placeholder?: string
    disabled?: boolean
    minWidth?: number
    align?: 'left' | 'right'
    dropdownAnchor?: string
    compact?: boolean
  }>(),
  {
    placeholder: '请选择',
    minWidth: 140,
    align: 'right',
    dropdownAnchor: '',
    compact: false,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
  (e: 'select', option: MtComboBoxOption): void
}>()

const rootRef = ref<HTMLElement | null>(null)
const dropdownRef = ref<HTMLElement | null>(null)
const activeIndex = ref(-1)
const open = ref(false)
const dropdownStyle = ref<Record<string, string>>({})
const isHovered = ref(false)

const isCompactExpanded = computed(() => props.compact && (open.value || isHovered.value))

const displayArrow = computed(() => true)

const selectedOption = computed(() => {
  return props.options.find((o) => o.key === props.modelValue && !o.divider)
})

const displayLabel = computed(() => {
  return selectedOption.value?.label || props.placeholder
})

const displayIcon = computed(() => {
  return selectedOption.value?.icon
})

function toggleDropdown() {
  if (props.disabled) return
  open.value = !open.value
}

function closeDropdown() {
  open.value = false
}

function selectOption(option: MtComboBoxOption) {
  if (option.disabled || option.divider) return
  if (option.key !== undefined) {
    emit('update:modelValue', option.key)
  }
  emit('select', option)
  closeDropdown()
}

function getEnabledIndices(): number[] {
  return props.options
    .map((item, idx) => ({ item, idx }))
    .filter(({ item }) => !item.disabled && !item.divider)
    .map(({ idx }) => idx)
}

function onKeyDown(event: KeyboardEvent) {
  if (props.disabled) return

  if (!open.value) {
    if (event.key === 'Enter' || event.key === ' ' || event.key === 'ArrowDown') {
      event.preventDefault()
      open.value = true
      return
    }
    return
  }

  if (event.key === 'Escape') {
    event.preventDefault()
    closeDropdown()
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
    if (activeIndex.value >= 0 && props.options[activeIndex.value]) {
      selectOption(props.options[activeIndex.value])
    }
    return
  }
}

function onDocumentClick(event: MouseEvent) {
  if (!rootRef.value) return
  if (rootRef.value.contains(event.target as Node)) return
  if (dropdownRef.value?.contains(event.target as Node)) return
  closeDropdown()
}

function updateDropdownPosition() {
  if (!rootRef.value || !dropdownRef.value) return
  const triggerRect = rootRef.value.getBoundingClientRect()

  let anchorRect: DOMRect | null = null
  if (props.dropdownAnchor) {
    const anchorEl = document.querySelector(props.dropdownAnchor)
    if (anchorEl) anchorRect = anchorEl.getBoundingClientRect()
  }
  if (!anchorRect) {
    const parentEl = rootRef.value.parentElement
    if (parentEl) anchorRect = parentEl.getBoundingClientRect()
  }
  if (!anchorRect) return

  const estimatedWidth = Math.max(triggerRect.width, props.minWidth)
  let left = props.align === 'right'
    ? triggerRect.right - anchorRect.left - estimatedWidth
    : triggerRect.left - anchorRect.left

  const anchorWidth = anchorRect.width
  if (left + estimatedWidth > anchorWidth - 8) {
    left = anchorWidth - estimatedWidth - 8
  }
  if (left < 8) left = 8

  dropdownStyle.value = {
    left: `${left}px`,
    top: `${triggerRect.bottom - anchorRect.top + 4}px`,
    minWidth: `${estimatedWidth}px`,
  }
}

watch(open, (isOpen) => {
  if (isOpen) {
    activeIndex.value = -1
    nextTick(() => {
      updateDropdownPosition()
      document.addEventListener('click', onDocumentClick, true)
      document.addEventListener('keydown', onKeyDown)
      window.addEventListener('resize', closeDropdown)
      window.addEventListener('scroll', closeDropdown, true)
    })
  } else {
    document.removeEventListener('click', onDocumentClick, true)
    document.removeEventListener('keydown', onKeyDown)
    window.removeEventListener('resize', closeDropdown)
    window.removeEventListener('scroll', closeDropdown, true)
    dropdownStyle.value = {}
  }
})

onMounted(() => {
  if (rootRef.value) {
    const initial = Math.max(rootRef.value.getBoundingClientRect().width, props.minWidth)
    dropdownStyle.value = { minWidth: `${initial}px` }
  }
})

onBeforeUnmount(() => {
  document.removeEventListener('click', onDocumentClick, true)
  document.removeEventListener('keydown', onKeyDown)
  window.removeEventListener('resize', closeDropdown)
  window.removeEventListener('scroll', closeDropdown, true)
})
</script>

<template>
  <div
    ref="rootRef"
    class="mt-combobox"
    :class="{
      'mt-combobox--open': open,
      'mt-combobox--disabled': disabled,
      'mt-combobox--compact': compact,
      'mt-combobox--compact-expanded': isCompactExpanded,
    }"
  >
    <button
      type="button"
      class="mt-combobox__trigger"
      :disabled="disabled"
      @click="toggleDropdown"
      @keydown="onKeyDown"
      @mouseenter="!disabled && compact && (isHovered = true)"
      @mouseleave="isHovered = false"
    >
      <component :is="displayIcon" v-if="displayIcon" :size="14" class="mt-combobox__trigger-icon" />
      <span class="mt-combobox__trigger-label" :class="{ 'mt-combobox__trigger-label--placeholder': !selectedOption }">
        {{ displayLabel }}
      </span>
      <ChevronDown v-if="displayArrow" :size="12" class="mt-combobox__trigger-arrow" />
    </button>

    <template v-if="dropdownAnchor">
      <Teleport :to="dropdownAnchor">
        <Transition name="mt-combobox-dropdown">
          <div
            v-if="open"
            ref="dropdownRef"
            class="mt-combobox__dropdown"
            :style="dropdownStyle"
          >
            <div class="mt-combobox__content">
              <ul class="mt-combobox__list">
                <li
                  v-for="(option, idx) in options"
                  :key="option.key ?? `i${idx}`"
                  :class="[
                    'mt-combobox__option',
                    {
                      'mt-combobox__option--divider': option.divider,
                      'mt-combobox__option--disabled': option.disabled,
                      'mt-combobox__option--active': !option.disabled && !option.divider && activeIndex === idx,
                      'mt-combobox__option--selected': option.key === modelValue,
                    },
                  ]"
                  @click="selectOption(option)"
                  @mouseenter="!option.disabled && !option.divider && (activeIndex = idx)"
                >
                  <template v-if="option.divider">
                    <span class="mt-combobox__divider" />
                  </template>
                  <template v-else>
                    <component :is="option.icon" v-if="option.icon" class="mt-combobox__option-icon" :size="14" :stroke-width="2" />
                    <span class="mt-combobox__option-label">{{ option.label }}</span>
                    <slot v-if="option.custom" :name="option.custom" />
                  </template>
                </li>
              </ul>
            </div>
          </div>
        </Transition>
      </Teleport>
    </template>

    <template v-if="!dropdownAnchor">
      <Transition name="mt-combobox-dropdown">
        <div
          v-if="open"
          ref="dropdownRef"
          class="mt-combobox__dropdown"
          :style="dropdownStyle"
        >
          <div class="mt-combobox__content">
            <ul class="mt-combobox__list">
              <li
                v-for="(option, idx) in options"
                :key="option.key ?? `i${idx}`"
                :class="[
                  'mt-combobox__option',
                  {
                    'mt-combobox__option--divider': option.divider,
                    'mt-combobox__option--disabled': option.disabled,
                    'mt-combobox__option--active': !option.disabled && !option.divider && activeIndex === idx,
                    'mt-combobox__option--selected': option.key === modelValue,
                  },
                ]"
                @click="selectOption(option)"
                @mouseenter="!option.disabled && !option.divider && (activeIndex = idx)"
              >
                <template v-if="option.divider">
                  <span class="mt-combobox__divider" />
                </template>
                <template v-else>
                  <component :is="option.icon" v-if="option.icon" class="mt-combobox__option-icon" :size="14" :stroke-width="2" />
                  <span class="mt-combobox__option-label">{{ option.label }}</span>
                  <slot v-if="option.custom" :name="option.custom" />
                </template>
              </li>
            </ul>
          </div>
        </div>
      </Transition>
    </template>
  </div>
</template>

<style scoped>
.mt-combobox {
  position: relative;
  display: inline-block;
}

.mt-combobox__trigger {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: var(--radius-sm);
  background: transparent;
  border: 1px solid transparent;
  color: var(--text-tertiary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
  user-select: none;
  white-space: nowrap;
}

.mt-combobox__trigger:hover:not(:disabled) {
  background: var(--interactive-hover);
  color: var(--text-secondary);
  border-color: var(--border-subtle);
}

.mt-combobox__trigger:focus-visible {
  outline: none;
  background: var(--interactive-hover);
  border-color: var(--accent);
  color: var(--text-secondary);
}

.mt-combobox--open .mt-combobox__trigger {
  background: var(--interactive-hover);
  color: var(--text-secondary);
  border-color: var(--border-subtle);
}

.mt-combobox__trigger:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.mt-combobox__trigger-icon {
  flex-shrink: 0;
}

.mt-combobox__trigger-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mt-combobox__trigger-label--placeholder {
  color: var(--text-quaternary);
}

.mt-combobox__trigger-arrow {
  flex-shrink: 0;
  transition: transform var(--dur-fast) var(--ease-out);
}

.mt-combobox--open .mt-combobox__trigger-arrow {
  transform: rotate(180deg);
}

/* ========== Compact 模式 ========== */

.mt-combobox--compact .mt-combobox__trigger {
  padding-left: 8px;
  padding-right: 8px;
  background: var(--surface-elevated, rgba(255, 255, 255, 0.04));
}

.mt-combobox--compact .mt-combobox__trigger-label {
  max-width: 0;
  opacity: 0;
  transform: translateX(-4px);
  transition:
    max-width 260ms cubic-bezier(0.4, 0, 0.2, 1),
    opacity 140ms ease-out 90ms,
    transform 260ms cubic-bezier(0.4, 0, 0.2, 1) 70ms,
    margin 260ms cubic-bezier(0.4, 0, 0.2, 1) 70ms;
  margin-left: 0;
}

.mt-combobox--compact.mt-combobox--compact-expanded .mt-combobox__trigger-label {
  max-width: 160px;
  opacity: 1;
  transform: translateX(0);
  margin-left: 2px;
  transition:
    max-width 260ms cubic-bezier(0.4, 0, 0.2, 1) 100ms,
    opacity 140ms ease-out 100ms,
    transform 260ms cubic-bezier(0.4, 0, 0.2, 1) 100ms,
    margin 260ms cubic-bezier(0.4, 0, 0.2, 1) 100ms;
}

@media (prefers-reduced-motion: reduce) {
  .mt-combobox--compact .mt-combobox__trigger-label {
    transition: none;
  }
}

.mt-combobox__dropdown {
  position: absolute;
  z-index: 9999;
}

.mt-combobox__content {
  --mt-combobox-blur: 40px;
  --mt-combobox-bg: rgb(22 22 32 / 0.85);

  pointer-events: auto;
  border-radius: var(--radius-lg);
  padding: var(--sp-2);
  background: var(--mt-combobox-bg);
  backdrop-filter: blur(var(--mt-combobox-blur)) saturate(180%);
  -webkit-backdrop-filter: blur(var(--mt-combobox-blur)) saturate(180%);
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

.os-win10 .mt-combobox__content {
  --mt-combobox-bg: rgb(28 28 32 / 0.92);
  border-color: var(--border-default);
  box-shadow:
    0 8px 24px rgba(0, 0, 0, 0.4),
    0 2px 8px rgba(0, 0, 0, 0.2);
}

.mt-combobox__list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
}

.mt-combobox__option {
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
    color 0.15s cubic-bezier(0.4, 0, 0.2, 1);
  min-height: 36px;
}

.mt-combobox__option + .mt-combobox__option {
  margin-top: 1px;
}

.mt-combobox__option + .mt-combobox__option--divider {
  margin-top: 4px;
}

.mt-combobox__option--divider + .mt-combobox__option {
  margin-top: 4px;
}

.mt-combobox__option:hover:not(.mt-combobox__option--divider):not(.mt-combobox__option--disabled) {
  background: var(--interactive-hover);
}

.mt-combobox__option--disabled {
  color: var(--text-quaternary);
  cursor: default;
}

.mt-combobox__option--active,
.mt-combobox__option--active:hover {
  background: var(--interactive-hover);
  outline: none;
}

.mt-combobox__option--selected {
  color: var(--accent);
}

.mt-combobox__option--selected .mt-combobox__option-icon {
  color: var(--accent);
}

.mt-combobox__option-icon {
  color: var(--text-secondary);
  flex-shrink: 0;
}

.mt-combobox__option-label {
  flex: 1;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mt-combobox__option--divider {
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

.mt-combobox__divider {
  display: none;
}

/* ========== 动画 ========== */

.mt-combobox-dropdown-enter-active .mt-combobox__content {
  transition:
    transform 0.18s cubic-bezier(0.34, 1.12, 0.64, 1),
    opacity 0.18s cubic-bezier(0.34, 1.12, 0.64, 1);
}

.mt-combobox-dropdown-leave-active .mt-combobox__content {
  transition:
    transform 0.14s var(--ease-out),
    opacity 0.14s var(--ease-out);
}

.mt-combobox-dropdown-enter-from .mt-combobox__content {
  transform: scale(0.94) translateY(-4px);
  opacity: 0;
}

.mt-combobox-dropdown-leave-to .mt-combobox__content {
  transform: scale(0.97) translateY(-2px);
  opacity: 0;
}

@media (prefers-reduced-motion: reduce) {
  .mt-combobox-dropdown-enter-active .mt-combobox__content,
  .mt-combobox-dropdown-leave-active .mt-combobox__content {
    transition: none;
  }

  .mt-combobox-dropdown-enter-from .mt-combobox__content,
  .mt-combobox-dropdown-leave-to .mt-combobox__content {
    transform: none;
  }
}
</style>
