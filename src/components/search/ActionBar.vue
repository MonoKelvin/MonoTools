<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { ChevronUp, ChevronDown, CornerDownLeft, Keyboard, Loader2, CheckCircle, AlertCircle } from "@lucide/vue"
import type { SearchResult } from '@/types/search'

const props = defineProps<{
  results: SearchResult[]
  selectedIndex: number
  indexBuilding: boolean
  indexStatus: string
  indexMessage: string
}>()

const emit = defineEmits<{
  (e: 'showHotkeys'): void
}>()

const showIndexStatus = ref(false)
const autoHideTimer = ref<number | null>(null)

watch(() => props.indexStatus, (newStatus) => {
  if (newStatus === 'building') {
    showIndexStatus.value = true
    if (autoHideTimer.value) {
      clearTimeout(autoHideTimer.value)
      autoHideTimer.value = null
    }
  } else if (newStatus === 'completed') {
    showIndexStatus.value = true
    if (autoHideTimer.value) {
      clearTimeout(autoHideTimer.value)
    }
    autoHideTimer.value = window.setTimeout(() => {
      showIndexStatus.value = false
    }, 5000)
  } else if (newStatus === 'error') {
    showIndexStatus.value = true
    if (autoHideTimer.value) {
      clearTimeout(autoHideTimer.value)
    }
    autoHideTimer.value = window.setTimeout(() => {
      showIndexStatus.value = false
    }, 8000)
  }
})

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

const displayText = computed(() => {
  if (showIndexStatus.value) {
    return props.indexMessage
  }
  return statusText.value
})
</script>

<template>
  <div class="action-bar">
    <div class="action-bar__left">
      <span class="action-bar__status" :class="{ 'action-bar__status--active': showIndexStatus }">
        <Loader2 v-if="showIndexStatus && indexStatus === 'building'" :size="12" class="action-bar__status-spinner" />
        <CheckCircle v-else-if="showIndexStatus && indexStatus === 'completed'" :size="12" class="action-bar__status-icon action-bar__status-icon--success" />
        <AlertCircle v-else-if="showIndexStatus && indexStatus === 'error'" :size="12" class="action-bar__status-icon action-bar__status-icon--error" />
        <span>{{ displayText }}</span>
      </span>
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
      <button class="action-bar__hotkey-btn" @click="emit('showHotkeys')" title="快捷键">
        <Keyboard :size="14" :stroke-width="2" />
      </button>
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
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--text-tertiary);
  font-size: var(--text-xs);
  font-weight: 400;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 250px;
  opacity: 1;
  transform: translateY(0);
  transition: opacity var(--dur-fast) var(--ease-out), transform var(--dur-fast) var(--ease-out);
}

.action-bar__status--active {
  padding: 2px 8px;
  border-radius: 10px;
  animation: fadeInUp 0.3s ease-out;
}

.action-bar__status--active:has(.action-bar__status-spinner) {
  background: rgba(255, 107, 107, 0.1);
  color: var(--accent);
}

.action-bar__status--active:has(.action-bar__status-icon--success) {
  background: rgba(16, 185, 129, 0.1);
  color: #10b981;
}

.action-bar__status--active:has(.action-bar__status-icon--error) {
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
}

.action-bar__status-spinner {
  animation: spin 1s linear infinite;
}

.action-bar__status-icon--success {
  color: #10b981;
}

.action-bar__status-icon--error {
  color: #ef4444;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

@keyframes fadeInUp {
  from {
    opacity: 0;
    transform: translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
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

.action-bar__hotkey-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-tertiary);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
}

.action-bar__hotkey-btn:hover {
  background: var(--surface-overlay);
  color: var(--text-secondary);
}
</style>
