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
import type { SearchResult } from '@/modules/search'
import { shellApi } from '@/services/api'
import { useSearchStore } from '@/modules/search'
import { useStatusMessages } from '@/modules/search/composables/useStatusMessages'

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
    const raw = typeof action.data === 'string' ? action.data : ''
    if (raw) return raw
  }
  if (action?.type === 'run') {
    const raw = props.item.subtitle || ''
    if (raw) return raw
  }
  return props.item.subtitle || props.item.title || ''
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
  const path = itemPath.value
  if (!path) {
    showToast('无法打开位置：当前项路径为空', 'error')
    closeMenu()
    return
  }
  // 直接传文件完整路径, 后端 explorer /select,<path> 会自动打开父目录并选中文件.
  shellApi.openFileLocation(path).catch((e) => {
    showToast('打开文件夹失败', 'error')
    console.error('[ContextMenu] 打开文件夹失败:', e)
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
  const path = itemPath.value
  if (!path) {
    showToast('无法打开属性：当前项路径为空', 'error')
    closeMenu()
    return
  }
  shellApi.showProperties(path).catch((e) => {
    showToast('打开属性失败', 'error')
    console.error('[ContextMenu] 打开属性失败:', e)
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
  const path = itemPath.value
  if (!path) {
    showToast('无法删除：当前项路径为空', 'error')
    closeMenu()
    return
  }
  const confirmed = window.confirm(`确定要删除 "${props.item.title}" 吗？\n文件将被移到回收站。`)
  if (!confirmed) return
  try {
    await shellApi.deleteToRecycleBin(path)
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
    <Transition name="ctx-menu" :appear="true">
      <div
        v-if="showMenu"
        ref="menuRef"
        class="ctx-menu"
        :style="{ left: adjustedX + 'px', top: adjustedY + 'px' }"
      >
        <div class="ctx-menu__content">
          <button class="ctx-menu__item" @click="handleOpen">
            <Play :size="14" :stroke-width="2" />
            <span>打开</span>
            <span class="ctx-menu__shortcut">Enter</span>
          </button>
          <div class="ctx-menu__divider" />
          <button class="ctx-menu__item" @click="handleOpenLocation">
            <FolderOpen :size="14" :stroke-width="2" />
            <span>打开文件所在路径</span>
            <span class="ctx-menu__shortcut">Ctrl+Enter</span>
          </button>
          <div class="ctx-menu__divider" />
          <button class="ctx-menu__item" @click="handleCopyPath">
            <Copy :size="14" :stroke-width="2" />
            <span>复制文件路径</span>
            <span class="ctx-menu__shortcut">Ctrl+C</span>
          </button>
          <button class="ctx-menu__item" @click="handleCopyDirPath">
            <Copy :size="14" :stroke-width="2" />
            <span>复制目录路径</span>
            <span class="ctx-menu__shortcut">Ctrl+Shift+C</span>
          </button>
          <button class="ctx-menu__item" @click="handleCopyName">
            <FileText :size="14" :stroke-width="2" />
            <span>复制名称</span>
          </button>
          <div class="ctx-menu__divider" />
          <button class="ctx-menu__item" @click="handleProperties">
            <Info :size="14" :stroke-width="2" />
            <span>属性</span>
            <span class="ctx-menu__shortcut">Alt+Enter</span>
          </button>
          <div class="ctx-menu__divider" />
          <button class="ctx-menu__item" @click="handlePinToggle">
            <component :is="isPinned ? PinOff : Pin" :size="14" :stroke-width="2" />
            <span>{{ isPinned ? '取消固定' : '固定到首页' }}</span>
          </button>
          <button class="ctx-menu__item ctx-menu__item--danger" @click="handleDelete">
            <Trash2 :size="14" :stroke-width="2" />
            <span>删除</span>
            <span class="ctx-menu__shortcut">Delete</span>
          </button>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
/* === 浮层容器 ============================================================= */
.ctx-menu {
  position: fixed;
  z-index: 9999;
  min-width: 248px;
  max-width: 60vw;
  pointer-events: auto;
}

/* === 内容卡片 (高级玻璃 + 多层抬升阴影) =================================== */
.ctx-menu__content {
  border-radius: var(--radius-lg);
  padding: var(--sp-2);
  background: var(--glass-bg);
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  border: 1px solid var(--glass-border);
  box-shadow: var(--shadow-xl);
  /* 顶部 1px 高光, 底部 1px 内阴影 — Raycast 浮层做法 */
  background-image:
    linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.05) 0%,
      rgba(255, 255, 255, 0) 30%
    );
  user-select: none;
  transform-origin: var(--ctx-menu-origin-x, top) var(--ctx-menu-origin-y, top);
  /* 让子项能继承正确的 transform 起点 */
  contain: layout paint;
}

.os-win10 .ctx-menu__content {
  background: rgba(28, 28, 32, 0.98);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

/* === 菜单项 =============================================================== */
.ctx-menu__item {
  /* button reset */
  appearance: none;
  -webkit-appearance: none;
  border: none;
  width: 100%;
  text-align: left;
  font: inherit;
  color: inherit;

  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: var(--radius-sm);
  font-size: var(--text-md);
  font-weight: 500;
  color: var(--text-primary);
  cursor: pointer;
  background: transparent;
  min-height: 34px;
  position: relative;
  /* 错落进入: 每项 12ms 间隔, 通过 :nth-child 自动算出来 */
  --enter-delay: calc(var(--idx, 0) * 12ms);
  transform: translateY(-2px);
  opacity: 0;
  animation: ctx-item-in 120ms var(--ease-out) var(--enter-delay) forwards;
  transition:
    background var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
}

.ctx-menu__item > svg {
  flex-shrink: 0;
  color: var(--text-tertiary);
  transition: color var(--dur-fast) var(--ease-out);
}

.ctx-menu__item:hover {
  background: var(--interactive-hover);
}
.ctx-menu__item:hover > svg {
  color: var(--text-secondary);
}

.ctx-menu__item:active {
  transform: scale(0.985);
  background: var(--list-selected-bg);
}

.ctx-menu__item--danger {
  color: var(--color-danger);
}
.ctx-menu__item--danger > svg {
  color: var(--color-danger);
}
.ctx-menu__item--danger:hover {
  background: var(--color-danger-soft);
}
.ctx-menu__item--danger:hover > svg {
  color: var(--color-danger);
}

.ctx-menu__shortcut {
  margin-left: auto;
  font-size: var(--text-xs);
  color: var(--text-quaternary);
  font-weight: 400;
  font-family: var(--font-mono);
  letter-spacing: 0.01em;
}

.ctx-menu__item:hover .ctx-menu__shortcut {
  color: var(--text-tertiary);
}

/* === 分隔线 =============================================================== */
.ctx-menu__divider {
  height: 1px;
  background: var(--border-subtle);
  margin: 4px 8px;
  /* 分隔线淡入, 比菜单项稍晚 */
  opacity: 0;
  animation: ctx-divider-in 200ms var(--ease-out) 60ms forwards;
}

/* === 进出动画 (容器) ====================================================== */
.ctx-menu-enter-active .ctx-menu__content,
.ctx-menu-leave-active .ctx-menu__content {
  transition:
    transform 100ms var(--ease-out),
    opacity 100ms var(--ease-out);
}
.ctx-menu-enter-from .ctx-menu__content,
.ctx-menu-leave-to .ctx-menu__content {
  transform: scale(0.96) translateY(-2px);
  opacity: 0;
}

@keyframes ctx-item-in {
  to {
    transform: translateY(0);
    opacity: 1;
  }
}

@keyframes ctx-divider-in {
  to { opacity: 1; }
}

/* === 通过 :nth-child 给菜单项注入 --idx, 避免在模板里手写 style ======== */
.ctx-menu__item:nth-child(1)  { --idx: 0; }
.ctx-menu__item:nth-child(2)  { --idx: 1; }
.ctx-menu__item:nth-child(3)  { --idx: 2; }
.ctx-menu__item:nth-child(4)  { --idx: 3; }
.ctx-menu__item:nth-child(5)  { --idx: 4; }
.ctx-menu__item:nth-child(6)  { --idx: 5; }
.ctx-menu__item:nth-child(7)  { --idx: 6; }
.ctx-menu__item:nth-child(8)  { --idx: 7; }
.ctx-menu__item:nth-child(9)  { --idx: 8; }
.ctx-menu__item:nth-child(10) { --idx: 9; }
.ctx-menu__item:nth-child(11) { --idx: 10; }
.ctx-menu__item:nth-child(12) { --idx: 11; }

/* === 无障碍 ============================================================== */
@media (prefers-reduced-motion: reduce) {
  .ctx-menu__item,
  .ctx-menu__divider {
    animation: none;
    transform: none;
    opacity: 1;
  }
  .ctx-menu-enter-active .ctx-menu__content,
  .ctx-menu-leave-active .ctx-menu__content {
    transition: opacity 80ms linear;
  }
}
</style>
