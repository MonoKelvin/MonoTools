<script setup lang="ts">
/**
 * ResultItem —�?通用搜索结果�?(file / directory / document / image / ...).
 *
 * 用�? "所有文�? �? 以及 resultType 不是 user-app/system-app/uwp-app
 * �?fallback 渲染场景. 历史�?.url / .lnk / 含图标的文件走这�?
 * 因此需要接�?useAppIcon 加载真实 PNG, 详见
 * `tests/ui/components/ResultItem.test.ts`.
 *
 * �?AppResultItem 的差�?
 * - 副标题显示完整路�?
 * - 图标�?(16-18px), 文字略小
 * - 标题截断由外�?contentRef/contentWidth 控制
 *
 * 图标渲染委托�?`useIconRenderer` composable, �?AppResultItem 共享
 * 同一�?4-tier 加载�?+ 350ms 兜底 timer + loadToken race 防护.
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
import { useIconRenderer } from '@/ui/widgets/appicon/useIconRenderer'
import { useAdaptiveText } from '@/utils/adaptiveText'
import { resultTypeMeta } from '../utils/resultTypeMeta'
import { ICON_CONFIG } from '@/core/config'

// --- Component ---
// 事件全部由父容器 (VirtualGroupedResults 的行 div) 统一处理, 这里只做展示.
// 仅声�?emit 留作扩展�? 右键菜单等场景需要时再使�?
const props = defineProps<{
  result: SearchResult
  active?: boolean
  index: number
  /** 标题是否允许换行: 默认 false (单行省略号), true 时可换行显示完整内容 */
  titleWrap?: boolean
  /** 是否禁止字体缩小: 默认 false (可缩小), true 时始终使用最大字号 */
  noFontShrink?: boolean
  /** 是否禁用 tooltip: 由父容器统一处理时设为 true, 避免重复 */
  noTooltip?: boolean
}>()

const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
  (e: 'open', item: SearchResult): void
  (e: 'contextmenu', event: MouseEvent, item: SearchResult): void
}>()
// 抑制 "声明但未使用" 警告 —�?保留 emit 以便父级透传扩展事件.
void emit

/**
 * 自适应标题文字: 优先缩小字体, 超出则省略号.
 * titleWrap=true 时允许多行显示.
 * noFontShrink=true 时始终使用最大字号, 不缩小字体.
 */
const {
  containerRef: titleContainerRef,
  displayText: adaptiveTitle,
  displayLines: adaptiveTitleLines,
  currentFontSize: _titleFontSize,
  isTruncated: titleIsTruncated,
  whiteSpaceMode: titleWhiteSpaceMode,
  update: updateTitleAdaptive,
} = useAdaptiveText(() => props.result?.title || '', {
  maxFontSize: 13.5,
  minFontSize: 9,
  fontWeight: '500',
  padding: 4,
  whiteSpace: 'nowrap',
  maxLines: undefined,
})

const titleFontSize = computed(() => props.noFontShrink ? 13.5 : _titleFontSize.value)

watch(
  () => props.titleWrap,
  (wrap) => {
    titleWhiteSpaceMode.value = wrap ? 'normal' : 'nowrap'
    nextTick(() => updateTitleAdaptive())
  },
  { immediate: true }
)

/**
 * 自适应副标题 (路径) 文字.
 * titleWrap=true 时允许多行显示.
 * noFontShrink=true 时始终使用最大字号, 不缩小字体.
 */
const {
  containerRef: subtitleContainerRef,
  displayText: adaptiveSubtitle,
  displayLines: adaptiveSubtitleLines,
  currentFontSize: _subtitleFontSize,
  isTruncated: subtitleIsTruncated,
  whiteSpaceMode: subtitleWhiteSpaceMode,
  update: updateSubtitleAdaptive,
} = useAdaptiveText(() => props.result?.subtitle || '', {
  maxFontSize: 11.5,
  minFontSize: 9,
  fontWeight: '400',
  fontFamily: 'var(--font-mono)',
  padding: 4,
  whiteSpace: 'nowrap',
  maxLines: undefined,
})

const subtitleFontSize = computed(() => props.noFontShrink ? 11.5 : _subtitleFontSize.value)

watch(
  () => props.titleWrap,
  (wrap) => {
    subtitleWhiteSpaceMode.value = wrap ? 'normal' : 'nowrap'
    nextTick(() => updateSubtitleAdaptive())
  },
  { immediate: false }
)

const contentRef = ref<HTMLElement | null>(null)
const titleRef = ref<HTMLElement | null>(null)
const subtitleRef = ref<HTMLElement | null>(null)
let ro: ResizeObserver | null = null

/**
 * 截断状�? 当文本被 truncateMiddle / truncatePathMiddle 缩短时为 true.
 * 用于决定是否�?title / subtitle 启用 tooltip (未截断时不需�?.
 * ResizeObserver 触发 applyTruncation, 截断状态实时更�?
 */
const titleTruncated = ref(false)
const subtitleTruncated = ref(false)

function applyTruncation() {
  const titleEl = titleRef.value
  const subtitleEl = subtitleRef.value
  if (!titleEl) return

  // Title �?measure against the title node itself for tighter fit.
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

  // Subtitle �?same idea: use its own font and clientWidth.
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
 * 标题 tooltip —�?仅在文本被截断时显示完整原文.
 * �?PrimeVue v-tooltip (项目统一玻璃风格), 替代浏览器默�?title 提示.
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
 * 路径 tooltip —�?当路径被截断时显示完整路�? 使用 mono 字体 tooltip 变体.
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
 * 通用兜底 Lucide 图标: �?resultTypeMeta �?icon, 没查到用 Command.
 * 集中定义�?src/utils/resultTypeMeta.ts, 新增 type 单点改动.
 */
const IconComponent = computed(() => {
  return resultTypeMeta(props.result?.resultType)?.icon ?? Command
})

/**
 * 角标短标�?(ResultItem �?label, ActionBar �?labelFull).
 */
const resultTypeLabel = computed(() => {
  return resultTypeMeta(props.result?.resultType)?.label ?? ''
})

/**
 * 图标渲染 composable —�?�?AppResultItem 共享同一套图标状态机.
 * 关键修复: 之前 .url / .lnk �?所有文�?组项�? 后端 IPC 提取�?PNG
 * 写进 useAppIcon.cache 但没人读, 显示 Lucide 兜底. 现在 ResultItem
 * 也接�?useAppIcon, 真实 PNG 会被消费.
 */
const { iconState, imgReady, refresh, onImgLoad, onImgError, dispose } = useIconRenderer({
  fallbackComponent: IconComponent.value,
  containerSelector: (id) => `[data-result-id="${id}"] img`,
  debugTag: 'ResultItem',
})

onMounted(async () => {
  // happy-dom 不实�?FontFaceSet, 防御性地 await
  try {
    if (typeof document !== 'undefined' && (document as any).fonts?.ready) {
      await (document as any).fonts.ready
    }
  } catch {
    // 忽略 fonts.ready 异常 (happy-dom / jsdom 不支�?
  }
  await nextTick()
  const contentEl = contentRef.value
  if (!contentEl) return
  ro = new ResizeObserver(() => requestAnimationFrame(applyTruncation))
  ro.observe(contentEl)
  applyTruncation()
  // 挂载时调 refresh, 触发真实 PNG 图标加载 (�?AppResultItem 一�?
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
          :size="18"
          :stroke-width="2"
        />
    </div>

    <div class="result-item__content" ref="contentRef">
      <div
        class="result-item__title"
        :class="{ 'result-item__title--wrap': titleWrap }"
        ref="titleContainerRef"
        :style="{ fontSize: titleFontSize + 'px' }"
        :title="!noTooltip && titleIsTruncated && !noFontShrink ? (result.title || '') : ''"
      >
        <template v-if="noFontShrink">
          {{ result.title || '' }}
        </template>
        <template v-else-if="titleWrap && adaptiveTitleLines.length > 1">
          <span v-for="(line, idx) in adaptiveTitleLines" :key="idx" class="result-title-line">{{ line }}</span>
        </template>
        <template v-else>
          {{ adaptiveTitle }}
        </template>
      </div>
      <div
        v-if="result.subtitle"
        class="result-item__subtitle"
        :class="{ 'result-item__subtitle--wrap': titleWrap }"
        ref="subtitleContainerRef"
        :style="{ fontSize: subtitleFontSize + 'px' }"
        :title="!noTooltip && subtitleIsTruncated && !noFontShrink ? (result.subtitle || '') : ''"
      >
        <template v-if="noFontShrink">
          {{ result.subtitle || '' }}
        </template>
        <template v-else-if="titleWrap && adaptiveSubtitleLines.length > 1">
          <span v-for="(line, idx) in adaptiveSubtitleLines" :key="idx" class="result-subtitle-line">{{ line }}</span>
        </template>
        <template v-else>
          {{ adaptiveSubtitle }}
        </template>
      </div>
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
  padding: 5px 14px;
  cursor: pointer;
  user-select: none;
  background: transparent;
  position: relative;
  overflow: hidden;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  transition:
    background var(--dur-fast) var(--ease-out),
    border-color var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
}

.result-item:hover {
  background: var(--list-hover-bg);
}

.result-item--active {
  background: var(--list-selected-bg);
}

.result-item__icon {
  flex-shrink: 0;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-tertiary);
  transition:
    color var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
  position: relative;
}

/* PNG 真实图标 (ResultItem 也读 useAppIcon cache, �?所有文�?组的
 * .url / .lnk / 含图标的文件能显示真�?PNG). �?AppResultItem 一致的渐入逻辑. */
.result-item__img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  opacity: 0;
  transition: opacity var(--dur-slow) var(--ease-out);
  pointer-events: none;
  user-select: none;
}
.result-item__img--ready { opacity: 1; }

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

.result-item__title--wrap {
  white-space: normal;
  overflow: visible;
  line-height: 1.45;
}

.result-title-line {
  display: block;
  line-height: 1.35;
}

.result-item__subtitle {
  font-size: 11.5px;
  color: var(--text-tertiary);
  margin-top: 2px;
  font-family: var(--font-mono);
  overflow: hidden;
  white-space: nowrap;
  text-rendering: optimizeLegibility;
  transition: color var(--dur-fast) var(--ease-out);
}

.result-item__subtitle--wrap {
  white-space: normal;
  overflow: visible;
  line-height: 1.45;
  word-break: break-all;
}

.result-item--active .result-item__subtitle {
  color: var(--text-secondary);
}

.result-item__meta {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  flex-shrink: 0;
  margin-left: auto;
}

/* 文件大小等次级元信息: 灰色, �?type badge 更轻的视觉权�?*/
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
  transition:
    color var(--dur-fast) var(--ease-out),
    border-color var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out);
}

.result-item--active .result-item__badge {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--accent-soft);
}

.result-item__shortcut {
  opacity: 0;
  transform: translateX(6px);
  transition:
    opacity var(--dur-normal) var(--ease-out),
    transform var(--dur-normal) var(--ease-out);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: var(--radius-sm);
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
