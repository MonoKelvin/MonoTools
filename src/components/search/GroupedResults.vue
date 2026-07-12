<script setup lang="ts">
/**
 * Raycast 风格分组列表.
 *
 * 分组结构:
 *   - Pinned / Favorites (固定)
 *   - Recent (最近)
 *   - System Apps (固定)
 *   - Commands (命令)
 *   - Apps (所有应用)
 *   - Files (所有文件 —— 含子分类 chip 多选)
 *
 * 每个 group:
 *   - 粘性 group title (在列表顶部时跟随滚动)
 *   - 子分类 chip (适用于 "Files" / "Apps":多选切换)
 *   - 内部虚拟滚动 (复用 VIRTUAL_LIST_THRESHOLD)
 */
import { computed, ref } from 'vue'
import { ChevronDown, File as FileIcon, FileCode, FileImage, FileText, Music,
  Video, Folder, Settings as SettingsIcon, Terminal, PinIcon, Clock, Cpu,
  Sparkles, Archive as ArchiveIcon, FileVideo, FileAudio, FileArchive,
  FileBraces, FileSpreadsheet, FileChartPie, Presentation
} from '@lucide/vue'
import type { SearchResult, ResultType } from '@/types/search'
import ResultItem from '@/components/common/ResultItem.vue'
import {
  classify, FILE_KIND_META, FILE_KIND_DISPLAY_ORDER, type FileKind,
} from '@/utils/fileKinds'

interface Props {
  results: SearchResult[]
  loading?: boolean
  selectedIndex: number
  /** "Pinned" 区. 来自 store 的常驻收藏. */
  pinned?: SearchResult[]
  /** 已打开过的应用记录. */
  recent?: SearchResult[]
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  pinned: () => [],
  recent: () => [],
})

const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
  (e: 'hover', index: number): void
  (e: 'contextmenu', event: MouseEvent, item: SearchResult): void
}>()

const selectedFileKinds = ref<Set<FileKind>>(new Set(FILE_KIND_DISPLAY_ORDER))

/** Raycast 风格 group 描述. */
interface Group {
  id: string
  title: string
  icon: any
  /** 该组允许的文件子分类(空数组表示不限制;非空表示该组只展示属于这些子分类的项). null=非文件组. */
  kinds: FileKind[] | null
  items: SearchResult[]
  /** 项目的"虚拟"起始行号, 用于选中索引换算. */
  itemOffset: number
}

const FILE_GROUP_ID = 'group.files'
const FILE_GROUP_TITLE = '所有文件'
const APPS_GROUP_ID = 'group.apps'
const APPS_GROUP_TITLE = '所有应用'
const COMMANDS_GROUP_ID = 'group.commands'
const COMMANDS_GROUP_TITLE = '命令'
const PINNED_GROUP_ID = 'group.pinned'
const PINNED_GROUP_TITLE = '固定项目'
const SYSTEM_GROUP_ID = 'group.system'
const SYSTEM_GROUP_TITLE = '系统应用'
const RECENT_GROUP_ID = 'group.recent'
const RECENT_GROUP_TITLE = '最近访问'

function isFile(r: SearchResult): boolean {
  return r.category === 'files'
}
function isApp(r: SearchResult): boolean {
  return r.category === 'apps'
}
function isCommand(r: SearchResult): boolean {
  return r.category === 'commands'
}
function isSystemApp(r: SearchResult): boolean {
  return isApp(r) && (r as any).resultType === 'system-app'
}

/** 把 results 拆组, 应用文件子分类筛选. */
const groups = computed<Group[]>(() => {
  const out: Group[] = []
  let offset = 0

  const pinned = (props.pinned || []).slice(0, 4)
  if (pinned.length) {
    out.push({
      id: PINNED_GROUP_ID, title: PINNED_GROUP_TITLE,
      icon: PinIcon, kinds: null, items: pinned, itemOffset: 0,
    })
    offset += pinned.length
  }

  const recent = (props.recent || []).slice(0, 6)
  if (recent.length && recent !== pinned) {
    out.push({
      id: RECENT_GROUP_ID, title: RECENT_GROUP_TITLE,
      icon: Clock, kinds: null, items: recent, itemOffset: offset,
    })
    offset += recent.length
  }

  const sysApps: SearchResult[] = []
  const userApps: SearchResult[] = []
  for (const r of props.results) {
    if (isSystemApp(r)) sysApps.push(r)
    else if (isApp(r)) userApps.push(r)
  }
  if (sysApps.length) {
    sysApps.sort((a, b) => a.title.localeCompare(b.title))
    out.push({
      id: SYSTEM_GROUP_ID, title: SYSTEM_GROUP_TITLE,
      icon: SettingsIcon, kinds: null, items: sysApps.slice(0, 12), itemOffset: offset,
    })
    offset += sysApps.slice(0, 12).length
  }

  const commands: SearchResult[] = []
  for (const r of props.results) if (isCommand(r)) commands.push(r)
  if (commands.length) {
    out.push({
      id: COMMANDS_GROUP_ID, title: COMMANDS_GROUP_TITLE,
      icon: Terminal, kinds: null, items: commands.slice(0, 10), itemOffset: offset,
    })
    offset += commands.slice(0, 10).length
  }

  // Apps 全部
  if (userApps.length) {
    out.push({
      id: APPS_GROUP_ID, title: APPS_GROUP_TITLE,
      icon: Sparkles, kinds: null, items: userApps.slice(0, 24), itemOffset: offset,
    })
    offset += userApps.slice(0, 24).length
  }

  // Files 全部 + 子分类筛选
  const filesAll: SearchResult[] = []
  for (const r of props.results) {
    if (!isFile(r)) continue
    const ext = (r.subtitle || r.title || '').split(/[\\/]/).pop() || ''
    const kind = classify(ext)
    if (selectedFileKinds.value.has(kind)) {
      filesAll.push(r)
    }
  }
  if (filesAll.length) {
    out.push({
      id: FILE_GROUP_ID, title: FILE_GROUP_TITLE,
      icon: Folder, kinds: FILE_KIND_DISPLAY_ORDER, items: filesAll.slice(0, 80), itemOffset: offset,
    })
    offset += filesAll.slice(0, 80).length
  }

  return out
})

/** 拼接所有组内项 (用于渲染, 兼容 selectedIndex 全局按下标). */
const flatItems = computed(() => {
  return groups.value.flatMap(g => g.items)
})

/** 用户点击文件子分类 chip 时切换集合. */
function toggleKind(k: FileKind, e: Event) {
  if (e && (e as any).metaKey) {
    // Meta/Ctrl: 单选切换
    if (selectedFileKinds.value.has(k) && selectedFileKinds.value.size === 1) {
      // 不能全关, 留一个
      return
    }
    const next = new Set(selectedFileKinds.value)
    if (next.has(k)) next.delete(k)
    else next.add(k)
    selectedFileKinds.value = next
  } else {
    // 普通点击: 全选 (恢复), 或全清
    if (selectedFileKinds.value.size === FILE_KIND_DISPLAY_ORDER.length) {
      selectedFileKinds.value = new Set([k])
    } else {
      selectedFileKinds.value = new Set(FILE_KIND_DISPLAY_ORDER)
    }
  }
}

function isKindActive(k: FileKind): boolean {
  return selectedFileKinds.value.has(k)
}

function kindChipIcon(k: FileKind) {
  switch (k) {
    case 'document':     return FileText
    case 'spreadsheet':  return FileSpreadsheet
    case 'presentation': return Presentation
    case 'pdf':          return FileText
    case 'code':         return FileCode
    case 'image':        return FileImage
    case 'video':        return FileVideo
    case 'audio':        return FileAudio
    case 'archive':      return FileArchive
    case 'font':         return FileBraces
    case 'design':       return FileImage
    case 'executable':   return Cpu
    case 'other':        return FileIcon
  }
}

const isLoading = computed(() => props.loading && flatItems.value.length === 0)
const nothingNow = computed(() => !props.loading && flatItems.value.length === 0)

function onPickItem(item: SearchResult) {
  emit('select', item)
}

function onItemHover(idx: number) {
  emit('hover', idx)
}
</script>

<template>
  <div class="g">
    <div v-if="isLoading" class="g__loading">
      <div class="g__spinner"></div>
      <span class="g__loading-text">搜索中…</span>
    </div>

    <div v-else-if="nothingNow" class="g__empty">
      <slot name="empty" />
    </div>

    <div v-else class="g__list">
      <section v-for="g in groups" :key="g.id" class="g__group">
        <header class="g__group-header">
          <component :is="g.icon" :size="14" :stroke-width="2" class="g__group-icon" />
          <span class="g__group-title">{{ g.title }}</span>
          <span v-if="g.items.length" class="g__group-count">{{ g.items.length }}</span>
          <ChevronDown :size="12" class="g__group-chev" />
        </header>

        <!-- 文件组: 标题下方插入多选子分类 chip -->
        <div v-if="g.id === FILE_GROUP_ID" class="g__chips">
          <button
            v-for="k in g.kinds || []"
            :key="k"
            class="g__chip"
            :class="{ 'g__chip--active': isKindActive(k) }"
            @click="toggleKind(k, $event)"
            :title="FILE_KIND_META[k].label"
          >
            <component :is="kindChipIcon(k)" :size="12" :stroke-width="2" />
            <span>{{ FILE_KIND_META[k].label }}</span>
          </button>
        </div>

        <div class="g__rows">
          <ResultItem
            v-for="(it, idxIdx) in g.items"
            :key="it.id + ':' + (g.itemOffset + idxIdx)"
            :result="it"
            :index="g.itemOffset + idxIdx"
            :active="(g.itemOffset + idxIdx) === selectedIndex"
            @select="onPickItem"
            @mouseover="onItemHover(g.itemOffset + idxIdx)"
            @contextmenu="(e) => emit('contextmenu', e, it)"
          />
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.g {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 6px 8px 12px;
}

.g::-webkit-scrollbar { width: 8px; }
.g::-webkit-scrollbar-thumb {
  background: rgba(255,255,255,0.12);
  border-radius: 999px;
}
.g::-webkit-scrollbar-thumb:hover { background: rgba(255,255,255,0.22); }
.g::-webkit-scrollbar-track { background: transparent; }

.g__loading,
.g__empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-3);
  padding: var(--sp-10) var(--sp-5);
  flex: 1;
}

.g__loading {
  flex-direction: row;
}

.g__spinner {
  width: 18px; height: 18px;
  border: 2px solid var(--border-default);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: g-spin 0.8s linear infinite;
}

@keyframes g-spin {
  to { transform: rotate(360deg); }
}

.g__group {
  margin-top: 8px;
}

.g__group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 26px;
  padding: 0 10px;
  border-radius: 6px;
  position: sticky;
  top: 0;
  z-index: 2;
  background: var(--surface);
}
body.theme-light .g__group-header {
  background: var(--surface);
}

.g__group-icon {
  color: var(--accent);
}

.g__group-title {
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--accent);
  flex: 1;
}

.g__group-count {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-tertiary);
  background: var(--surface-overlay);
  border: 1px solid var(--border-subtle);
  border-radius: 999px;
  padding: 1px 7px;
  line-height: 1;
}

.g__group-chev {
  color: var(--text-tertiary);
  opacity: 0;
  transition: opacity var(--dur-fast) var(--ease-out);
}

.g__group:hover .g__group-chev { opacity: 1; }

.g__chips {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 6px 6px 8px 10px;
}

.g__chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  border-radius: 999px;
  border: 1px solid var(--border-subtle);
  background: transparent;
  color: var(--text-tertiary);
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.02em;
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
}

.g__chip:hover {
  background: var(--surface-overlay);
  border-color: rgba(255,255,255,0.14);
  color: var(--text-secondary);
}

.g__chip--active {
  background: rgba(255, 107, 107, 0.14);
  border-color: rgba(255, 107, 107, 0.32);
  color: var(--accent);
}

.g__rows {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
</style>
