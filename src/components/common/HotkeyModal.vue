<script setup lang="ts">
import { computed } from 'vue'
import { Keyboard } from '@lucide/vue'
import { hotkeyManager } from '@/services/hotkeyManager'
import MtModal from './MtModal.vue'

interface Props {
  visible: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const categories = computed(() => hotkeyManager.getCategories())

const getHotkeysByCategory = (category: string) => {
  return hotkeyManager.getByCategory(category)
}
</script>

<template>
  <MtModal
    :visible="visible"
    title="快捷键"
    :icon="Keyboard"
    width="480px"
    max-height="580px"
    @close="emit('close')"
  >
    <div v-for="category in categories" :key="category" class="hotkey-modal__section">
      <h3 class="hotkey-modal__section-title">{{ category }}</h3>
      <div class="hotkey-modal__list">
        <div v-for="hotkey in getHotkeysByCategory(category)" :key="hotkey.id" class="hotkey-modal__item">
          <span class="hotkey-modal__description">{{ hotkey.description }}</span>
          <div class="hotkey-modal__keys">
            <span
              v-for="(part, idx) in hotkey.key.split(' + ')"
              :key="idx"
              class="kbd"
            >
              {{ part }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </MtModal>
</template>

<style scoped>
.hotkey-modal__section {
  margin-bottom: var(--sp-5);
}

.hotkey-modal__section:last-child {
  margin-bottom: 0;
}

.hotkey-modal__section-title {
  font-size: var(--text-xs);
  font-weight: 600;
  color: var(--text-quaternary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: var(--sp-2);
}

.hotkey-modal__list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.hotkey-modal__item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-2) var(--sp-3);
  border-radius: var(--radius-sm);
  transition: background var(--dur-fast) var(--ease-out);
}

.hotkey-modal__item:hover {
  background: rgba(255, 255, 255, 0.04);
}

.hotkey-modal__description {
  font-size: var(--text-sm);
  color: var(--text-secondary);
}

.hotkey-modal__keys {
  display: flex;
  align-items: center;
  gap: 4px;
}

.kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 24px;
  height: 24px;
  padding: 0 8px;
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  background: var(--surface-raised);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-xs);
  line-height: 1;
}
</style>