<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { ChevronUp, ChevronDown, CornerDownLeft, Keyboard, Loader2, CheckCircle, AlertCircle, Info } from "@lucide/vue"
import type { SearchResult } from '@/types/search'
import { ACTION_BAR_TIMEOUTS, FONT_SIZES, ICON_CONFIG } from '@/config'
import { resultTypeMeta } from '@/utils/resultTypeMeta'
import { useStatusMessages } from '@/composables/useStatusMessages'

const props = defineProps<{
  results: SearchResult[]
  selectedIndex: number
  indexBuilding: boolean
  indexStatus: string
  indexMessage: string
  /** 索引扩展字段,用于 UI 展示"哪一个盘符" + 总卷/当前卷index, 默认值兜底 */
  indexVolumesTotal?: number
  indexVolumeIndex?: number
  indexCurrentVolume?: string
}>()

const emit = defineEmits<{
  (e: 'showHotkeys'): void
}>()

const { currentMessage: statusMessage, hasMessages: hasStatusMessages } = useStatusMessages()

const showIndexStatus = ref(false)
const autoHideTimer = ref<number | null>(null)

// 默认: building 状态保持常驻显示, 让用户始终看到后台索引在做;
//       completed 5s 后隐藏, 错误 8s 后隐藏.
watch(() => props.indexStatus, (newStatus) => {
  if (newStatus === 'building' || newStatus === 'idle') {
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
    }, ACTION_BAR_TIMEOUTS.completedMs)
  } else if (newStatus === 'error') {
    showIndexStatus.value = true
    if (autoHideTimer.value) {
      clearTimeout(autoHideTimer.value)
    }
    autoHideTimer.value = window.setTimeout(() => {
      showIndexStatus.value = false
    }, ACTION_BAR_TIMEOUTS.errorMs)
  }
})

const statusText = computed(() => {
  if (props.results.length === 0) {
    return '未找到结果'
  }

  const selected = props.results[props.selectedIndex]

  if (selected) {
    // 单一真源: 标签从 RESULT_TYPE_META 查, 避免与 ResultItem / ResultItemTypeMeta drift.
    const typeLabel = resultTypeMeta(selected.resultType)?.labelFull ?? selected.resultType
    return `已选择: ${selected.title} · ${typeLabel}`
  }

  return `共 ${props.results.length} 项结果`
})

// 多盘符索引进度文本: "索引中 1/3 · C: (123456)"
const volumeProgress = computed(() => {
  const total = props.indexVolumesTotal ?? 0
  const idx = props.indexVolumeIndex ?? 0
  const cur = props.indexCurrentVolume ?? ''
  if (total > 0) {
    if (cur) return `索引中 ${idx}/${total} · ${cur}`
    return `索引中 ${idx}/${total}`
  }
  return ''
})

// 展示文本: 优先显示状态栏消息管理器中的消息, 其次显示索引状态, 最后显示选中项信息
const displayText = computed(() => {
  // 优先级1: 状态栏消息管理器中的消息（统一入口）
  if (statusMessage.value) {
    return statusMessage.value.text
  }
  // 优先级2: 索引状态
  if (showIndexStatus.value) {
    if (props.indexStatus === 'building' && volumeProgress.value) {
      return volumeProgress.value
    }
    return props.indexMessage
  }
  // 优先级3: 选中项 / 结果统计
  return statusText.value
})

// 当前状态图标: 根据消息管理器中的消息类型决定
const currentStatusType = computed(() => {
  if (statusMessage.value) {
    return statusMessage.value.type
  }
  if (showIndexStatus.value) {
    return props.indexStatus as 'building' | 'completed' | 'error' | 'idle'
  }
  return null
})

// 是否显示状态样式背景
const showStatusActive = computed(() => {
  return !!statusMessage.value || (showIndexStatus.value && props.indexStatus !== 'idle')
})
</script>

<template>
  <div class="action-bar">
    <div class="action-bar__left">
      <span class="action-bar__status" :class="{ 'action-bar__status--active': showStatusActive }">
        <Loader2 v-if="currentStatusType === 'building' || currentStatusType === 'loading'" :size="12" class="action-bar__status-spinner" />
        <CheckCircle v-else-if="currentStatusType === 'completed' || currentStatusType === 'success'" :size="12" class="action-bar__status-icon action-bar__status-icon--success" />
        <AlertCircle v-else-if="currentStatusType === 'error'" :size="12" class="action-bar__status-icon action-bar__status-icon--error" />
        <Info v-else-if="currentStatusType === 'info'" :size="12" class="action-bar__status-icon action-bar__status-icon--info" />
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
      <button
        class="action-bar__hotkey-btn"
        @click="emit('showHotkeys')"
        v-tooltip="{ value: '查看快捷键', showDelay: ICON_CONFIG.tooltipDelayMs, position: 'top' }"
      >
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
  padding: 6px 14px;
  background: transparent;
  border-top: 1px solid var(--border-subtle);
  flex-shrink: 0;
  height: 30px;
}

.action-bar__left {
  display: flex;
  align-items: center;
  gap: var(--sp-5);
  min-width: 0;
  flex: 1;
}

.action-bar__right {
  display: flex;
  align-items: center;
  gap: var(--sp-4);
  flex-shrink: 0;
}

.action-bar__hint {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}

.action-bar__keys {
  display: inline-flex;
  align-items: center;
  gap: 2px;
}

.action-bar__label {
  color: var(--text-quaternary);
  font-size: 10.5px;
  font-weight: 500;
  letter-spacing: 0.02em;
}

.action-bar__status {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--text-tertiary);
  font-size: v-bind('FONT_SIZES.sm + "px"');
  font-weight: 400;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 360px;
  opacity: 1;
  transform: translateY(0);
  transition:
    color var(--dur-normal) var(--ease-out),
    background var(--dur-normal) var(--ease-out),
    opacity var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
}

.action-bar__status--active {
  padding: 2px 8px;
  border-radius: 10px;
  animation: fadeInUp 320ms var(--ease-out);
}

.action-bar__status--active:has(.action-bar__status-spinner) {
  background: var(--accent-soft);
  color: var(--accent);
}

.action-bar__status--active:has(.action-bar__status-icon--success) {
  background: rgba(255, 255, 255, 0.04);
  color: var(--text-secondary);
}

.action-bar__status--active:has(.action-bar__status-icon--error) {
  background: var(--color-danger-bg);
  color: var(--color-danger);
}

.action-bar__status-spinner {
  animation: spin 1s linear infinite;
  color: var(--accent);
}

.action-bar__status-icon--success {
  color: var(--text-primary);
}

.action-bar__status-icon--error {
  color: var(--color-danger);
}

.action-bar__status-icon--info {
  color: var(--text-secondary);
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
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  font-family: var(--font-mono);
  font-size: v-bind('FONT_SIZES.xxs + "px"');
  color: var(--text-tertiary);
  background: var(--inset);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  line-height: 1;
}

.action-bar__hotkey-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  background: transparent;
  border: none;
  border-radius: 6px;
  color: var(--text-tertiary);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
}

.action-bar__hotkey-btn:hover {
  background: var(--list-hover-bg);
  color: var(--text-secondary);
}
</style>
