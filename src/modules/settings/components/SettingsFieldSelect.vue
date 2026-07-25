<script setup lang="ts">
import { computed } from 'vue'
import MtComboBox, { type MtComboBoxOption } from '@/ui/components/MtComboBox.vue'

const props = defineProps<{
  modelValue: string
  options: Array<{ label: string; value: string }>
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const comboBoxOptions = computed<MtComboBoxOption[]>(() =>
  props.options.map(o => ({ key: o.value, label: o.label })),
)

const currentValue = computed({
  get: () => props.modelValue,
  set: (val: string) => emit('update:modelValue', val),
})
</script>

<template>
  <MtComboBox
    :options="comboBoxOptions"
    :model-value="currentValue"
    :disabled="disabled"
    compact
    align="right"
    @update:model-value="(v: string) => emit('update:modelValue', v)"
  />
</template>
