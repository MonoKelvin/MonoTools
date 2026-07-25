<script setup lang="ts">
import { ref, computed } from 'vue'
import { Check, ChevronDown } from '@lucide/vue'

const props = defineProps<{
  modelValue: string[]
  options: Array<{ label: string; value: string }>
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string[]): void
}>()

const open = ref(false)

const selectedSet = computed(() => new Set(props.modelValue ?? []))

const displayLabel = computed(() => {
  const vals = props.modelValue ?? []
  if (vals.length === 0) return '未选择'
  const labels = vals
    .map(v => props.options.find(o => o.value === v)?.label)
    .filter(Boolean)
  if (labels.length <= 2) return labels.join(', ')
  return `${labels.slice(0, 2).join(', ')} +${labels.length - 2}`
})

function toggleOption(value: string) {
  const current = new Set(props.modelValue ?? [])
  if (current.has(value)) {
    current.delete(value)
  } else {
    current.add(value)
  }
  emit('update:modelValue', Array.from(current))
}

function toggleDropdown() {
  if (props.disabled) return
  open.value = !open.value
}

function closeDropdown() {
  open.value = false
}
</script>

<template>
  <div class="settings-multi-select">
    <button
      type="button"
      class="settings-multi-select__trigger"
      :class="{ 'settings-multi-select__trigger--open': open }"
      :disabled="disabled"
      @click="toggleDropdown"
      @blur="closeDropdown"
    >
      <span class="settings-multi-select__label">{{ displayLabel }}</span>
      <ChevronDown
        :size="14"
        :stroke-width="1.5"
        class="settings-multi-select__arrow"
        :class="{ 'settings-multi-select__arrow--open': open }"
      />
    </button>

    <Transition name="scale">
      <div v-if="open" class="settings-multi-select__dropdown glass-card">
        <button
          v-for="option in options"
          :key="option.value"
          type="button"
          class="settings-multi-select__option"
          :class="{ 'settings-multi-select__option--selected': selectedSet.has(option.value) }"
          @mousedown.prevent="toggleOption(option.value)"
        >
          <span class="settings-multi-select__check">
            <Check v-if="selectedSet.has(option.value)" :size="14" :stroke-width="2" />
          </span>
          <span class="settings-multi-select__option-label">{{ option.label }}</span>
        </button>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.settings-multi-select {
  position: relative;
  min-width: 140px;
}

.settings-multi-select__trigger {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-2);
  padding: var(--sp-2) var(--sp-3);
  border-radius: var(--radius-sm);
  background: var(--surface-raised);
  border: 1px solid var(--border-default);
  color: var(--text-secondary);
  font-size: var(--text-sm);
  font-weight: 500;
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
  font-family: var(--font-sans);
}

.settings-multi-select__trigger:hover:not(:disabled) {
  border-color: var(--border-hover);
  color: var(--text-primary);
}

.settings-multi-select__trigger--open {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-glow);
}

.settings-multi-select__trigger:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.settings-multi-select__label {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 160px;
}

.settings-multi-select__arrow {
  color: var(--text-tertiary);
  transition: transform var(--dur-fast) var(--ease-out);
  flex-shrink: 0;
}

.settings-multi-select__arrow--open {
  transform: rotate(180deg);
}

.settings-multi-select__dropdown {
  position: absolute;
  top: calc(100% + 4px);
  right: 0;
  z-index: 100;
  min-width: 180px;
  padding: var(--sp-2);
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.settings-multi-select__option {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: var(--sp-2) var(--sp-3);
  border-radius: var(--radius-sm);
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
  font-family: var(--font-sans);
  width: 100%;
  text-align: left;
}

.settings-multi-select__option:hover {
  background: var(--list-hover-bg);
  color: var(--text-primary);
}

.settings-multi-select__option--selected {
  color: var(--text-primary);
}

.settings-multi-select__check {
  width: 16px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--accent);
  flex-shrink: 0;
}

.settings-multi-select__option-label {
  flex: 1;
}
</style>
