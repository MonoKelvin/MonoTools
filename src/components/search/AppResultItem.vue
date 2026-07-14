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
 * - **内置玻璃风格工具提示**: 通过 v-tooltip 显示完整应用名.
 *
 * 与 ResultItem 共享选中态 / 悬停态 / 快捷键 ↵ 行为, 可直接在列表中替换.
 *
 * 图标渲染委托给 `useIconRenderer` composable, 与 ResultItem 共享同一套
 * 4-tier 加载链 + 350ms 兜底 timer + loadToken race 防护. 详见
 * `src/composables/useIconRenderer.ts`.
 */
import { computed } from 'vue'
import { AppWindow, CornerDownLeft } from '@lucide/vue'
import type { SearchResult } from '@/types/search'
import { useIconRenderer } from '@/composables/useIconRenderer'
import { FONT_SIZES, ICON_CONFIG } from '@/config'

const props = defineProps<{
  result: SearchResult
  active?: boolean
  index: number
}>()

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
import { onMounted, onBeforeUnmount, watch } from 'vue'
onMounted(() => refresh(props.result))
watch(() => props.result?.id, () => refresh(props.result))
// 显式调用 dispose (composable 已挂 onBeforeUnmount, 此处为对称性)
onBeforeUnmount(dispose)

/**
 * 工具提示: 显示应用的绝对路径, 便于用户确认程序位置.
 * 选中态(active)下不显示, 避免键盘导航时 tooltip 跟随高亮
 * 持续闪现造成视觉干扰.
 */
const tooltipOptions = computed(() => {
  if (props.active) return undefined
  const path = props.result?.action?.type === 'launch' || props.result?.action?.type === 'open'
    ? props.result.action.data
    : props.result?.subtitle || ''
  if (!path) return undefined
  return {
    value: path,
    class: 'app-tooltip',
    showDelay: ICON_CONFIG.appTooltipDelayMs,
    fitContent: true,
    position: 'bottom' as const,
    autoHide: true,
    escape: true,
  }
})
</script>

<template>
  <div
    :class="['app-result-item', { 'app-result-item--active': active }]"
    v-tooltip="tooltipOptions"
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
    </div>

    <div class="app-result-item__title">{{ result.title }}</div>

    <div class="app-result-item__meta">
      <CornerDownLeft :size="15" :stroke-width="1.8" class="app-result-item__enter" />
    </div>
  </div>
</template>

<style scoped>
.app-result-item {
  display: flex;
  align-items: center;
  gap: var(--sp-4);
  padding: 7px 12px;
  cursor: pointer;
  user-select: none;
  background: transparent;
  position: relative;
  overflow: hidden;
  border: 1px solid transparent;
  border-radius: 9px;
  transition:
    color var(--dur-fast) var(--ease-out);
}

.app-result-item__icon {
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 7px;
  background: transparent;
  color: var(--text-tertiary);
  overflow: hidden;
  position: relative;
  transition:
    color var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
}

.app-result-item:hover .app-result-item__icon {
  color: var(--text-secondary);
  transform: scale(1.04);
}

.app-result-item--active .app-result-item__icon {
  color: var(--accent);
  filter: drop-shadow(0 0 6px var(--accent-glow));
  transform: scale(1.06);
}

.app-result-item__img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  padding: 2px;
  opacity: 0;
  transition: opacity 220ms var(--ease-out);
  pointer-events: none;
}

.app-result-item__img--ready {
  opacity: 1;
}

.app-result-item__lucide {
  pointer-events: none;
}

/* === Monogram 单字母占位符 (无真实图标时) === */
.app-result-item__monogram {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 7px;
  font-size: 14px;
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

.app-result-item__meta {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  margin-left: auto;
  width: 22px;
  height: 22px;
  border-radius: 6px;
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
