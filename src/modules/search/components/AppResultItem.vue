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
 * - **自定义 hover tooltip**: 显示应用绝对路径 (PrimeVue v-tooltip 在
 *   我们的 happy-dom / WebView2 / 虚拟列表 组合下不稳定, 改用纯 CSS +
 *   鼠标事件 + setTimeout 的自绘 tooltip, 视觉与项目玻璃风格一致).
 *
 * 与 ResultItem 共享选中态 / 悬停态 / 快捷键 ↵ 行为, 可直接在列表中替换.
 *
 * 图标渲染委托给 `useIconRenderer` composable, 与 ResultItem 共享同一套
 * 4-tier 加载链 + 350ms 兜底 timer + loadToken race 防护. 详见
 * `src/composables/useIconRenderer.ts`.
 */
import { computed, onBeforeUnmount, onMounted, ref, watch, nextTick } from 'vue'
import { AppWindow, Monitor, Package, CornerDownLeft } from '@lucide/vue'
import type { SearchResult } from '@/modules/search'
import { useIconRenderer } from '@/ui/widgets/appicon/useIconRenderer'
import { FONT_SIZES, ICON_CONFIG } from '@/core/config'

const props = defineProps<{
  result: SearchResult
  active?: boolean
  index: number
  /** 徽章尺寸: sm 用于列表/网格, xs 用于图标模式 */
  badgeSize?: 'sm' | 'xs'
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
onMounted(() => refresh(props.result))
watch(() => props.result?.id, () => refresh(props.result))
// 显式调用 dispose (composable 已挂 onBeforeUnmount, 此处为对称性)
onBeforeUnmount(dispose)

/**
 * 自定义 hover tooltip —— 显示应用绝对路径.
 *
 * 为什么不沿用 PrimeVue v-tooltip:
 * - 在 happy-dom (测试) / WebView2 (生产) / 虚拟列表 (v-for 复用) 这三种
 *   环境下, v-tooltip 的 @mouseenter 监听和定位偶尔丢失, 表现"hover
 *   不出来". 改用纯 CSS + 鼠标事件 + setTimeout 后, 不依赖任何外部库.
 * - 自绘的好处: 玻璃风格 (blur / 边框 / 圆角) 与项目 tooltip.scss 完全
 *   一致, 不需要再为 PrimeVue 兜底样式写补丁.
 *
 * 显示规则:
 * - 选中态 (active): 不显示, 避免键盘导航时 tooltip 跟随高亮持续闪现.
 * - 路径为空: 不显示 (没必要给用户一个空 tooltip).
 * - 显示延迟: ICON_CONFIG.appTooltipDelayMs (360ms), 避免鼠标划过闪烁.
 *
 * 路径来源优先级:
 * 1) launch / open 动作: action.data (程序的绝对路径)
 * 2) 否则: subtitle (兜底)
 */
const isHovered = ref(false)
const tooltipVisible = ref(false)
const tooltipStyle = ref<{ left: string; top: string }>({ left: '0px', top: '0px' })
const itemRef = ref<HTMLElement | null>(null)
let showTimer: ReturnType<typeof setTimeout> | null = null

const absolutePath = computed(() => {
  const r = props.result
  if (!r) return ''
  if (r.action?.type === 'launch' || r.action?.type === 'open') {
    return r.action.data ?? ''
  }
  if (r.action?.type === 'run') {
    return r.subtitle || ''
  }
  return r.subtitle || ''
})

function updateTooltipPosition() {
  if (!itemRef.value) return
  const rect = itemRef.value.getBoundingClientRect()
  const vw = window.innerWidth
  const tooltipMaxWidth = 360
  const gap = 6

  let left = rect.left + rect.width / 2
  let top = rect.bottom + gap

  if (left - tooltipMaxWidth / 2 < 8) {
    left = 8 + tooltipMaxWidth / 2
  }
  if (left + tooltipMaxWidth / 2 > vw - 8) {
    left = vw - 8 - tooltipMaxWidth / 2
  }

  tooltipStyle.value = {
    left: `${left}px`,
    top: `${top}px`,
  }
}

function clearShowTimer() {
  if (showTimer) {
    clearTimeout(showTimer)
    showTimer = null
  }
}

function onItemEnter() {
  isHovered.value = true
  if (props.active) return
  if (!absolutePath.value) return
  clearShowTimer()
  showTimer = setTimeout(() => {
    updateTooltipPosition()
    tooltipVisible.value = true
    showTimer = null
  }, ICON_CONFIG.appTooltipDelayMs)
}

function onItemLeave() {
  isHovered.value = false
  tooltipVisible.value = false
  clearShowTimer()
}

// result 变化时强制重置 (例如键盘切换选中项, result 引用换了)
watch(() => props.result?.id, () => {
  tooltipVisible.value = false
  clearShowTimer()
})

// active 变化 (键盘上下方向键) 时, 立即关闭 tooltip, 避免干扰选中态
watch(() => props.active, (isActive) => {
  if (isActive) {
    tooltipVisible.value = false
    clearShowTimer()
  }
})

onBeforeUnmount(() => {
  clearShowTimer()
})

const isSystemApp = computed(() => props.result?.resultType === 'system-app')
const isUwpApp = computed(() => props.result?.resultType === 'uwp-app')

const badgeInfo = computed(() => {
  if (isUwpApp.value) {
    return { icon: Package, label: 'UWP 应用', type: 'uwp' }
  }
  if (isSystemApp.value) {
    return { icon: Monitor, label: '系统应用', type: 'system' }
  }
  return null
})
</script>

<template>
  <div
    ref="itemRef"
    :class="['app-result-item', { 'app-result-item--active': active }]"
    :data-app-result-id="result?.id"
    @mouseenter="onItemEnter"
    @mouseleave="onItemLeave"
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
        class="app-result-item__badge"
        :class="[
          `app-result-item__badge--${badgeInfo.type}`,
          `app-result-item__badge--${badgeSize ?? 'sm'}`
        ]"
        v-tooltip="{ value: badgeInfo.label, showDelay: 400, position: 'top' }"
      >
        <component :is="badgeInfo.icon" :size="badgeSize === 'xs' ? 12 : 10" :stroke-width="2" />
      </div>
    </div>

    <div class="app-result-item__title">{{ result.title }}</div>

    <div class="app-result-item__meta">
      <CornerDownLeft :size="15" :stroke-width="1.8" class="app-result-item__enter" />
    </div>

    <!-- 自定义 hover tooltip: 显示应用绝对路径.
         通过 Teleport 挂载到 body, 用 position: fixed 定位,
         避免被虚拟滚动容器的 overflow: auto 裁剪. -->
    <Teleport to="body">
      <Transition name="app-tooltip-fade">
        <div
          v-if="tooltipVisible && absolutePath"
          class="app-tooltip"
          :style="tooltipStyle"
          role="tooltip"
        >
          {{ absolutePath }}
        </div>
      </Transition>
    </Teleport>
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
  overflow: visible;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  transition: color var(--dur-fast) var(--ease-out);
}

.app-result-item__icon {
  flex-shrink: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-tertiary);
  overflow: visible;
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
    transform var(--dur-fast) var(--ease-out);
}

.app-result-item__badge--sm { width: 16px; height: 16px; border-radius: var(--radius-xs); }
.app-result-item__badge--xs { width: 18px; height: 18px; border-radius: var(--radius-xs); }

.app-result-item:hover .app-result-item__badge,
.app-result-item:hover .app-result-item__badge--uwp {
  color: var(--text-secondary);
  border-color: var(--border-default);
}

.app-result-item:hover .app-result-item__badge--uwp {
  transform: scale(1.06);
}

.app-result-item--active .app-result-item__badge--system {
  color: var(--text-secondary);
  border-color: var(--border-default);
}

.app-result-item--active .app-result-item__badge--uwp {
  color: var(--text-secondary);
  border-color: var(--border-default);
  transform: scale(1.06);
}

/* === Monogram 单字母占位符 (无真实图标时) === */
.app-result-item__monogram {
  width: 60px;
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  font-size: 26px;
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

/* === 自定义 hover tooltip: 显示应用绝对路径 ===
   通过 Teleport 挂载到 body, 使用 position: fixed 定位,
   避免被虚拟滚动容器的 overflow: auto 裁剪. */
.app-tooltip {
  position: fixed;
  left: 0;
  top: 0;
  transform: translateX(-50%);
  z-index: 9999;
  max-width: 360px;
  width: max-content;
  padding: 5px 9px;
  font-size: 11.5px;
  font-weight: 500;
  line-height: 1.4;
  letter-spacing: 0.01em;
  color: var(--text-primary);
  background: var(--glass-bg-soft);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-lg);
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  white-space: normal;
  overflow-wrap: break-word;
  word-break: break-all;
  pointer-events: none;
  user-select: none;
  text-align: center;
  font-family: var(--font-mono);
}

.os-win10 .app-tooltip {
  background: rgba(28, 28, 32, 0.98);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

/* === tooltip 显隐动画 === */
.app-tooltip-fade-enter-active,
.app-tooltip-fade-leave-active {
  transition:
    opacity 160ms var(--ease-out),
    transform 200ms var(--ease-out);
}

.app-tooltip-fade-enter-from,
.app-tooltip-fade-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(-3px);
}
</style>
