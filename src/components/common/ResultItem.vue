<script setup lang="ts">
import { computed } from 'vue'
import type { SearchResult } from '@/types/search'

const truncatePath = (path: string, maxLength: number = 80): string => {
  if (path.length <= maxLength) {
    return path
  }

  const parts = path.split('\\')
  if (parts.length <= 2) {
    return path.substring(0, maxLength - 3) + '...'
  }

  const startPart = parts[0] + '\\'
  const endPart = '\\' + parts[parts.length - 1]
  const middleLength = maxLength - startPart.length - endPart.length - 3

  if (middleLength <= 0) {
    return path.substring(0, maxLength - 3) + '...'
  }

  const middleParts = parts.slice(1, parts.length - 1)
  let middle = middleParts.join('\\')

  if (middle.length > middleLength) {
    const half = Math.floor(middleLength / 2)
    middle = middle.substring(0, half) + '...' + middle.substring(middle.length - half)
  }

  return startPart + middle + endPart
}

const truncateText = (text: string, maxLength: number = 40): string => {
  if (text.length <= maxLength) {
    return text
  }

  const half = Math.floor((maxLength - 3) / 2)
  return text.substring(0, half) + '...' + text.substring(text.length - half)
}
import {
  FolderOpen, FileText, Terminal, Command, Grid3x3,
  Monitor, User, Package, Image, Video, Music, Archive, Cpu,
  Folder, AppWindow, FileCode, FileImage, FileVideo, FileAudio,
  FileArchive, FileBraces, File
} from "@lucide/vue"

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

const IconComponent = computed(() => {
  const typeMap: Record<string, typeof FolderOpen> = {
    'system-app': Monitor,
    'user-app': AppWindow,
    'uwp-app': Package,
    'directory': Folder,
    'document': FileCode,
    'image': FileImage,
    'video': FileVideo,
    'audio': FileAudio,
    'executable': FileBraces,
    'archive': FileArchive,
    'other-file': File,
    'command': Terminal,
  }

  return typeMap[props.result.resultType] || Command
})

const categoryColor = computed(() => {
  const typeMap: Record<string, string> = {
    'system-app': '#6366f1',
    'user-app': '#3b82f6',
    'uwp-app': '#8b5cf6',
    'directory': '#f59e0b',
    'document': '#06b6d4',
    'image': '#ec4899',
    'video': '#84cc16',
    'audio': '#f97316',
    'executable': '#ef4444',
    'archive': '#10b981',
    'other-file': '#6b7280',
    'command': '#14b8a6',
  }

  return typeMap[props.result.resultType] || 'var(--accent)'
})

const resultTypeLabel = computed(() => {
  const labels: Record<string, string> = {
    'system-app': '系统',
    'user-app': '用户',
    'uwp-app': 'UWP',
    'directory': '文件夹',
    'document': '文档',
    'image': '图片',
    'video': '视频',
    'audio': '音频',
    'executable': '可执行',
    'archive': '压缩',
    'other-file': '其他',
    'command': '命令',
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
    <div class="result-item__icon" :style="{ '--category-color': categoryColor }">
      <component :is="IconComponent" :size="16" :stroke-width="2" />
    </div>

    <div class="result-item__content">
      <div class="result-item__title" :title="result.title">{{ truncateText(result.title, 50) }}</div>
      <div
        v-if="result.subtitle"
        class="result-item__subtitle"
        :title="result.subtitle"
      >
        {{ truncatePath(result.subtitle, 90) }}
      </div>
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
