<script setup lang="ts">
/**
 * AppResultItem —— 专门用于"应用"类搜索结果.
 *
 * 与通用 ResultItem 的差异:
 * - **不显示副标题 (路径)**: 应用结果路径属于内部细节, 不向用户暴露.
 * - **图标优先**: 图标分辨率更高 (28x28), 占视觉重点.
 * - **三态图标**: 静态 SVG (立即) / 后端 PNG (异步) / Lucide 通用兜底 / monogram 兜底.
 * - **更大标题字号 (14px / 600)**: 强化应用名识别度.
 * - **省略号截断**: 标题超出时自动 ... 显示.
 *
 * 与 ResultItem 共享选中态 / 悬停态 / 快捷键 ↵ 行为, 可直接在列表中替换.
 *
 * 图标渲染委托给 `useIconRenderer` composable, 与 ResultItem 共享同一套
 * 4-tier 加载链 + 350ms 兜底 timer + loadToken race 防护. 详见
 * `src/composables/useIconRenderer.ts`.
 */
import { computed, onBeforeUnmount, onMounted, ref, watch, nextTick } from 'vue'
import { AppWindow, Monitor, Smartphone } from '@lucide/vue'
import type { SearchResult } from '@/modules/search'
import { useIconRenderer } from '@/ui/widgets/appicon/useIconRenderer'
import { useAdaptiveText } from '@/utils/adaptiveText'
import { FONT_SIZES } from '@/core/config'

const props = withDefaults(defineProps<{
  result: SearchResult
  active?: boolean
  index: number
  /** 徽章尺寸: sm 用于列表/网格, xs 用于图标模式 */
  badgeSize?: 'sm' | 'xs'
  /** 标题是否允许换行: 默认 false (单行省略号), true 时可换行显示完整内容 */
  titleWrap?: boolean
  /** 是否禁止字体缩小: 默认 false (可缩小), true 时始终使用最大字号 */
  noFontShrink?: boolean
  /** 是否禁用 tooltip: 由父容器统一处理时设为 true, 避免重复 */
  noTooltip?: boolean
  /** 当前分组排序模式 */
  sortMode?: string
}>(), {
  badgeSize: 'sm',
})

// 事件全部由父容器 (VirtualGroupedResults 的行 div) 统一处理, 这里只做展示.
// 仅声明 emit 留作扩展点: 右键菜单等场景需要时再使用.
const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
  (e: 'open', item: SearchResult): void
  (e: 'contextmenu', ev: MouseEvent, item: SearchResult): void
}>()
// 抑制 "声明但未使用" 警告 —— 保留 emit 以便父级透传扩展事件.
void emit

/**
 * 图标渲染 composable —— 封装 loadToken / imgFallback / isSame / onImgLoad
 * 等所有"图标状态机"逻辑. AppResultItem 只关心如何展示, 不关心如何获取.
 */
const { iconState, imgReady, refresh, onImgLoad, onImgError, dispose } = useIconRenderer({
  fallbackComponent: AppWindow,
  containerSelector: (id) => `[data-app-result-id="${id}"] img`,
  debugTag: 'AppResultItem',
})

// 挂载 + result 变化时触发图标加载
onMounted(() => refresh(props.result))
watch(() => props.result?.id, () => refresh(props.result))
// 显式调用 dispose (composable 已挂 onBeforeUnmount, 此处为对称性)
onBeforeUnmount(dispose)

const isSystemApp = computed(() => props.result?.resultType === 'system-app')
const isUwpApp = computed(() => props.result?.resultType === 'uwp-app')

const badgeInfo = computed(() => {
  if (isUwpApp.value) {
    return { icon: Smartphone, label: 'UWP 应用', type: 'uwp', color: 'hsl(215, 55%, 72%)', borderColor: 'hsla(215, 55%, 68%, 0.45)' }
  }
  if (isSystemApp.value) {
    return { icon: Monitor, label: '系统应用', type: 'system', color: 'hsl(220, 10%, 65%)', borderColor: 'hsla(220, 10%, 58%, 0.35)' }
  }
  return null
})

const badgeEl = ref<HTMLElement | null>(null)

/**
 * 自适应标题文字: 优先缩小字体, 超出则省略号, hover 显示 tooltip.
 * titleWrap=true 时允许多行显示, 尽量展示完整内容.
 * noFontShrink=true 时始终使用最大字号, 不缩小字体.
 */
const {
  containerRef: titleContainerRef,
  displayText: adaptiveTitle,
  displayLines: adaptiveTitleLines,
  currentFontSize: _titleFontSize,
  isTruncated: titleIsTruncated,
  whiteSpaceMode,
  update: updateAdaptiveText,
} = useAdaptiveText(() => props.result?.title || '', {
  maxFontSize: 14,
  minFontSize: 10,
  fontWeight: '600',
  padding: 4,
  whiteSpace: 'nowrap',
  maxLines: undefined,
})

const titleFontSize = computed(() => props.noFontShrink ? 14 : _titleFontSize.value)

watch(
  () => props.titleWrap,
  (wrap) => {
    whiteSpaceMode.value = wrap ? 'normal' : 'nowrap'
    nextTick(() => updateAdaptiveText())
  },
  { immediate: true }
)
</script>

<template>
  <div
    :class="['app-result-item', { 'app-result-item--active': active }]"
    :data-app-result-id="result?.id"
  >
    <div class="app-result-item__icon">
      <img
        v-if="iconState.kind === 'svg' || iconState.kind === 'png'"
        :src="iconState.value"
        class="app-result-item__img"
        :class="{ 'app-result-item__img--ready': imgReady }"
        @load="onImgLoad"
        @error="onImgError"
        decoding="async"
        draggable="false"
        alt=""
      />
      <div
        v-else-if="iconState.kind === 'monogram'"
        class="app-result-item__monogram"
        :style="{ background: iconState.color }"
        :data-letter="iconState.letter"
      >
        {{ iconState.letter }}
      </div>
      <component
        v-else
        :is="iconState.value"
        :size="18"
        :stroke-width="1.7"
        class="app-result-item__lucide"
      />
      <div
        v-if="badgeInfo"
        ref="badgeEl"
        class="app-result-item__badge"
        :class="[
          `app-result-item__badge--${badgeInfo.type}`,
          `app-result-item__badge--${badgeSize ?? 'sm'}`
        ]"
      >
        <component :is="badgeInfo.icon" :size="badgeSize === 'xs' ? 12 : 10" :stroke-width="2" />
      </div>
    </div>

    <div
      class="app-result-item__title"
      :class="{ 'app-result-item__title--wrap': titleWrap }"
      ref="titleContainerRef"
      :style="{ fontSize: titleFontSize + 'px' }"
    >
      <template v-if="noFontShrink">
        {{ result?.title || '' }}
      </template>
      <template v-else-if="titleWrap && adaptiveTitleLines.length > 1">
        <span v-for="(line, idx) in adaptiveTitleLines" :key="idx" class="app-title-line">{{ line }}</span>
      </template>
      <template v-else>
        {{ adaptiveTitle }}
      </template>
    </div>

    <div class="app-result-item__meta">
    </div>
  </div>
</template>

<style scoped>
.app-result-item {
  display: flex;
  align-items: center;
  gap: var(--sp-4);
  padding: 5px 14px;
  cursor: pointer;
  user-select: none;
  background: transparent;
  position: relative;
  overflow: visible;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  transition:
    background var(--dur-fast) var(--ease-out),
    border-color var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
}

.app-result-item:hover {
  background: var(--list-hover-bg);
}

.app-result-item--active {
  background: var(--list-selected-bg);
}

.app-result-item__icon {
  flex-shrink: 0;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  background: rgba(255, 255, 255, 0.02);
  color: var(--text-tertiary);
  overflow: visible;
  position: relative;
  transition:
    color var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out);
}

.app-result-item:hover .app-result-item__icon {
  color: var(--text-secondary);
  background: rgba(255, 255, 255, 0.04);
  transform: scale(1.04);
}

.app-result-item--active .app-result-item__icon {
  color: var(--accent);
  background: var(--accent-soft);
  filter: drop-shadow(0 0 6px var(--accent-glow));
  transform: scale(1.06);
}

.app-result-item__img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  padding: 2px;
  opacity: 0;
  transition: opacity var(--dur-slow) var(--ease-out);
  pointer-events: none;
}
.app-result-item__img--ready { opacity: 1; }

.app-result-item__lucide { pointer-events: none; }

/* === Badge 角标 (system / uwp) === */
.app-result-item__badge {
  position: absolute;
  right: -4px;
  bottom: -4px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--canvas-elevated);
  border: 1px solid var(--border-subtle);
  color: var(--text-tertiary);
  box-shadow: var(--shadow-sm);
  z-index: 2;
  pointer-events: none;
  transition:
    color var(--dur-fast) var(--ease-out),
    border-color var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out);
}

.app-result-item__badge--sm { width: 16px; height: 16px; border-radius: var(--radius-xs); }
.app-result-item__badge--xs { width: 18px; height: 18px; border-radius: var(--radius-xs); }

.app-result-item:hover .app-result-item__badge {
  color: var(--text-secondary);
  border-color: var(--border-default);
}

/* system 在 hover/active 时也变灰 */
.app-result-item:hover .app-result-item__badge--system,
.app-result-item--active .app-result-item__badge--system {
  color: var(--text-secondary);
  border-color: var(--border-default);
}

/* uwp hover/active 不覆盖颜色，保留内联的浅灰蓝色 */
.app-result-item:hover .app-result-item__badge--uwp,
.app-result-item--active .app-result-item__badge--uwp {
  transform: scale(1.06);
}

/* === Monogram 单字母占位符 (无真实图标时) === */
.app-result-item__monogram {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  font-size: 12px;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: 0.02em;
  line-height: 1;
  text-transform: uppercase;
  user-select: none;
  pointer-events: none;
  transition:
    color var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
}

.app-result-item:hover .app-result-item__monogram {
  color: var(--text-primary);
  transform: scale(1.04);
}

.app-result-item--active .app-result-item__monogram {
  color: var(--accent);
  filter: drop-shadow(0 0 6px var(--accent-glow));
  transform: scale(1.06);
}

.app-result-item__title {
  flex: 1;
  min-width: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.35;
  letter-spacing: -0.005em;
  text-rendering: optimizeLegibility;
  transition: color var(--dur-fast) var(--ease-out);
}

.app-result-item__title--wrap {
  white-space: normal;
  text-overflow: clip;
  overflow: visible;
  line-height: 1.45;
}

.app-title-line {
  display: block;
  line-height: 1.35;
}

.app-result-item__meta {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  margin-left: auto;
  width: 22px;
  height: 22px;
  border-radius: var(--radius-sm);
  opacity: 0;
  transform: translateX(6px);
  transition:
    opacity var(--dur-normal) var(--ease-out),
    transform var(--dur-normal) var(--ease-out);
}

.app-result-item:hover .app-result-item__meta,
.app-result-item--active .app-result-item__meta {
  opacity: 1;
  transform: translateX(0);
}

.app-result-item__enter {
  color: var(--text-muted);
  opacity: 0.85;
  transition:
    color var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out),
    opacity var(--dur-fast) var(--ease-out);
}

.app-result-item--active .app-result-item__enter {
  color: var(--accent);
  opacity: 1;
  transform: scale(1.08);
}
</style>
