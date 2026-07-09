<script setup lang="ts">
import { computed } from 'vue'
import type { SearchResult } from '@/types/search'
import {
  FolderOpen, FileText, Terminal, Command, Grid3x3,
  Monitor, User, Package, Image, Video, Music, Archive, Cpu
} from "@lucide/vue"

const props = defineProps<{
  result: SearchResult
  active?: boolean
  index: number
}>()

const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
  (e: 'mouseover', index: number): void
}>()

const IconComponent = computed(() => {
  const typeMap: Record<string, typeof FolderOpen> = {
    'system-app': Monitor,
    'user-app': User,
    'uwp-app': Package,
    'directory': FolderOpen,
    'document': FileText,
    'image': Image,
    'video': Video,
    'audio': Music,
    'executable': Cpu,
    'archive': Archive,
    'other-file': FileText,
    'command': Terminal,
  }

  return typeMap[props.result.resultType] || Command
})

const categoryColor = computed(() => {
  const typeMap: Record<string, string> = {
    'system-app': '#6366f1',
    'user-app': '#10b981',
    'uwp-app': '#f59e0b',
    'directory': '#3b82f6',
    'document': '#8b5cf6',
    'image': '#ec4899',
    'video': '#84cc16',
    'audio': '#f97316',
    'executable': '#ef4444',
    'archive': '#06b6d4',
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
  >
    <div class="result-item__icon" :style="{ '--category-color': categoryColor }">
      <component :is="IconComponent" :size="16" :stroke-width="2" />
    </div>

    <div class="result-item__content">
      <div class="result-item__title">{{ result.title }}</div>
      <div v-if="result.subtitle" class="result-item__subtitle">{{ result.subtitle }}</div>
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
  border-radius: var(--radius-lg);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
  user-select: none;
  background: transparent;
}

.result-item:hover {
  background: var(--surface-hover);
}

.result-item--active {
  background: var(--surface-overlay);
}

.result-item__icon {
  flex-shrink: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  background: var(--surface-raised);
  color: var(--text-tertiary);
  transition: all var(--dur-fast) var(--ease-out);
}

.result-item:hover .result-item__icon {
  color: var(--text-secondary);
}

.result-item--active .result-item__icon {
  background: rgba(139, 92, 246, 0.12);
  color: var(--accent);
}

.result-item__content {
  flex: 1;
  min-width: 0;
}

.result-item__title {
  font-size: var(--text-base);
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: var(--leading-tight);
}

.result-item__subtitle {
  font-size: var(--text-sm);
  color: var(--text-quaternary);
  margin-top: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-item--active .result-item__subtitle {
  color: var(--text-tertiary);
}

.result-item__meta {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  flex-shrink: 0;
}

.result-item__badge {
  padding: 2px 8px;
  font-size: var(--text-xs);
  font-weight: 500;
  color: var(--text-primary);
  background: rgba(139, 92, 246, 0.12);
  border-radius: var(--radius-full);
  color: var(--badge-color, var(--accent));
  background-color: color-mix(in srgb, var(--badge-color, var(--accent)) 12%, transparent);
}

.result-item--active .result-item__badge {
  color: var(--text-secondary);
}

.result-item__shortcut {
  opacity: 0.5;
  transition: opacity var(--dur-fast) var(--ease-out);
}

.result-item:hover .result-item__shortcut,
.result-item--active .result-item__shortcut {
  opacity: 1;
}

.kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 20px;
  height: 20px;
  padding: 0 6px;
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  background: var(--surface-raised);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-xs);
  line-height: 1;
}
</style>
