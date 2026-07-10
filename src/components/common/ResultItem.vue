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
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.05);
  color: var(--text-tertiary);
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
}

.result-item__icon::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: 10px;
  background: var(--category-color);
  opacity: 0;
  transition: opacity var(--dur-fast) var(--ease-out);
  filter: blur(20px);
}

.result-item:hover .result-item__icon {
  color: var(--text-secondary);
  background: rgba(255, 255, 255, 0.08);
  transform: scale(1.08);
}

.result-item--active .result-item__icon {
  background: var(--surface-raised);
  color: var(--text-secondary);
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
  font-size: 12px;
  color: rgba(255, 255, 255, 0.4);
  margin-top: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
