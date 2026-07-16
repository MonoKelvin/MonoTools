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
        <div class="hotkey-modal__empty-icon" aria-hidden="true">
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor"
               stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <rect width="20" height="14" x="2" y="6" rx="2" />
            <path d="M6 10h.01" /><path d="M6 14h.01" />
            <path d="M10 10h.01" /><path d="M10 14h.01" />
            <path d="M14 10h.01" /><path d="M14 14h.01" />
          </svg>
        </div>
        <p>暂无快捷键</p>
      </div>
      <section
        v-for="(category, ci) in categories"
        :key="category"
        class="hotkey-modal__section"
        :style="{ '--section-delay': `${ci * 40}ms` }"
      >
        <h3 class="hotkey-modal__section-title">{{ category }}</h3>
        <ul class="hotkey-modal__list">
          <li
            v-for="(hotkey, hi) in getHotkeysByCategory(category)"
            :key="hotkey.id"
            class="hotkey-modal__item"
            :style="{ '--item-delay': `${ci * 40 + hi * 18 + 30}ms` }"
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
          </li>
        </ul>
      </section>
    </div>
  </MtModal>
</template>

<style scoped>
/* === 内层滚动区 ========================================================== */
.hotkey-modal__inner {
  max-height: calc(70vh - 80px);
  overflow-y: auto;
  overflow-x: hidden;
  padding: 4px 6px 4px 4px;
  margin: -4px -6px -4px -4px;
}

.hotkey-modal__inner::-webkit-scrollbar {
  width: 6px;
}
.hotkey-modal__inner::-webkit-scrollbar-thumb {
  background: var(--border-default);
  border-radius: var(--radius-full);
  transition: background var(--dur-fast) var(--ease-out);
}
.hotkey-modal__inner::-webkit-scrollbar-thumb:hover {
  background: var(--border-hover);
}
.hotkey-modal__inner::-webkit-scrollbar-track {
  background: transparent;
}

/* === 空状态 ============================================================== */
.hotkey-modal__empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-3);
  padding: var(--sp-9) 0;
  color: var(--text-quaternary);
  font-size: var(--text-sm);
}
.hotkey-modal__empty-icon {
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  background: var(--inset);
  border: 1px solid var(--border-subtle);
  color: var(--text-tertiary);
}

/* === 分类标题 ============================================================ */
.hotkey-modal__section {
  margin-bottom: var(--sp-5);
  /* 整个 section 也错落进入 */
  opacity: 0;
  transform: translateY(4px);
  animation: hotkey-section-in 280ms var(--ease-out) var(--section-delay, 0ms) forwards;
}
.hotkey-modal__section:last-child {
  margin-bottom: 0;
}

.hotkey-modal__section-title {
  font-size: var(--text-xs);
  font-weight: 600;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.08em;
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

/* === 列表 & 列表项 ====================================================== */
.hotkey-modal__list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.hotkey-modal__item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 7px 10px;
  border-radius: var(--radius-sm);
  transition:
    background var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
  /* 错落进入, 单项比 section 再晚一拍 */
  opacity: 0;
  transform: translateX(-4px);
  animation: hotkey-item-in 240ms var(--ease-out) var(--item-delay, 0ms) forwards;
}
.hotkey-modal__item:hover {
  background: var(--list-hover-bg);
  transform: translateX(2px);
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

/* === 键帽 (Fluent 2 风格: 1px 边框 + 顶部 1px 高光) ===================== */
.kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 24px;
  height: 22px;
  padding: 0 7px;
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
  background: var(--inset);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-xs);
  line-height: 1;
  /* 顶部 1px 高光 (Fluent 键帽标志性细节) + 底部 1px 暗线 */
  background-image: linear-gradient(
    180deg,
    rgba(255, 255, 255, 0.04) 0%,
    rgba(255, 255, 255, 0) 60%
  );
  box-shadow:
    0 1px 0 rgba(0, 0, 0, 0.18) inset,
    0 1px 0 rgba(255, 255, 255, 0.02) inset;
  transition:
    transform var(--dur-fast) var(--ease-out),
    border-color var(--dur-fast) var(--ease-out);
}

.hotkey-modal__item:hover .kbd {
  border-color: var(--border-default);
}

/* === 动画 ================================================================ */
@keyframes hotkey-section-in {
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes hotkey-item-in {
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

/* === 无障碍 ============================================================== */
@media (prefers-reduced-motion: reduce) {
  .hotkey-modal__section,
  .hotkey-modal__item {
    animation: none;
    opacity: 1;
    transform: none;
  }
  .hotkey-modal__item:hover {
    transform: none;
  }
}
</style>
