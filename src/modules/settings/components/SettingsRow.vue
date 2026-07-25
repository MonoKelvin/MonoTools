<script setup lang="ts">
import { computed, h } from 'vue'
import type { SettingItemDef, SettingValue } from '../types'
import SettingsFieldToggle from './SettingsFieldToggle.vue'
import SettingsFieldSelect from './SettingsFieldSelect.vue'
import SettingsFieldMultiSelect from './SettingsFieldMultiSelect.vue'
import SettingsFieldHotkey from './SettingsFieldHotkey.vue'
import SettingsFieldText from './SettingsFieldText.vue'
import SettingsFieldNumber from './SettingsFieldNumber.vue'
import SettingsFieldPathList from './SettingsFieldPathList.vue'

const props = defineProps<{
  item: SettingItemDef
  modelValue: SettingValue
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: SettingValue): void
}>()

const currentValue = computed({
  get: () => props.modelValue ?? props.item.default,
  set: (val: SettingValue) => emit('update:modelValue', val),
})

function renderField() {
  switch (props.item.type) {
    case 'boolean':
      return h(SettingsFieldToggle, {
        modelValue: Boolean(currentValue.value),
        disabled: props.disabled,
        'onUpdate:modelValue': (val: boolean) => (currentValue.value = val),
      })
    case 'select':
      return h(SettingsFieldSelect, {
        modelValue: String(currentValue.value ?? ''),
        disabled: props.disabled,
        options: props.item.options ?? [],
        'onUpdate:modelValue': (val: string) => (currentValue.value = val),
      })
    case 'select-multi':
      return h(SettingsFieldMultiSelect, {
        modelValue: Array.isArray(currentValue.value) ? currentValue.value : [],
        disabled: props.disabled,
        options: props.item.options ?? [],
        'onUpdate:modelValue': (val: string[]) => (currentValue.value = val),
      })
    case 'hotkey':
      return h(SettingsFieldHotkey, {
        modelValue: String(currentValue.value ?? ''),
        disabled: props.disabled,
        'onUpdate:modelValue': (val: string) => (currentValue.value = val),
      })
    case 'string':
      return h(SettingsFieldText, {
        modelValue: String(currentValue.value ?? ''),
        disabled: props.disabled,
        placeholder: props.item.placeholder,
        'onUpdate:modelValue': (val: string) => (currentValue.value = val),
      })
    case 'number':
      return h(SettingsFieldNumber, {
        modelValue: Number(currentValue.value ?? props.item.default ?? 0),
        disabled: props.disabled,
        min: props.item.min,
        max: props.item.max,
        step: props.item.step,
        'onUpdate:modelValue': (val: number) => (currentValue.value = val),
      })
    case 'pathList':
      return h(SettingsFieldPathList, {
        modelValue: Array.isArray(currentValue.value) ? currentValue.value : [],
        disabled: props.disabled,
        'onUpdate:modelValue': (val: string[]) => (currentValue.value = val),
      })
    case 'info':
      return h('span', { class: 'settings-row__info-text' }, props.item.content ?? '')
    default:
      return null
  }
}
</script>

<template>
  <div class="settings-row">
    <div class="settings-row__info">
      <span class="settings-row__label">{{ item.label }}</span>
      <span v-if="item.description" class="settings-row__desc">{{ item.description }}</span>
    </div>
    <div class="settings-row__control">
      <component
        :is="renderField()"
        :key="item.key"
      />
    </div>
  </div>
</template>

<style scoped>
.settings-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-5);
  padding: var(--sp-4) 0;

  &:not(:last-child) {
    border-bottom: 1px solid var(--border-subtle);
  }
}

.settings-row__info {
  flex: 1;
  min-width: 0;
  margin-right: var(--sp-5);
}

.settings-row__label {
  display: block;
  font-size: var(--text-base);
  font-weight: 500;
  color: var(--text-primary);
  line-height: var(--leading-tight);
}

.settings-row__desc {
  display: block;
  font-size: var(--text-sm);
  font-weight: 400;
  color: var(--text-quaternary);
  line-height: var(--leading-normal);
  margin-top: 2px;
}

.settings-row__control {
  flex-shrink: 0;
}
</style>
