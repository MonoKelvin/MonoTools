<script setup lang="ts">
import type { SettingGroupDef } from '../types'
import type { SettingValue } from '../types'
import SettingsRow from './SettingsRow.vue'
import MtCard from '@/ui/components/MtCard.vue'

const props = defineProps<{
  group: SettingGroupDef
  values: Record<string, SettingValue>
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:item-value', key: string, value: SettingValue): void
}>()

function updateItemValue(key: string, value: SettingValue) {
  emit('update:item-value', key, value)
}
</script>

<template>
  <MtCard class="settings-group">
    <header class="settings-group__header">
      <h3 class="settings-group__title">{{ group.label }}</h3>
      <p v-if="group.description" class="settings-group__description">{{ group.description }}</p>
    </header>
    <div class="settings-group__body">
      <SettingsRow
        v-for="item in group.items"
        :key="item.key"
        :item="item"
        :model-value="values[item.key] ?? item.default"
        :disabled="loading"
        @update:model-value="updateItemValue(item.key, $event)"
      />
      <div v-if="group.items.length === 0" class="settings-group__empty">
        暂无设置项
      </div>
    </div>
  </MtCard>
</template>

<style scoped>
.settings-group {
  margin-bottom: var(--sp-4);
}

.settings-group__header {
  margin-bottom: var(--sp-4);
}

.settings-group__title {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.06em;
  line-height: var(--leading-tight);
  margin: 0 0 var(--sp-2) 0;
}

.settings-group__description {
  font-size: var(--text-base);
  font-weight: 400;
  color: var(--text-secondary);
  line-height: var(--leading-normal);
  margin: 0;
}

.settings-group__body {
  display: flex;
  flex-direction: column;
}

.settings-group__empty {
  padding: var(--sp-4) 0;
  font-size: var(--text-sm);
  color: var(--text-quaternary);
  text-align: center;
}
</style>
