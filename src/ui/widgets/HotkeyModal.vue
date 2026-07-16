<script setup lang="ts">
import { computed } from 'vue'
import { Keyboard } from '@lucide/vue'
import { hotkeyManager } from '@/services/hotkeyManager'
import { useCommandsStore } from '@/core/command/store'
import MtModal from '@/ui/components/MtModal.vue'

interface Props {
  visible: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const commandsStore = useCommandsStore()

// 当面板打开 & 命令尚未加载时，触发一次拉取，以确保 hotkeyManager 有内容可显示。
// 面板内 props.visible 变化也会重新计算（响应式）。
const categories = computed(() => {
  // 依赖 commandsStore.specs 让 hotkeyManager 与面板同步刷新
  void commandsStore.specs
  return hotkeyManager.getCategories()
})

const getHotkeysByCategory = (category: string) => {
  return hotkeyManager.getByCategory(category)
}

if (!commandsStore.isLoaded) {
  void commandsStore.loadFromBackend().catch(() => undefined)
}
</script>

<!-- Hotkey 面板使用全屏宽度的 modal body, 内容自带滚动 -->
<template>
  <MtModal
    :visible="visible"
    title="快捷键"
    :icon="Keyboard"
    width="520px"
    max-height="70vh"
    @close="emit('close')"
  >
    <div class="hotkey-modal__inner">
      <div v-if="categories.length === 0" class="hotkey-modal__empty">
        暂无快捷键
      </div>
      <div v-for="category in categories" :key="category" class="hotkey-modal__section">
        <h3 class="hotkey-modal__section-title">{{ category }}</h3>
        <div class="hotkey-modal__list">
          <div
            v-for="hotkey in getHotkeysByCategory(category)"
            :key="hotkey.id"
            class="hotkey-modal__item"
          >
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
    </div>
  </MtModal>
</template>

<style scoped>
/* 内层容器:高度固定 + 滚动, 避免整个 modal 因内容超出而拉伸. */
.hotkey-modal__inner {
  max-height: calc(70vh - 80px);
  overflow-y: auto;
  overflow-x: hidden;
  padding-right: 4px;
}

/* 滚动条: webkit 玻璃感. */
.hotkey-modal__inner::-webkit-scrollbar {
  width: 6px;
}
.hotkey-modal__inner::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.12);
  border-radius: 999px;
}
.hotkey-modal__inner::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.2);
}
.hotkey-modal__inner::-webkit-scrollbar-track {
  background: transparent;
}

.hotkey-modal__empty {
  font-size: var(--text-sm);
  color: var(--text-quaternary);
  padding: var(--sp-5) 0;
  text-align: center;
}

.hotkey-modal__section {
  margin-bottom: var(--sp-5);
}

.hotkey-modal__section:last-child {
  margin-bottom: 0;
}

.hotkey-modal__section-title {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.07em;
  margin-bottom: var(--sp-3);
  display: flex;
  align-items: center;
  gap: 6px;
}

.hotkey-modal__section-title::before {
  content: '';
  width: 3px;
  height: 3px;
  background: var(--text-muted);
  border-radius: 50%;
  flex-shrink: 0;
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
  padding: 7px 10px;
  border-radius: var(--radius-md);
  transition: background var(--dur-fast) var(--ease-out);
}

.hotkey-modal__item:hover {
  background: rgba(255, 255, 255, 0.04);
}

.hotkey-modal__description {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  font-weight: 400;
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
  font-size: 11.5px;
  font-weight: 500;
  color: var(--text-secondary);
  background: var(--inset);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-xs);
  line-height: 1;
  box-shadow: 0 1px 0 rgba(255, 255, 255, 0.02) inset;
}
</style>