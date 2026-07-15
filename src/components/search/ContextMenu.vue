<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue'
import {
  FolderOpen,
  Copy,
  FileText,
  Pin,
  PinOff,
  Play,
  Info,
  Trash2,
} from "@lucide/vue"
import type { SearchResult } from '@/types/search'
import { shellApi } from '@/services/api'
import { useSearchStore } from '@/stores/search'
import { useStatusMessages } from '@/composables/useStatusMessages'

interface Props {
  visible: boolean
  x: number
  y: number
  item: SearchResult | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void
  (e: 'close'): void
  (e: 'pin-toggle', item: SearchResult): void
  (e: 'open', item: SearchResult): void
}>()

const search = useSearchStore()
const { addMessage } = useStatusMessages()

const menuRef = ref<HTMLElement | null>(null)
const adjustedX = ref(0)
const adjustedY = ref(0)
const showMenu = ref(false)

watch(() => props.visible, (v) => {
  if (v) {
    adjustedX.value = props.x + 4
    adjustedY.value = props.y + 6
    showMenu.value = true
    nextTick(() => {
      adjustPosition()
      document.addEventListener('mousedown', onOutsideClick, true)
      document.addEventListener('keydown', onKeydown)
      document.addEventListener('scroll', onScroll, true)
      document.addEventListener('contextmenu', onOutsideClick, true)
    })
  } else {
    showMenu.value = false
    document.removeEventListener('mousedown', onOutsideClick, true)
    document.removeEventListener('keydown', onKeydown)
    document.removeEventListener('scroll', onScroll, true)
    document.removeEventListener('contextmenu', onOutsideClick, true)
  }
})

function adjustPosition() {
  if (!menuRef.value) return
  const rect = menuRef.value.getBoundingClientRect()
  const vw = window.innerWidth
  const vh = window.innerHeight
  const margin = 8

  let left = adjustedX.value
  let top = adjustedY.value

  // 右侧溢出 → 贴右
  if (left + rect.width > vw - margin) {
    left = Math.max(margin, vw - rect.width - margin)
  }
  // 底部溢出 → 贴底
  if (top + rect.height > vh - margin) {
    top = Math.max(margin, vh - rect.height - margin)
  }
  // 左侧溢出 → 贴左
  if (left < margin) left = margin
  // 顶部溢出 → 贴顶
  if (top < margin) top = margin

  adjustedX.value = left
  adjustedY.value = top
}

function onOutsideClick(e: MouseEvent) {
  if (!menuRef.value) return
  if (menuRef.value.contains(e.target as Node)) return
  closeMenu()
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') closeMenu()
}

function onScroll() {
  closeMenu()
}

function closeMenu() {
  emit('update:visible', false)
  emit('close')
}

/** 获取当前项的文件路径 */
const itemPath = computed(() => {
  if (!props.item) return ''
  const action = props.item.action
  if (action?.type === 'open' || action?.type === 'launch') {
    return typeof action.data === 'string' ? action.data : ''
  }
  if (action?.type === 'run') {
    return props.item.subtitle || ''
  }
  return props.item.subtitle || ''
})

/** 获取当前项的目录路径 */
const itemDirPath = computed(() => {
  const path = itemPath.value
  if (!path) return ''
  const lastSep = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'))
  return lastSep > 0 ? path.substring(0, lastSep) : ''
})

/** 当前项是否已被 pin —— 决定菜单显示 "固定" 还是 "取消固定". */
const isPinned = computed(() => {
  if (!props.item) return false
  return search.isPinned(props.item.id)
})

function handleOpen() {
  if (!props.item) return
  search.executeItem(props.item).catch(() => undefined)
  closeMenu()
}

function handleOpenLocation() {
  shellApi.openFileLocation(itemPath.value).catch((e) => {
    showToast('打开文件夹失败', 'error')
    console.error('打开文件夹失败:', e)
  })
  closeMenu()
}

function handleCopyPath() {
  navigator.clipboard.writeText(itemPath.value)
    .then(() => showToast('已复制文件路径'))
    .catch(() => showToast('复制路径失败', 'error'))
  closeMenu()
}

function handleCopyDirPath() {
  navigator.clipboard.writeText(itemDirPath.value)
    .then(() => showToast('已复制目录路径'))
    .catch(() => showToast('复制目录路径失败', 'error'))
  closeMenu()
}

function handleCopyName() {
  if (!props.item) return
  navigator.clipboard.writeText(props.item.title)
    .then(() => showToast('已复制文件名'))
    .catch(() => showToast('复制名称失败', 'error'))
  closeMenu()
}

function handleProperties() {
  shellApi.showProperties(itemPath.value).catch((e) => {
    showToast('打开属性失败', 'error')
    console.error('打开属性失败:', e)
  })
  closeMenu()
}

function handlePinToggle() {
  if (!props.item) return
  emit('pin-toggle', props.item)
  showToast(isPinned.value ? '已取消固定' : '已固定到首页')
  closeMenu()
}

function handleDelete() {
  handleDeleteConfirm()
}

async function handleDeleteConfirm() {
  if (!props.item) return
  const confirmed = window.confirm(`确定要删除 "${props.item.title}" 吗？\n文件将被移到回收站。`)
  if (!confirmed) return
  try {
    await shellApi.deleteToRecycleBin(itemPath.value)
    showToast('已删除到回收站')
    search.runSearch().catch(() => undefined)
  } catch (e) {
    showToast('删除失败', 'error')
    console.error('删除失败:', e)
  }
  closeMenu()
}

/** 显示临时提示消息 */
function showToast(text: string, type: 'info' | 'success' | 'error' = 'success') {
  addMessage({
    type,
    text,
    priority: 5,
    group: 'toast',
    timeout: type === 'error' ? 3000 : 2000,
  })
}
</script>

<template>
  <Teleport to="body">
    <Transition name="ctx-menu">
      <div
        v-if="showMenu"
        ref="menuRef"
        class="ctx-menu"
        :style="{ left: adjustedX + 'px', top: adjustedY + 'px' }"
      >
        <div class="ctx-menu__content">
          <div class="ctx-menu__item" @click="handleOpen">
            <Play :size="14" />
            <span>打开</span>
            <span class="ctx-menu__shortcut">Enter</span>
          </div>
          <div class="ctx-menu__divider" />
          <div class="ctx-menu__item" @click="handleOpenLocation">
            <FolderOpen :size="14" />
            <span>打开文件所在路径</span>
            <span class="ctx-menu__shortcut">Ctrl+Enter</span>
          </div>
          <div class="ctx-menu__divider" />
          <div class="ctx-menu__item" @click="handleCopyPath">
            <Copy :size="14" />
            <span>复制文件路径</span>
            <span class="ctx-menu__shortcut">Ctrl+C</span>
          </div>
          <div class="ctx-menu__item" @click="handleCopyDirPath">
            <Copy :size="14" />
            <span>复制目录路径</span>
            <span class="ctx-menu__shortcut">Ctrl+Shift+C</span>
          </div>
          <div class="ctx-menu__item" @click="handleCopyName">
            <FileText :size="14" />
            <span>复制名称</span>
          </div>
          <div class="ctx-menu__divider" />
          <div class="ctx-menu__item" @click="handleProperties">
            <Info :size="14" />
            <span>属性</span>
            <span class="ctx-menu__shortcut">Alt+Enter</span>
          </div>
          <div class="ctx-menu__divider" />
          <div class="ctx-menu__item" @click="handlePinToggle">
            <component :is="isPinned ? PinOff : Pin" :size="14" />
            <span>{{ isPinned ? '取消固定' : '固定到首页' }}</span>
          </div>
          <div class="ctx-menu__item ctx-menu__item--danger" @click="handleDelete">
            <Trash2 :size="14" />
            <span>删除</span>
            <span class="ctx-menu__shortcut">Delete</span>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.ctx-menu {
  position: fixed;
  z-index: 9999;
  min-width: 240px;
  max-width: 60vw;
  pointer-events: auto;
}

.ctx-menu__content {
  border-radius: var(--radius-lg);
  padding: var(--sp-2);
  background: rgb(17 17 17 / 0.78);
  backdrop-filter: blur(48px) saturate(180%);
  -webkit-backdrop-filter: blur(48px) saturate(180%);
  border: 1px solid var(--glass-border);
  box-shadow:
    0 4px 8px rgba(0, 0, 0, 0.24),
    0 8px 16px rgba(0, 0, 0, 0.20),
    0 16px 32px rgba(0, 0, 0, 0.16),
    0 24px 48px rgba(0, 0, 0, 0.10),
    0 32px 64px rgba(0, 0, 0, 0.05);
  background-image:
    linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.04) 0%,
      rgba(255, 255, 255, 0.0) 40%,
      rgba(0, 0, 0, 0.02) 100%
    );
  user-select: none;
  transform-origin: top center;
}

.ctx-menu__item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 14px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  cursor: pointer;
  background: transparent;
  transition: background 0.15s cubic-bezier(0.4, 0, 0.2, 1);
  min-height: 36px;
}

.ctx-menu__item + .ctx-menu__item {
  margin-top: 1px;
}

.ctx-menu__item:hover {
  background: var(--interactive-hover);
}

.ctx-menu__item--danger {
  color: var(--color-danger);
}

.ctx-menu__item--danger:hover {
  background: var(--color-danger-bg);
}

.ctx-menu__shortcut {
  margin-left: auto;
  font-size: 11px;
  color: var(--text-tertiary);
  font-weight: 400;
}

.ctx-menu__divider {
  height: 1px;
  background: var(--border-subtle);
  margin: 6px 14px;
}

.ctx-menu-enter-active .ctx-menu__content,
.ctx-menu-leave-active .ctx-menu__content {
  transition:
    transform 0.18s cubic-bezier(0.34, 1.12, 0.64, 1),
    opacity 0.18s cubic-bezier(0.34, 1.12, 0.64, 1);
}

.ctx-menu-enter-from .ctx-menu__content,
.ctx-menu-leave-to .ctx-menu__content {
  transform: scale(0.92) translateY(-6px);
  opacity: 0;
}
</style>
