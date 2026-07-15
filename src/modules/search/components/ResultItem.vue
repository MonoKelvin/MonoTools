<script setup lang="ts">
/**
 * ResultItem —— 通用搜索结果项 (file / directory / document / image / ...).
 *
 * 用途: "所有文件" 组, 以及 resultType 不是 user-app/system-app/uwp-app
 * 的 fallback 渲染场景. 历史上 .url / .lnk / 含图标的文件走这里,
 * 因此需要接入 useAppIcon 加载真实 PNG, 详见
 * `tests/ui/components/ResultItem.test.ts`.
 *
 * 与 AppResultItem 的差异:
 * - 副标题显示完整路径
 * - 图标小 (16-18px), 文字略小
 * - 标题截断由外部 contentRef/contentWidth 控制
 *
 * 图标渲染委托给 `useIconRenderer` composable, 与 AppResultItem 共享
 * 同一套 4-tier 加载链 + 350ms 兜底 timer + loadToken race 防护.
 */
import { computed, onBeforeUnmount, onMounted, ref, watch, nextTick } from 'vue'
import type { SearchResult } from '@/modules/search'
import { textW, truncateMiddle, truncatePathMiddle } from '@/utils/text'
import {
  FolderOpen, FileText, Terminal, Command, Grid3x3,
  Monitor, User, Package, Image, Video, Music, Archive, Cpu,
  Folder, AppWindow, FileCode, FileImage, FileVideo, FileAudio,
  FileArchive, File, CornerDownLeft
} from "@lucide/vue"
import { useIconRenderer } from '@/common/composables/useIconRenderer'
import { resultTypeMeta } from '@/utils/resultTypeMeta'
import { ICON_CONFIG } from '@/core/config'

// --- Component ---
// 事件全部由父容器 (VirtualGroupedResults 的行 div) 统一处理, 这里只做展示.
// 仅声明 emit 留作扩展点: 右键菜单等场景需要时再使用.
const props = defineProps<{
  result: SearchResult
  active?: boolean
  index: number
}>()

const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
  (e: 'open', item: SearchResult): void
  (e: 'contextmenu', event: MouseEvent, item: SearchResult): void
}>()
// 抑制 "声明但未使用" 警告 —— 保留 emit 以便父级透传扩展事件.
void emit

const contentRef = ref<HTMLElement | null>(null)
const titleRef = ref<HTMLElement | null>(null)
const subtitleRef = ref<HTMLElement | null>(null)
let ro: ResizeObserver | null = null

/**
 * 截断状态: 当文本被 truncateMiddle / truncatePathMiddle 缩短时为 true.
 * 用于决定是否对 title / subtitle 启用 tooltip (未截断时不需要).
 * ResizeObserver 触发 applyTruncation, 截断状态实时更新.
 */
const titleTruncated = ref(false)
const subtitleTruncated = ref(false)

function applyTruncation() {
  const titleEl = titleRef.value
  const subtitleEl = subtitleRef.value
  if (!titleEl) return

  // Title — measure against the title node itself for tighter fit.
  const titleFont = getComputedStyle(titleEl).font
  const titleMax = titleEl.clientWidth
  if (titleMax > 1) {
    const titleText = props.result.title
    const needsTruncation = textW(titleText, titleFont) > titleMax - 1
    titleEl.textContent = needsTruncation
      ? truncateMiddle(titleText, titleMax, titleFont)
      : titleText
    titleTruncated.value = needsTruncation
  } else {
    titleEl.textContent = props.result.title
    titleTruncated.value = false
  }

  // Subtitle — same idea: use its own font and clientWidth.
  if (subtitleEl && props.result.subtitle) {
    const subFont = getComputedStyle(subtitleEl).font
    const subMax = subtitleEl.clientWidth
    if (subMax > 1) {
      const subText = props.result.subtitle
      const needsTruncation = textW(subText, subFont) > subMax - 1
      subtitleEl.textContent = needsTruncation
        ? truncatePathMiddle(subText, subMax, subFont)
        : subText
      subtitleTruncated.value = needsTruncation
    } else {
      subtitleEl.textContent = props.result.subtitle
      subtitleTruncated.value = false
    }
  } else {
    subtitleTruncated.value = false
  }
}

/**
 * 标题 tooltip —— 仅在文本被截断时显示完整原文.
 * 用 PrimeVue v-tooltip (项目统一玻璃风格), 替代浏览器默认 title 提示.
 */
const titleTooltip = computed(() => {
  if (!titleTruncated.value) return undefined
  return {
    value: props.result.title,
    class: 'mono-tooltip',
    showDelay: ICON_CONFIG.tooltipDelayMs,
    position: 'top' as const,
    autoHide: true,
    escape: true,
  }
})

/**
 * 路径 tooltip —— 当路径被截断时显示完整路径, 使用 mono 字体 tooltip 变体.
 */
const subtitleTooltip = computed(() => {
  if (!subtitleTruncated.value || !props.result.subtitle) return undefined
  return {
    value: props.result.subtitle,
    class: 'mono-tooltip',
    showDelay: ICON_CONFIG.tooltipDelayMs,
    position: 'top' as const,
    autoHide: true,
    escape: true,
  }
})

watch(() => props.result, () => nextTick(applyTruncation), { flush: 'post' })

/**
 * 通用兜底 Lucide 图标: 从 resultTypeMeta 查 icon, 没查到用 Command.
 * 集中定义在 src/utils/resultTypeMeta.ts, 新增 type 单点改动.
 */
const IconComponent = computed(() => {
  return resultTypeMeta(props.result?.resultType)?.icon ?? Command
})

/**
 * 角标短标签 (ResultItem 用 label, ActionBar 用 labelFull).
 */
const resultTypeLabel = computed(() => {
  return resultTypeMeta(props.result?.resultType)?.label ?? ''
})

/**
 * 图标渲染 composable —— 与 AppResultItem 共享同一套图标状态机.
 * 关键修复: 之前 .url / .lnk 等"所有文件"组项目, 后端 IPC 提取的 PNG
 * 写进 useAppIcon.cache 但没人读, 显示 Lucide 兜底. 现在 ResultItem
 * 也接入 useAppIcon, 真实 PNG 会被消费.
 */
const { iconState, imgReady, refresh, onImgLoad, onImgError, dispose } = useIconRenderer({
  fallbackComponent: IconComponent.value,
  containerSelector: (id) => `[data-result-id="${id}"] img`,
  debugTag: 'ResultItem',
})

onMounted(async () => {
  // happy-dom 不实现 FontFaceSet, 防御性地 await
  try {
    if (typeof document !== 'undefined' && (document as any).fonts?.ready) {
      await (document as any).fonts.ready
    }
  } catch {
    // 忽略 fonts.ready 异常 (happy-dom / jsdom 不支持)
  }
  await nextTick()
  const contentEl = contentRef.value
  if (!contentEl) return
  ro = new ResizeObserver(() => requestAnimationFrame(applyTruncation))
  ro.observe(contentEl)
  applyTruncation()
  // 挂载时调 refresh, 触发真实 PNG 图标加载 (与 AppResultItem 一致)
  refresh(props.result)
})

watch(() => props.result?.id, () => refresh(props.result))
onBeforeUnmount(() => {
  dispose()
  if (ro) { ro.disconnect(); ro = null }
})
</script>

<template>
  <div
    :class="['result-item', { 'result-item--active': active }]"
    :data-result-id="result?.id"
  >
    <div class="result-item__icon">
      <img
        v-if="iconState.kind === 'svg' || iconState.kind === 'png'"
        :src="iconState.value"
        class="result-item__img"
        :class="{ 'result-item__img--ready': imgReady }"
        @load="onImgLoad"
        @error="onImgError"
        decoding="async"
        draggable="false"
        alt=""
      />
      <component
        v-else
        :is="iconState.value || IconComponent"
        :size="16"
        :stroke-width="2"
      />
    </div>

    <div class="result-item__content" ref="contentRef">
      <div
        class="result-item__title"
        ref="titleRef"
        v-tooltip="titleTooltip"
      ></div>
      <div
        v-if="result.subtitle"
        class="result-item__subtitle"
        ref="subtitleRef"
        v-tooltip="subtitleTooltip"
      ></div>
    </div>

    <div class="result-item__meta">
      <span v-if="result.meta" class="result-item__meta-text">{{ result.meta }}</span>
      <span v-if="resultTypeLabel" class="result-item__badge">{{ resultTypeLabel }}</span>
      <span class="result-item__shortcut">
        <CornerDownLeft :size="15" :stroke-width="1.8" class="result-item__enter" />
      </span>
    </div>
  </div>
</template>

<style scoped>
.result-item {
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

.result-item__icon {
  flex-shrink: 0;
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  background: transparent;
  color: var(--text-tertiary);
  transition:
    color var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
  position: relative;
}

/**
 * PNG 真实图标 (ResultItem 也读 useAppIcon cache, 让"所有文件"组的
 * .url / .lnk / 含图标的文件能显示真实 PNG).
 * - 与 Lucide 通用图标 16x16 视觉对齐, 居中放置
 * - opacity 0 → 1 渐入与 AppResultItem 一致
 * - 350ms 兜底 timer 在 happy-dom / WebView2 不会丢失显示
 */
.result-item__img {
  width: 18px;
  height: 18px;
  object-fit: contain;
  opacity: 0;
  transition: opacity 220ms var(--ease-out);
  pointer-events: none;
  user-select: none;
}
.result-item__img--ready {
  opacity: 1;
}

.result-item:hover .result-item__icon {
  color: var(--text-secondary);
  background: rgba(255, 255, 255, 0.04);
  transform: scale(1.04);
}

.result-item--active .result-item__icon {
  color: var(--accent);
  background: var(--accent-soft);
  filter: drop-shadow(0 0 6px var(--accent-glow));
  transform: scale(1.06);
}

.result-item__content {
  flex: 1;
  min-width: 0;
}

.result-item__title {
  font-size: 13.5px;
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  white-space: nowrap;
  line-height: 1.35;
  text-rendering: optimizeLegibility;
  letter-spacing: -0.005em;
  transition: color var(--dur-fast) var(--ease-out);
}

.result-item__subtitle {
  font-size: 11.5px;
  color: var(--text-tertiary);
  margin-top: 2px;
  font-family: var(--font-mono);
  overflow: hidden;
  white-space: nowrap;
  text-rendering: optimizeLegibility;
  letter-spacing: 0;
  transition: color var(--dur-fast) var(--ease-out);
}

.result-item--active .result-item__subtitle {
  color: var(--text-secondary);
}

.result-item__meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  margin-left: auto;
}

/* 文件大小等次级元信息: 灰色, 比 type badge 更轻的视觉权重 */
.result-item__meta-text {
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--text-quaternary);
  letter-spacing: 0.01em;
  font-variant-numeric: tabular-nums;
  transition: color var(--dur-fast) var(--ease-out);
}

.result-item--active .result-item__meta-text {
  color: var(--text-tertiary);
}

.result-item__badge {
  padding: 1px 7px;
  font-size: 10px;
  font-weight: 500;
  letter-spacing: 0.02em;
  color: var(--text-quaternary);
  background: transparent;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-full);
  transition: color var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out), background var(--dur-fast) var(--ease-out);
}

.result-item--active .result-item__badge {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--accent-soft);
}

.result-item__shortcut {
  opacity: 0;
  transform: translateX(6px);
  transition: opacity var(--dur-normal) var(--ease-out), transform var(--dur-normal) var(--ease-out);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 6px;
}

.result-item:hover .result-item__shortcut,
.result-item--active .result-item__shortcut {
  opacity: 1;
  transform: translateX(0);
}

.result-item__enter {
  color: var(--text-muted);
  opacity: 0.85;
  transition:
    color var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out),
    opacity var(--dur-fast) var(--ease-out);
}

.result-item--active .result-item__enter {
  color: var(--accent);
  opacity: 1;
  transform: scale(1.08);
}
</style>
