<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, computed } from 'vue'
import { FolderOpen, Copy, FileText, ExternalLink, Pin, PinOff } from "@lucide/vue"
import type { SearchResult } from '@/types/search'
import { shellApi } from '@/services/api'
import { useSearchStore } from '@/stores/search'

interface Props {
  visible: boolean
  x: number
  y: number
  item: SearchResult | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'pin-toggle', item: SearchResult): void
}>()

const search = useSearchStore()

/** 当前项是否已被 pin —— 决定菜单显示 "固定" 还是 "取消固定". */
const isPinned = computed(() => {
  if (!props.item) return false
  return search.isPinned(props.item.id)
})

const copyPath = async () => {
  if (!props.item) return
  try {
    await navigator.clipboard.writeText(props.item.subtitle || props.item.title)
    emit('close')
  } catch (e) {
    console.error('复制路径失败:', e)
  }
}

const copyName = async () => {
  if (!props.item) return
  try {
    await navigator.clipboard.writeText(props.item.title)
    emit('close')
  } catch (e) {
    console.error('复制名称失败:', e)
  }
}

const openContainingFolder = async () => {
  if (!props.item) return
  try {
    const path = props.item.subtitle || props.item.title
    const dir = path.substring(0, path.lastIndexOf('\\'))
    await shellApi.open(dir)
    emit('close')
  } catch (e) {
    console.error('打开文件夹失败:', e)
  }
}

const onPinToggle = () => {
  if (!props.item) return
  emit('pin-toggle', props.item)
  emit('close')
}

const handleClickOutside = (e: MouseEvent) => {
  const target = e.target as HTMLElement
  if (!target.closest('.context-menu')) {
    emit('close')
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
  document.addEventListener('contextmenu', handleClickOutside)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', handleClickOutside)
  document.removeEventListener('contextmenu', handleClickOutside)
})
</script>

<template>
  <Teleport to="body">
    <Transition name="context-menu">
      <div
        v-if="visible && item"
        class="context-menu"
        :style="{ left: `${x}px`, top: `${y}px` }"
      >
        <div class="context-menu__item" @click="openContainingFolder">
          <FolderOpen :size="14" />
          <span>打开文件所在路径</span>
        </div>
        <div class="context-menu__item" @click="copyPath">
          <Copy :size="14" />
          <span>复制完整路径名称</span>
        </div>
        <div class="context-menu__item" @click="copyName">
          <FileText :size="14" />
          <span>复制名称</span>
        </div>
        <div class="context-menu__divider"></div>
        <div class="context-menu__item" @click="onPinToggle">
          <component :is="isPinned ? PinOff : Pin" :size="14" />
          <span>{{ isPinned ? '取消固定' : '固定到首页' }}</span>
        </div>
        <div class="context-menu__item" @click="() => emit('close')">
          <ExternalLink :size="14" />
          <span>关闭</span>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.context-menu {
  position: fixed;
  min-width: 200px;
  background: var(--surface-overlay);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-2xl);
  padding: 4px;
  z-index: 10000;
  backdrop-filter: blur(20px);
}

.os-win10 .context-menu {
  background: rgba(28, 28, 32, 0.98);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
  border-color: var(--border-default);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
}

.context-menu__item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  color: var(--text-secondary);
  font-size: 13px;
  cursor: pointer;
  border-radius: 6px;
  transition: all var(--dur-fast) var(--ease-out);
}

.context-menu__item:hover {
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-primary);
}

.context-menu__divider {
  height: 1px;
  background: var(--border-subtle);
  margin: 4px 8px;
}

.context-menu-enter-active,
.context-menu-leave-active {
  transition: all 0.15s cubic-bezier(0.4, 0, 0.2, 1);
}

.context-menu-enter-from,
.context-menu-leave-to {
  opacity: 0;
  transform: scale(0.95) translateY(4px);
}
</style>
