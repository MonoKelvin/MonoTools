<script setup lang="ts">
/**
 * EmptyState — 通用空状态组件 (Raycast / Linear 风格)
 *
 * 设计要点:
 * - 大号图标: 圆形 / 圆角容器 + 顶部 1px 高光 + 内阴影
 * - 标题 + 副标题分层, 间距克制
 * - 呼吸式微动画: 图标 6s 缓慢上下浮动
 * - 暗背景下图标用线性 stroke + 弱色, 避免与内容竞争注意力
 *
 * 用法:
 *   <EmptyState title="没有结果" hint="试试别的关键词" />
 *   <EmptyState title="输入关键字开始搜索" icon="search" />
 */
import { computed, type Component } from 'vue'
import { Search, SearchX, Inbox, AlertCircle, Sparkles, FileX, FolderX } from '@lucide/vue'

export type EmptyIcon = 'search' | 'no-results' | 'inbox' | 'error' | 'sparkles' | 'file' | 'folder'

const ICONS: Record<EmptyIcon, Component> = {
  search: Search,
  'no-results': SearchX,
  inbox: Inbox,
  error: AlertCircle,
  sparkles: Sparkles,
  file: FileX,
  folder: FolderX,
}

interface Props {
  /** 标题 */
  title: string
  /** 副标题 (次要描述) */
  hint?: string
  /** 图标 (默认 search) */
  icon?: EmptyIcon
  /** 自定义图标组件, 优先级最高 */
  iconComponent?: Component
  /** 整体垂直 padding, 默认可让 empty 占满结果区 */
  padding?: 'sm' | 'md' | 'lg' | 'xl'
}

const props = withDefaults(defineProps<Props>(), {
  icon: 'search',
  padding: 'lg',
})

const renderIcon = computed<Component>(() =>
  props.iconComponent ?? ICONS[props.icon],
)
</script>

<template>
  <div :class="['mt-empty', `mt-empty--${padding}`]">
    <div class="mt-empty__icon">
      <component :is="renderIcon" :size="26" :stroke-width="1.6" />
    </div>
    <div class="mt-empty__title">{{ title }}</div>
    <div v-if="hint" class="mt-empty__hint">{{ hint }}</div>
    <div v-if="$slots.default" class="mt-empty__actions">
      <slot />
    </div>
  </div>
</template>

<style scoped>
/* === 容器 =============================================================== */
.mt-empty {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-3);
  text-align: center;
  width: 100%;
  height: 100%;
  min-height: 0;
  animation: mt-empty-in 320ms var(--ease-out) both;
}

/* === 图标容器 ========================================================== */
.mt-empty__icon {
  position: relative;
  z-index: 1;
  width: 56px;
  height: 56px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-tertiary);
  background: transparent;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-lg);
}

/* === 文本 =============================================================== */
.mt-empty__title {
  position: relative;
  z-index: 1;
  color: var(--text-secondary);
  font-size: var(--text-base);
  font-weight: 500;
  letter-spacing: -0.005em;
  line-height: 1.4;
}

.mt-empty__hint {
  position: relative;
  z-index: 1;
  color: var(--text-quaternary);
  font-size: var(--text-sm);
  font-weight: 400;
  letter-spacing: 0.01em;
  max-width: 320px;
  line-height: 1.5;
}

.mt-empty__actions {
  position: relative;
  z-index: 1;
  margin-top: var(--sp-3);
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

/* === 动画 =============================================================== */
@keyframes mt-empty-in {
  from {
    opacity: 0;
    transform: translateY(6px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* === 无障碍 ============================================================ */
@media (prefers-reduced-motion: reduce) {
  .mt-empty,
  .mt-empty__icon {
    animation: none;
  }
}
</style>
