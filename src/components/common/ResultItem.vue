<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch, nextTick } from 'vue'
import type { SearchResult } from '@/types/search'

// --- Canvas-based pixel width measurement (cached) ---
const _canvas = document.createElement('canvas')
const _ctx = _canvas.getContext('2d')!
const _cwCache = new Map<string, number>()

function getCW(ch: string, font: string): number {
  const key = ch + '\x00' + font
  const hit = _cwCache.get(key)
  if (hit !== undefined) return hit
  const w = _ctx.measureText(ch).width
  _cwCache.set(key, w)
  return w
}

function textW(text: string, font: string): number {
  let w = 0
  for (let i = 0; i < text.length; i++) w += getCW(text[i], font)
  return w
}

const ELLIPSIS = '...'
const ELLIPSIS_W = (() => {
  const w = textW(ELLIPSIS, '14px sans-serif')
  _cwCache.set('...\x0014px sans-serif', w)
  return w
})()

// Pre-compute prefix/suffix width arrays for O(1) lookup
function buildWidthArrays(text: string, font: string): { pref: number[]; suff: number[] } {
  const pref = new Array(text.length + 1)
  const suff = new Array(text.length + 1)
  pref[0] = 0
  for (let i = 0; i < text.length; i++) pref[i + 1] = pref[i] + getCW(text[i], font)
  suff[text.length] = 0
  for (let i = text.length - 1; i >= 0; i--) suff[i] = suff[i + 1] + getCW(text[i], font)
  return { pref, suff }
}

// Binary search: find max left chars such that pref[left] + ellipsisW + suff[text.length - left] <= maxW
function maxLeftFit(maxW: number, textLen: number, pref: number[], suff: number[]): number {
  if (maxW <= 0) return 0
  let lo = 0, hi = Math.floor(textLen / 2)
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1
    if (pref[mid] + ELLIPSIS_W + suff[textLen - mid] <= maxW) lo = mid
    else hi = mid - 1
  }
  return lo
}

// --- Middle ellipsis truncation using pixel width ---
function truncateMiddle(text: string, maxWidth: number, font: string): string {
  if (maxWidth <= 0) return ELLIPSIS
  if (textW(text, font) <= maxWidth) return text
  if (maxWidth <= ELLIPSIS_W) return ELLIPSIS

  const { pref, suff } = buildWidthArrays(text, font)
  const n = text.length
  const left = maxLeftFit(maxWidth, n, pref, suff)
  if (left <= 0) return ELLIPSIS

  return text.substring(0, left) + ELLIPSIS + text.substring(n - left)
}

// --- Path truncation: preserve drive letter + filename, truncate middle ---
function truncatePathMiddle(path: string, maxWidth: number, font: string): string {
  if (maxWidth <= 0) return ELLIPSIS
  if (textW(path, font) <= maxWidth) return path
  if (maxWidth <= ELLIPSIS_W) return ELLIPSIS

  const parts = path.split('\\')
  if (parts.length <= 2) return truncateMiddle(path, maxWidth, font)

  const drive = parts[0] + '\\'
  const filename = parts[parts.length - 1]
  const driveW = textW(drive, font)
  const fileW = textW(filename, font)

  // If even drive + ellipsis + filename doesn't fit, fall back to full middle truncation
  if (driveW + ELLIPSIS_W + fileW >= maxWidth) {
    const fileAvail = maxWidth - driveW - ELLIPSIS_W
    if (fileAvail <= ELLIPSIS_W) return truncateMiddle(path, maxWidth, font)
    return drive + ELLIPSIS + truncateMiddle(filename, fileAvail, font)
  }

  // Try to include middle directories
  const middleParts = parts.slice(1, parts.length - 1)
  const middleStr = middleParts.join('\\')
  const middleAvail = maxWidth - driveW - ELLIPSIS_W - fileW

  if (textW(middleStr, font) <= middleAvail) {
    return drive + middleStr + '\\' + filename
  }

  // Truncate the middle part
  return drive + truncateMiddle(middleStr, middleAvail, font) + '\\' + filename
}

// --- Component ---
const props = defineProps<{
  result: SearchResult
  active?: boolean
  index: number
}>()

const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
  (e: 'mouseover', index: number): void
  (e: 'contextmenu', event: MouseEvent, item: SearchResult): void
}>()

const contentRef = ref<HTMLElement | null>(null)
const titleRef = ref<HTMLElement | null>(null)
const subtitleRef = ref<HTMLElement | null>(null)
let ro: ResizeObserver | null = null

function applyTruncation() {
  const contentEl = contentRef.value
  const titleEl = titleRef.value
  const subtitleEl = subtitleRef.value
  if (!contentEl || !titleEl) return

  const maxW = contentEl.clientWidth
  if (maxW <= 0) return

  // Use content element's computed font for accurate measurement
  const font = getComputedStyle(contentEl).font

  // Title
  const titleText = props.result.title
  if (textW(titleText, font) > maxW) {
    titleEl.textContent = truncateMiddle(titleText, maxW, font)
  } else {
    titleEl.textContent = titleText
  }

  // Subtitle
  if (subtitleEl && props.result.subtitle) {
    const subText = props.result.subtitle
    if (textW(subText, font) > maxW) {
      subtitleEl.textContent = truncatePathMiddle(subText, maxW, font)
    } else {
      subtitleEl.textContent = subText
    }
  }
}

watch(() => props.result, () => nextTick(applyTruncation))

onMounted(async () => {
  await document.fonts.ready
  await nextTick()
  if (!contentRef.value) return
  ro = new ResizeObserver(() => requestAnimationFrame(applyTruncation))
  ro.observe(contentRef.value)
  applyTruncation()
})

onBeforeUnmount(() => {
  if (ro) { ro.disconnect(); ro = null }
})

const IconComponent = computed(() => {
  const typeMap: Record<string, typeof FolderOpen> = {
    'system-app': Monitor, 'user-app': AppWindow, 'uwp-app': Package,
    'directory': Folder, 'document': FileCode, 'image': FileImage,
    'video': FileVideo, 'audio': FileAudio, 'executable': FileBraces,
    'archive': FileArchive, 'other-file': File, 'command': Terminal,
  }
  return typeMap[props.result.resultType] || Command
})

const categoryColor = computed(() => {
  const typeMap: Record<string, string> = {
    'system-app': '#6366f1', 'user-app': '#3b82f6', 'uwp-app': '#8b5cf6',
    'directory': '#f59e0b', 'document': '#06b6d4', 'image': '#ec4899',
    'video': '#84cc16', 'audio': '#f97316', 'executable': '#ef4444',
    'archive': '#10b981', 'other-file': '#6b7280', 'command': '#14b8a6',
  }
  return typeMap[props.result.resultType] || 'var(--accent)'
})

const resultTypeLabel = computed(() => {
  const labels: Record<string, string> = {
    'system-app': '系统', 'user-app': '用户', 'uwp-app': 'UWP',
    'directory': '文件夹', 'document': '文档', 'image': '图片',
    'video': '视频', 'audio': '音频', 'executable': '可执行',
    'archive': '压缩', 'other-file': '其他', 'command': '命令',
  }
  return labels[props.result.resultType] || ''
})

import {
  FolderOpen, FileText, Terminal, Command, Grid3x3,
  Monitor, User, Package, Image, Video, Music, Archive, Cpu,
  Folder, AppWindow, FileCode, FileImage, FileVideo, FileAudio,
  FileArchive, FileBraces, File
} from "@lucide/vue"
</script>

<template>
  <div
    :class="['result-item', { 'result-item--active': active }]"
    @click="emit('select', result)"
    @mouseenter="emit('mouseover', index)"
    @contextmenu="(e) => emit('contextmenu', e, result)"
  >
    <div class="result-item__icon" :style="{ '--category-color': categoryColor }">
      <component :is="IconComponent" :size="16" :stroke-width="2" />
    </div>

    <div class="result-item__content" ref="contentRef">
      <div class="result-item__title" ref="titleRef" :title="result.title"></div>
      <div
        v-if="result.subtitle"
        class="result-item__subtitle"
        ref="subtitleRef"
        :title="result.subtitle"
      ></div>
    </div>

    <div class="result-item__meta">
      <span v-if="resultTypeLabel" class="result-item__badge" :style="{ '--badge-color': categoryColor }">{{ resultTypeLabel }}</span>
      <span class="result-item__shortcut">
        <span class="kbd">↵</span>
      </span>
    </div>
  </div>
</template>

<style scoped>
.result-item {
  display: flex;
  align-items: center;
  gap: var(--sp-4);
  padding: var(--sp-3) var(--sp-4);
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  user-select: none;
  background: transparent;
  position: relative;
  overflow: hidden;
}

.result-item::before {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(135deg, rgba(255, 255, 255, 0.05) 0%, transparent 50%);
  opacity: 0;
  transition: opacity var(--dur-fast) var(--ease-out);
}

.result-item:hover::before,
.result-item--active::before {
  opacity: 1;
}

.result-item:hover {
  background: rgba(255, 255, 255, 0.06);
  transform: translateX(4px);
}

.result-item--active {
  background: var(--surface-overlay);
}

.result-item__icon {
  flex-shrink: 0;
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 12px;
  background: color-mix(in srgb, var(--category-color) 12%, transparent);
  color: var(--category-color);
  transition: opacity var(--dur-fast) var(--ease-out);
  position: relative;
  border: 1px solid color-mix(in srgb, var(--category-color) 20%, transparent);
}

.result-item:hover .result-item__icon {
  background: color-mix(in srgb, var(--category-color) 16%, transparent);
}

.result-item--active .result-item__icon {
  background: color-mix(in srgb, var(--category-color) 20%, transparent);
}

.result-item__content {
  flex: 1;
  min-width: 0;
}

.result-item__title {
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  white-space: nowrap;
  line-height: var(--leading-tight);
}

.result-item__subtitle {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.4);
  margin-top: 2px;
  font-family: var(--font-mono);
  overflow: hidden;
  white-space: nowrap;
}

.result-item--active .result-item__subtitle {
  color: rgba(255, 255, 255, 0.6);
}

.result-item__meta {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  flex-shrink: 0;
}

.result-item__badge {
  padding: 3px 10px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.3px;
  color: var(--badge-color, var(--accent));
  background: color-mix(in srgb, var(--badge-color, var(--accent)) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--badge-color, var(--accent)) 20%, transparent);
  border-radius: 12px;
  transition: all var(--dur-fast) var(--ease-out);
}

.result-item--active .result-item__badge {
  background: color-mix(in srgb, var(--badge-color, var(--accent)) 15%, transparent);
  border-color: color-mix(in srgb, var(--badge-color, var(--accent)) 30%, transparent);
}

.result-item__shortcut {
  opacity: 0;
  transform: translateX(8px);
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.result-item:hover .result-item__shortcut,
.result-item--active .result-item__shortcut {
  opacity: 1;
  transform: translateX(0);
}

.kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 22px;
  height: 22px;
  padding: 0 7px;
  font-family: var(--font-mono);
  font-size: 11px;
  color: rgba(255, 255, 255, 0.7);
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 6px;
  line-height: 1;
  transition: all var(--dur-fast) var(--ease-out);
}

.result-item--active .kbd {
  background: rgba(255, 107, 107, 0.2);
  border-color: rgba(255, 107, 107, 0.3);
  color: var(--accent);
}
</style>
