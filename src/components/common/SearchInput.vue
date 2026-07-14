<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Search, X, Settings, Terminal, LogOut, Sun } from "@lucide/vue"
import { invoke } from '@tauri-apps/api/core'
import { isTauri } from '@/services/env'
import MtMenu from './MtMenu.vue'
import type { MtMenuItem } from './MtMenu.vue'

const props = withDefaults(
  defineProps<{
    modelValue: string
    placeholder?: string
  }>(),
  { placeholder: '搜索应用、文件、命令...' },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
  (e: 'enter'): void
  (e: 'arrowUp'): void
  (e: 'arrowDown'): void
  (e: 'escape'): void
  (e: 'contextmenu', event: MouseEvent | CustomEvent): void
}>()

const inputRef = ref<HTMLInputElement | null>(null)
const focused = ref(false)

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter') { e.preventDefault(); emit('enter') }
  else if (e.key === 'ArrowDown') { e.preventDefault(); emit('arrowDown') }
  else if (e.key === 'ArrowUp') { e.preventDefault(); emit('arrowUp') }
  else if (e.key === 'Escape') { e.preventDefault(); emit('escape') }
}

onMounted(() => {
  inputRef.value?.focus()
})

/* ============================================================================
 * 拖拽判定: 鼠标在搜索框上, 仅在文字范围外的空白处才启动窗口拖拽.
 *
 * 历史实现: 每次 mousemove 都 createElement('canvas') + measureText('m') 计算
 * 字符宽度, 在 500+ 文件的列表上叠加 60Hz 鼠标事件, 触发明显的 GC 抖动.
 * 现在的方案:
 *   - 复用同一只 canvas (单例, 模块级), 不再每帧 new.
 *   - 用 ResizeObserver + watch 缓存 rect/font, 仅在窗口/字体变更时重算.
 *   - 用 rAF 合并 mousemove, 避免单帧多次 measureText.
 * ========================================================================== */

let measureCanvas: HTMLCanvasElement | null = null
let measureCtx: CanvasRenderingContext2D | null = null
let cachedCharWidth = 0
let cachedRect: DOMRect | null = null
let cachedFontKey = ''

/** 单例 canvas, 仅在首次需要时 lazy 创建. */
function getMeasureCtx(): CanvasRenderingContext2D | null {
  if (measureCtx) return measureCtx
  if (typeof document === 'undefined') return null
  if (!measureCanvas) measureCanvas = document.createElement('canvas')
  if (!measureCtx) measureCtx = measureCanvas.getContext('2d')
  return measureCtx
}

/** 重建缓存: 字体/输入框位置变化时调用. */
function refreshMeasureCache(input: HTMLInputElement) {
  const ctx = getMeasureCtx()
  if (!ctx) return
  const style = getComputedStyle(input)
  const fontKey = `${style.fontWeight}|${style.fontSize}|${style.fontFamily}`
  if (fontKey !== cachedFontKey) {
    ctx.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`
    cachedCharWidth = ctx.measureText('m').width
    cachedFontKey = fontKey
  }
  cachedRect = input.getBoundingClientRect()
}

function isOverInteractiveArea(input: HTMLInputElement, clientX: number): boolean {
  if (!cachedRect) refreshMeasureCache(input)
  if (!cachedRect) return false

  const text = input.value || props.placeholder || ''

  // 可交互区域 = 文字长度 + 右侧预留 5 个字符宽度
  // 预留区域用于点击后继续输入、光标定位等文本编辑操作.
  const charW = cachedCharWidth || 8
  const textWidth = text.length * charW
  const reserveWidth = 5 * charW

  return clientX >= cachedRect.left && clientX <= cachedRect.left + textWidth + reserveWidth
}

let rafScheduled = false
let lastMoveX = 0
let lastMoveWrapper: HTMLElement | null = null

function handleSearchBarMousemove(event: MouseEvent) {
  lastMoveX = event.clientX
  lastMoveWrapper = event.currentTarget as HTMLElement
  if (rafScheduled) return
  rafScheduled = true
  requestAnimationFrame(() => {
    rafScheduled = false
    if (!lastMoveWrapper) return
    const input = inputRef.value
    if (!input) return
    if (!cachedRect) refreshMeasureCache(input)
    const isInteractive = isOverInteractiveArea(input, lastMoveX)
    const cursor = isInteractive ? 'text' : 'default'
    lastMoveWrapper.style.cursor = cursor
    input.style.cursor = cursor
  })
}

function handleSearchBarMouseleave() {
  const input = inputRef.value
  if (input) input.style.cursor = ''
  if (lastMoveWrapper) lastMoveWrapper.style.cursor = ''
  lastMoveWrapper = null
  cachedRect = null
}

async function handleSearchBarMousedown(event: MouseEvent) {
  const target = event.target as HTMLElement

  if (target.closest('.search-bar__logo')) return
  if (target.closest('.search-bar__clear')) return

  if (showLogoMenu.value) showLogoMenu.value = false

  const input = inputRef.value
  if (input && target.closest('.search-bar__input-wrapper')) {
    refreshMeasureCache(input)
    const isInteractive = isOverInteractiveArea(input, event.clientX)
    if (isInteractive) return
  }

  event.preventDefault()

  if (isTauri) {
    try {
      await invoke('set_dragging', { dragging: true })
      await invoke('start_dragging')
      setTimeout(async () => {
        try {
          await invoke('set_dragging', { dragging: false })
        } catch {}
      }, 500)
    } catch {}
  }
}

type MenuKey = 'settings' | 'commands' | 'theme' | 'quit'

interface MenuItem {
  key: MenuKey
  label: string
  icon: any
  danger?: boolean
}

const menuItems: MenuItem[] = [
  { key: 'settings', label: '设置', icon: Settings },
  { key: 'commands', label: '命令管理', icon: Terminal },
  { key: 'theme', label: '切换主题', icon: Sun },
  { key: 'quit', label: '退出', icon: LogOut, danger: true },
]

const showLogoMenu = ref(false)
const logoMenuPos = ref({ x: 0, y: 0 })

function onLogoClick(event: MouseEvent) {
  event.preventDefault()
  event.stopPropagation()
  const offsetX = 6
  const offsetY = 6
  logoMenuPos.value = { x: event.clientX + offsetX, y: event.clientY + offsetY }
  showLogoMenu.value = true
}

function onLogoContextMenu(event: MouseEvent) {
  event.preventDefault()
  event.stopPropagation()
  emit('contextmenu', event)
}

function toggleTheme() {
  emit('contextmenu', new CustomEvent('nav-to-theme'))
}

function onMenuSelect(item: MtMenuItem) {
  showLogoMenu.value = false
  if (item.key) {
    switch (item.key as MenuKey) {
      case 'settings': emit('contextmenu', new CustomEvent('nav-to-settings')); break
      case 'commands': emit('contextmenu', new CustomEvent('nav-to-commands')); break
      case 'theme': toggleTheme(); break
      case 'quit': if (isTauri) invoke('quit_app').catch(() => {}); break
    }
  }
}

defineExpose({ focus: () => inputRef.value?.focus() })
</script>

<template>
  <div
    class="search-bar"
    :class="{ 'search-bar--focused': focused }"
    @mousedown="handleSearchBarMousedown"
    @mousemove="handleSearchBarMousemove"
    @mouseleave="handleSearchBarMouseleave"
  >
    <div class="search-bar__left">
      <div class="search-bar__icon" aria-hidden="true">
        <Search :size="18" :stroke-width="1.5" />
      </div>

      <div class="search-bar__input-wrapper">
        <input
          ref="inputRef"
          type="text"
          :value="modelValue"
          :placeholder="placeholder"
          class="search-bar__input"
          @input="(e) => emit('update:modelValue', (e.target as HTMLInputElement).value)"
          @keydown="onKeydown"
          @focus="focused = true"
          @blur="focused = false"
        />
      </div>
    </div>

    <div class="search-bar__right">
      <Transition name="fade">
        <button
          v-if="modelValue"
          class="search-bar__clear"
          type="button"
          @mousedown.stop.prevent
          @click.stop="emit('update:modelValue', '')"
          aria-label="清空"
        >
          <X :size="14" :stroke-width="2" />
        </button>
      </Transition>

      <div
        class="search-bar__logo"
        @mousedown.stop
        @click.stop="onLogoClick"
        @contextmenu.stop="onLogoContextMenu"
      >
        <img
          src="/logo/logo-only.png"
          alt="MonoTools"
          class="search-bar__logo-img"
          draggable="false"
        />
      </div>
    </div>

    <MtMenu
      :items="menuItems.map(item => ({
        key: item.key,
        label: item.label,
        icon: item.icon,
        danger: item.danger
      }))"
      v-model="showLogoMenu"
      :x="logoMenuPos.x"
      :y="logoMenuPos.y"
      :anchor="'pointer'"
      :min-width="180"
      @select="onMenuSelect"
    />
  </div>
</template>

<style scoped>
.search-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  height: 52px;
  flex-shrink: 0;
  position: relative;
  user-select: none;
  background: transparent;
  border-bottom: 1px solid var(--border-subtle);
  transition: all var(--dur-fast) var(--ease-out);
}

.search-bar--focused {
  background: rgba(255, 255, 255, 0.015);
}

.search-bar__left {
  display: flex;
  align-items: center;
  gap: var(--sp-4);
  flex: 1;
  min-width: 0;
}

.search-bar__right {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

.search-bar__icon {
  flex-shrink: 0;
  width: 18px;
  height: 18px;
  color: var(--text-tertiary);
  opacity: 0.65;
  transition: all var(--dur-fast) var(--ease-out);
}

.search-bar--focused .search-bar__icon {
  color: var(--accent);
  opacity: 1;
}

.search-bar__input-wrapper {
  flex: 1;
  min-width: 0;
}

.search-bar__input {
  width: 100%;
  padding: 0;
  font-family: var(--font-sans);
  font-size: 15px;
  font-weight: 400;
  line-height: 1.4;
  color: var(--text-primary);
  background: transparent;
  border: none;
  outline: none;
  caret-color: var(--accent);
  letter-spacing: -0.005em;
}

.search-bar__input::placeholder {
  color: var(--text-quaternary);
  transition: opacity var(--dur-fast) var(--ease-out);
  font-weight: 400;
}

.search-bar--focused .search-bar__input::placeholder {
  opacity: 0.55;
}

.search-bar__clear {
  flex-shrink: 0;
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-tertiary);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
}

.search-bar__clear:hover {
  background: var(--list-hover-bg);
  color: var(--text-secondary);
}

.search-bar__logo {
  flex-shrink: 0;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: all var(--dur-fast) var(--ease-out);
  -webkit-app-region: no-drag;
}

.search-bar__logo:hover {
  background: var(--list-hover-bg);
}

.search-bar__logo-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  filter: grayscale(100%) opacity(0.5);
  transition: all var(--dur-normal) var(--ease-out);
}

.search-bar__logo:hover .search-bar__logo-img {
  filter: grayscale(0%) opacity(1);
  transform: scale(1.05);
}

.search-bar__logo:active .search-bar__logo-img {
  transform: scale(0.95);
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity var(--dur-fast) var(--ease-out);
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
