<script setup lang="ts">
import { computed, ref, watch } from 'vue'
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
import type { MtMenuItem } from '@/ui/components/MtMenu.vue'
import MtMenu from '@/ui/components/MtMenu.vue'
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
}>()

const search = useSearchStore()
const { addMessage } = useStatusMessages()

const showMenu = ref(false)
const menuX = ref(0)
const menuY = ref(0)

watch(() => props.visible, (v) => {
  if (v) {
    menuX.value = props.x
    menuY.value = props.y
    showMenu.value = true
  } else {
    showMenu.value = false
  }
})

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

/** 当前项是否已被 pin */
const isPinned = computed(() => {
  if (!props.item) return false
  return search.isPinned(props.item.id)
})

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
  shellApi.openFileLocation(path).catch(() => {
    showToast('打开文件夹失败', 'error')
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
  const dir = itemPath.value
  if (!dir) return
  const lastSep = Math.max(dir.lastIndexOf('\\'), dir.lastIndexOf('/'))
  const dirPath = lastSep > 0 ? dir.substring(0, lastSep) : ''
  navigator.clipboard.writeText(dirPath)
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
  shellApi.showProperties(path).catch(() => {
    showToast('打开属性失败', 'error')
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
  if (!props.item) return
  const path = itemPath.value
  if (!path) {
    showToast('无法删除：当前项路径为空', 'error')
    closeMenu()
    return
  }
  const confirmed = window.confirm(`确定要删除 "${props.item.title}" 吗？\n文件将被移到回收站。`)
  if (!confirmed) return
  shellApi.deleteToRecycleBin(path).then(() => {
    showToast('已删除到回收站')
    search.runSearch().catch(() => undefined)
  }).catch(() => {
    showToast('删除失败', 'error')
  })
  closeMenu()
}

/** 构建菜单项列表 */
const menuItems = computed<MtMenuItem[]>(() => [
  { key: 'open', label: '打开', icon: Play, shortcut: 'Enter' },
  { divider: true },
  { key: 'open-location', label: '打开文件所在路径', icon: FolderOpen, shortcut: 'Ctrl+Enter' },
  { divider: true },
  { key: 'copy-path', label: '复制文件路径', icon: Copy, shortcut: 'Ctrl+C' },
  { key: 'copy-dir', label: '复制目录路径', icon: Copy, shortcut: 'Ctrl+Shift+C' },
  { key: 'copy-name', label: '复制名称', icon: FileText },
  { divider: true },
  { key: 'properties', label: '属性', icon: Info, shortcut: 'Alt+Enter' },
  { divider: true },
  { key: 'pin', label: isPinned.value ? '取消固定' : '固定到首页', icon: isPinned.value ? PinOff : Pin },
  { key: 'delete', label: '删除', icon: Trash2, shortcut: 'Delete', danger: true },
])

function onMenuSelect(item: MtMenuItem) {
  switch (item.key) {
    case 'open': handleOpen(); break
    case 'open-location': handleOpenLocation(); break
    case 'copy-path': handleCopyPath(); break
    case 'copy-dir': handleCopyDirPath(); break
    case 'copy-name': handleCopyName(); break
    case 'properties': handleProperties(); break
    case 'pin': handlePinToggle(); break
    case 'delete': handleDelete(); break
  }
}
</script>

<template>
  <MtMenu
    :items="menuItems"
    :model-value="showMenu"
    :x="menuX"
    :y="menuY"
    @update:model-value="(v) => { if (!v) closeMenu() }"
    @select="onMenuSelect"
  />
</template>
