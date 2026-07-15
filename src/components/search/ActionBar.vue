<script setup lang="ts">
/**
 * 通用状态栏 (ActionBar).
 *
 * 这是一个**纯展示组件**: 只负责把外部传入的 {@link StatusBarMessage}
 * 渲染成图标 + 结构化文本 + (loading 态) 动态省略号, 并在消息切换时
 * 播放模糊渐变过渡. 组件本身不感知任何业务 (索引 / 选中 / 盘符 / 搜索),
 * 业务侧用 `useSearchStatusBar` 等编排器构建好消息后传入.
 *
 * 通用能力:
 * - 6 种语义类型图标: info / success / warning / error / loading / debug
 * - 文本片段按 kind 染色 (muted / label / primary / number / accent)
 * - loading 态自动追加 ".", "..", "..." 循环省略号
 * - 消息切换: A 横向拉伸模糊淡出 + B 自下方滑入 (50% → 100% 透明度)
 *
 * 右侧操作区通过 `#actions` 插槽暴露, 默认渲染导航/打开/快捷键提示,
 * 外部可覆盖以自定义.
 */

import { computed, onUnmounted, ref, watch, type Component } from 'vue'
import {
  ChevronUp, ChevronDown, CornerDownLeft, Keyboard,
  Loader2, CheckCircle, AlertCircle, Info, AlertTriangle, Bug,
} from '@lucide/vue'
import { FONT_SIZES, ICON_CONFIG } from '@/config'
import type { StatusBarMessage, StatusBarType } from '@/types/statusBar'

const props = defineProps<{
  /** 外部构建好的通用消息. null 时不渲染状态区. */
  message: StatusBarMessage | null
}>()

const emit = defineEmits<{
  (e: 'showHotkeys'): void
}>()

// === 类型 → 默认图标映射 =====================================================
// 通用语义图标, 不含任何业务. message.icon 可覆盖.
const DEFAULT_ICONS: Record<StatusBarType, Component> = {
  info: Info,
  success: CheckCircle,
  warning: AlertTriangle,
  error: AlertCircle,
  loading: Loader2,
  debug: Bug,
}

const isLoading = computed(() => props.message?.type === 'loading')

const statusIcon = computed<Component | null>(() => {
  if (!props.message) return null
  return props.message.icon ?? DEFAULT_ICONS[props.message.type]
})

// === 加载省略号: ".", "..", "..." 循环 =======================================
// 独立 ref, 不进入过渡 key, 避免每次跳动触发整体过渡.
const dotsText = ref('')
let dotsTimer: number | null = null

watch(
  isLoading,
  (loading) => {
    if (dotsTimer) {
      clearInterval(dotsTimer)
      dotsTimer = null
    }
    if (loading) {
      const cycle = ['.', '..', '...']
      let i = 0
      dotsText.value = cycle[0]
      dotsTimer = window.setInterval(() => {
        i = (i + 1) % cycle.length
        dotsText.value = cycle[i]
      }, 420)
    } else {
      dotsText.value = ''
    }
  },
  { immediate: true },
)

onUnmounted(() => {
  if (dotsTimer) clearInterval(dotsTimer)
})
</script>

<template>
  <div class="action-bar">
    <div class="action-bar__left">
      <div class="action-bar__status">
        <Transition name="status-swap">
          <span :key="message?.id ?? 'empty'" class="action-bar__status-text">
            <template v-if="message">
              <component
                :is="statusIcon"
                v-if="statusIcon"
                :size="12"
                :class="[
                  isLoading ? 'action-bar__status-spinner' : 'action-bar__status-icon',
                  `action-bar__status-icon--${message.type}`,
                ]"
              />
              <template v-for="(seg, i) in message.segments" :key="i">
                <span class="seg" :class="`seg--${seg.kind ?? 'label'}`">{{ seg.text }}</span>
              </template>
              <span v-if="isLoading" class="loading-dots">{{ dotsText }}</span>
            </template>
          </span>
        </Transition>
      </div>
    </div>

    <div class="action-bar__right">
      <slot name="actions">
        <span class="action-bar__hint">
          <span class="action-bar__keys">
            <span class="kbd"><ChevronUp :size="12" :stroke-width="2.5" /></span>
            <span class="kbd"><ChevronDown :size="12" :stroke-width="2.5" /></span>
          </span>
          <span class="action-bar__label">导航</span>
        </span>
        <span class="action-bar__hint">
          <span class="kbd"><CornerDownLeft :size="12" :stroke-width="2.5" /></span>
          <span class="action-bar__label">打开</span>
        </span>
        <button
          class="action-bar__hotkey-btn"
          @click="emit('showHotkeys')"
          v-tooltip="{ value: '查看快捷键', showDelay: ICON_CONFIG.tooltipDelayMs, position: 'top' }"
        >
          <Keyboard :size="14" :stroke-width="2" />
        </button>
      </slot>
    </div>
  </div>
</template>

<style scoped>
.action-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 14px;
  background: transparent;
  border-top: 1px solid var(--border-subtle);
  flex-shrink: 0;
  height: 30px;
}

.action-bar__left {
  display: flex;
  align-items: center;
  gap: var(--sp-5);
  min-width: 0;
  flex: 1;
}

.action-bar__right {
  display: flex;
  align-items: center;
  gap: var(--sp-4);
  flex-shrink: 0;
}

.action-bar__hint {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}

.action-bar__keys {
  display: inline-flex;
  align-items: center;
  gap: 2px;
}

.action-bar__label {
  color: var(--text-quaternary);
  font-size: 10.5px;
  font-weight: 500;
  letter-spacing: 0.02em;
}

/* === 状态文本: 纯文本, 无胶囊背景 === */
.action-bar__status {
  position: relative;
  display: flex;
  align-items: center;
  min-width: 0;
  overflow: hidden;
  max-width: 360px;
}

.action-bar__status-text {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--text-tertiary);
  font-size: v-bind('FONT_SIZES.sm + "px"');
  font-weight: 400;
  white-space: nowrap;
  will-change: transform, opacity, filter;
}

/* === 片段染色: 简约灰阶 + 金色高亮数字 === */
.seg {
  display: inline;
}

.seg--muted {
  color: var(--text-quaternary);
}

.seg--label {
  color: var(--text-tertiary);
}

.seg--primary {
  color: var(--text-primary);
  font-weight: 500;
}

.seg--accent {
  color: var(--text-secondary);
}

.seg--number {
  color: var(--accent-warm);
  font-weight: 500;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.01em;
}

/* === 加载省略号: 跟随数字色 === */
.loading-dots {
  color: var(--accent-warm);
  font-weight: 600;
  letter-spacing: 1px;
  margin-left: 1px;
  display: inline-block;
  width: 1.4em;
  text-align: left;
}

/* === 图标: 按 type 染色 === */
.action-bar__status-spinner {
  animation: spin 1s linear infinite;
  color: var(--accent-warm);
  flex-shrink: 0;
}

.action-bar__status-icon {
  flex-shrink: 0;
}

.action-bar__status-icon--info,
.action-bar__status-icon--debug {
  color: var(--text-tertiary);
}

.action-bar__status-icon--success {
  color: var(--text-primary);
}

.action-bar__status-icon--warning {
  color: var(--accent-warm);
}

.action-bar__status-icon--error {
  color: var(--color-danger);
}

.action-bar__status-icon--loading {
  color: var(--accent-warm);
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* === 文本切换过渡 ============================================================
 * A → B: A 横向拉伸 + 模糊 + 淡出 (绝对定位脱离文档流), B 从下方滑入并从
 * 50% 透明度渐变到正常. 交叉过渡无空档, 快速 (≈160ms) 以适配方向键连按.
 */
.status-swap-leave-active {
  position: absolute;
  left: 0;
  transition:
    opacity 100ms var(--ease-in-out),
    transform 100ms var(--ease-in-out),
    filter 100ms var(--ease-in-out);
}

.status-swap-leave-to {
  opacity: 0;
  transform: translateX(5px) scaleX(1.05);
  filter: blur(2.5px);
}

.status-swap-enter-active {
  position: relative;
  transition:
    opacity 130ms var(--ease-out),
    transform 130ms var(--ease-out),
    filter 130ms var(--ease-out);
}

.status-swap-enter-from {
  opacity: 0.4;
  transform: translateY(3px);
  filter: blur(0.4px);
}

.kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  font-family: var(--font-mono);
  font-size: v-bind('FONT_SIZES.xxs + "px"');
  color: var(--text-tertiary);
  background: var(--inset);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  line-height: 1;
}

.action-bar__hotkey-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  background: transparent;
  border: none;
  border-radius: 6px;
  color: var(--text-tertiary);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
}

.action-bar__hotkey-btn:hover {
  background: var(--list-hover-bg);
  color: var(--text-secondary);
}
</style>
