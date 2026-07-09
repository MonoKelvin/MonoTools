<script setup lang="ts">
import { computed } from 'vue'
import { ChevronUp, ChevronDown, CornerDownLeft, X } from "@lucide/vue"
import type { SearchResult } from '@/types/search'

const props = defineProps<{
  results: SearchResult[]
  selectedIndex: number
}>()

const statusText = computed(() => {
  if (props.results.length === 0) {
    return '未找到结果'
  }
  
  const selected = props.results[props.selectedIndex]
  
  if (selected) {
    const typeLabels: Record<string, string> = {
      'system-app': '系统程序',
      'user-app': '用户程序',
      'uwp-app': 'UWP 应用',
      'directory': '文件夹',
      'document': '文档',
      'image': '图片',
      'video': '视频',
      'audio': '音频',
      'executable': '可执行文件',
      'archive': '压缩文件',
      'other-file': '其他文件',
      'command': '命令',
    }
    
    const typeLabel = typeLabels[selected.resultType] || selected.resultType
    return `已选择: ${selected.title} · ${typeLabel}`
  }
  
  return `共 ${props.results.length} 项结果`
})
</script>

<template>
  <div class="action-bar">
    <div class="action-bar__left">
      <span class="action-bar__status">{{ statusText }}</span>
    </div>

    <div class="action-bar__right">
      <span class="action-bar__hint">
        <span class="action-bar__keys">
          <span class="kbd"><ChevronUp :size="12" :stroke-width="2.5" /></span>
          <span class="kbd"><ChevronDown :size="12" :stroke-width="2.5" /></span>
        </span>
        <span class="action-bar__label">导航</span>
      </span>
      <span class="action-bar__hint">
        <span class="kbd"><CornerDownLeft :size="12" :stroke-width="2.5" /></span>
        <span class="action-bar__label">打开</span>
      </span>
      <span class="action-bar__hint">
        <span class="kbd"><X :size="12" :stroke-width="2.5" /></span>
        <span class="action-bar__label">关闭</span>
      </span>
    </div>
  </div>
</template>

<style scoped>
.action-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-3) var(--sp-5);
  background: var(--surface);
  border-top: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.action-bar__left {
  display: flex;
  align-items: center;
  gap: var(--sp-5);
}

.action-bar__right {
  display: flex;
  align-items: center;
  gap: var(--sp-5);
}

.action-bar__hint {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-3);
}

.action-bar__keys {
  display: inline-flex;
  align-items: center;
  gap: 3px;
}

.action-bar__label {
  color: var(--text-quaternary);
  font-size: var(--text-xs);
  font-weight: 400;
  letter-spacing: 0.02em;
}

.action-bar__status {
  color: var(--text-tertiary);
  font-size: var(--text-xs);
  font-weight: 400;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 300px;
}

.kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  background: var(--surface-raised);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-xs);
  line-height: 1;
}
</style>
