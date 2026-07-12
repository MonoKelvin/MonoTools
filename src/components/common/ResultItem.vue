<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch, nextTick } from 'vue'
import type { SearchResult } from '@/types/search'
import { textW, truncateMiddle, truncatePathMiddle } from '@/utils/text'
import {
  FolderOpen, FileText, Terminal, Command, Grid3x3,
  Monitor, User, Package, Image, Video, Music, Archive, Cpu,
  Folder, AppWindow, FileCode, FileImage, FileVideo, FileAudio,
  FileArchive, FileBraces, File
} from "@lucide/vue"

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
  const titleEl = titleRef.value
  const subtitleEl = subtitleRef.value
  if (!titleEl) return

  // Title — measure against the title node itself for tighter fit.
  const titleFont = getComputedStyle(titleEl).font
  const titleMax = titleEl.clientWidth
  if (titleMax > 1) {
    const titleText = props.result.title
    titleEl.textContent = textW(titleText, titleFont) > titleMax - 1
      ? truncateMiddle(titleText, titleMax, titleFont)
      : titleText
  } else {
    titleEl.textContent = props.result.title
  }

  // Subtitle — same idea: use its own font and clientWidth.
  if (subtitleEl && props.result.subtitle) {
    const subFont = getComputedStyle(subtitleEl).font
    const subMax = subtitleEl.clientWidth
    if (subMax > 1) {
      const subText = props.result.subtitle
      subtitleEl.textContent = textW(subText, subFont) > subMax - 1
        ? truncatePathMiddle(subText, subMax, subFont)
        : subText
    } else {
      subtitleEl.textContent = props.result.subtitle
    }
  }
}

watch(() => props.result, () => nextTick(applyTruncation), { flush: 'post' })

onMounted(async () => {
  await document.fonts.ready
  await nextTick()
  const contentEl = contentRef.value
  if (!contentEl) return
  ro = new ResizeObserver(() => requestAnimationFrame(applyTruncation))
  ro.observe(contentEl)
  applyTruncation()
})

onBeforeUnmount(() => {
  if (ro) { ro.disconnect(); ro = null }
})

const IconComponent = computed(() => {
  const typeMap: Record<string, any> = {
    'system-app': Monitor, 'user-app': AppWindow, 'uwp-app': Package,
    'directory': Folder, 'document': FileText, 'image': FileImage,
    'video': FileVideo, 'audio': FileAudio, 'executable': FileBraces,
    'archive': FileArchive, 'other-file': File, 'command': Terminal,
  }
  return typeMap[props.result.resultType] || Command
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
</script>

<template>
  <div
    :class="['result-item', { 'result-item--active': active }]"
    @click="emit('select', result)"
    @mouseenter="emit('mouseover', index)"
    @contextmenu="(e) => emit('contextmenu', e, result)"
  >
    <div class="result-item__icon">
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
      <span v-if="result.meta" class="result-item__meta-text">{{ result.meta }}</span>
      <span v-if="resultTypeLabel" class="result-item__badge">{{ resultTypeLabel }}</span>
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
  padding: 7px 12px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition:
    background var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out);
  user-select: none;
  background: transparent;
  position: relative;
  overflow: hidden;
  border: none;
}

.result-item:hover {
  background: var(--list-hover-bg);
}

.result-item--active {
  background: var(--list-selected-bg);
}

.result-item--active::before {
  content: '';
  position: absolute;
  left: 0;
  top: 8px;
  bottom: 8px;
  width: 2px;
  border-radius: 2px;
  background: var(--accent);
  box-shadow: 0 0 8px var(--accent-glow);
  animation: active-bar-in 280ms var(--ease-spring);
}

@keyframes active-bar-in {
  0% {
    transform: scaleY(0.4);
    opacity: 0;
  }
  100% {
    transform: scaleY(1);
    opacity: 1;
  }
}

.result-item__icon {
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 7px;
  background: transparent;
  color: var(--text-tertiary);
  transition:
    color var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
  position: relative;
}

.result-item:hover .result-item__icon {
  color: var(--text-secondary);
  transform: scale(1.04);
}

.result-item--active .result-item__icon {
  color: var(--accent);
  filter: drop-shadow(0 0 6px var(--accent-glow));
  transform: scale(1.04);
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
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--text-muted);
  background: transparent;
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  line-height: 1;
  transition: color var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out), background var(--dur-fast) var(--ease-out);
}

.result-item--active .kbd {
  background: var(--accent-soft);
  border-color: var(--accent);
  color: var(--accent);
}
</style>
