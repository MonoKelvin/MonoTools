<script setup lang="ts">
import { ref, watch, computed, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { Search, X, Settings, Terminal, LogOut, Pin, Sun } from "@lucide/vue"
import { invoke } from '@tauri-apps/api/core'
import { isTauri } from '@/services/env'

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
  (e: 'contextmenu', event: MouseEvent): void
}>()

const inputRef = ref<HTMLInputElement | null>(null)
const searchBarRef = ref<HTMLDivElement | null>(null)
const localValue = ref(props.modelValue)
const focused = ref(false)
const showMenu = ref(false)
const menuPos = ref({ x: 0, y: 0 })

watch(() => props.modelValue, (v) => (localValue.value = v))
watch(localValue, (v) => emit('update:modelValue', v))

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter') { e.preventDefault(); emit('enter') }
  else if (e.key === 'ArrowDown') { e.preventDefault(); emit('arrowDown') }
  else if (e.key === 'ArrowUp') { e.preventDefault(); emit('arrowUp') }
  else if (e.key === 'Escape') { e.preventDefault(); emit('escape') }
}

onMounted(() => {
  inputRef.value?.focus()
})

// ========== 拖拽逻辑 ==========

function isOverText(input: HTMLInputElement, clientX: number): boolean {
  const rect = input.getBoundingClientRect()
  const frontBoundary = rect.left + rect.width * 0.2
  if (clientX <= frontBoundary) return true

  if (!input.value || input.value.length === 0) return false

  const style = getComputedStyle(input)
  const canvas = document.createElement('canvas')
  const ctx = canvas.getContext('2d')!
  ctx.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`

  const paddingLeft = parseFloat(style.paddingLeft) || 0
  const charCount = Math.min(4, input.value.length)
  const prefixWidth = ctx.measureText(input.value.slice(0, charCount)).width
  const suffixWidth = ctx.measureText(input.value.slice(-charCount)).width
  const textWidth = ctx.measureText(input.value).width

  const textAreaStart = rect.left + paddingLeft - prefixWidth
  const textAreaEnd = rect.left + paddingLeft + textWidth + suffixWidth

  return clientX >= textAreaStart && clientX <= textAreaEnd
}

async function handleSearchBarMousedown(event: MouseEvent) {
  const target = event.target as HTMLElement

  // 点击输入框区域：不拖拽
  if (target.closest('.search-input-wrapper')) {
    return
  }

  // 点击 Logo 图标：触发右键菜单
  if (target.closest('.logo-area')) {
    return
  }

  // 点击其他区域：拖拽窗口
  event.preventDefault()
  if (isTauri) {
    try {
      await invoke('start_dragging')
    } catch (error) {
      console.error('Failed to start dragging:', error)
    }
  }
}

function handleSearchBarMousemove(event: MouseEvent) {
  const input = inputRef.value
  if (!input) return

  const overText = isOverText(input, event.clientX)
  const cursor = overText ? 'text' : 'default'

  const wrapper = event.currentTarget as HTMLElement
  wrapper.style.cursor = cursor
  input.style.cursor = cursor
}

function handleSearchBarMouseleave() {
  const input = inputRef.value
  if (input) {
    input.style.cursor = ''
  }
}

// ========== Logo 菜单 ==========

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

function onLogoClick(event: MouseEvent) {
  event.preventDefault()
  event.stopPropagation()

  // 计算菜单位置，限制在窗口内
  const logoRect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  const menuWidth = 220
  const menuHeight = 200

  let x = logoRect.left
  let y = logoRect.bottom + 4

  // 防止菜单超出窗口右边界
  if (x + menuWidth > window.innerWidth) {
    x = window.innerWidth - menuWidth - 8
  }

  // 防止菜单超出窗口下边界
  if (y + menuHeight > window.innerHeight) {
    y = logoRect.top - menuHeight - 4
  }

  menuPos.value = { x, y }
  showMenu.value = true
}

function onLogoContextMenu(event: MouseEvent) {
  event.preventDefault()
  event.stopPropagation()
  emit('contextmenu', event)
}

function toggleTheme() {
  emit('contextmenu', new MouseEvent('contextmenu'))
}

function onMenuSelect(key: MenuKey) {
  showMenu.value = false
  switch (key) {
    case 'settings':
      emit('contextmenu', new CustomEvent('nav-to-settings'))
      break
    case 'commands':
      emit('contextmenu', new CustomEvent('nav-to-commands'))
      break
    case 'theme':
      toggleTheme()
      break
    case 'quit':
      if (isTauri) invoke('quit_app').catch((e) => console.error('quit failed:', e))
      break
  }
}

// 点击外部关闭菜单
function handleClickOutside(event: MouseEvent) {
  if (showMenu.value && !(event.target as HTMLElement).closest('.logo-menu')) {
    showMenu.value = false
  }
}

onMounted(() => {
  document.addEventListener('mousedown', handleClickOutside)
})

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', handleClickOutside)
})

function focus() {
  inputRef.value?.focus()
}

defineExpose({ focus })
</script>

<template>
  <div
    ref="searchBarRef"
    class="search-bar"
    :class="{ 'is-focused': focused }"
    @mousedown="handleSearchBarMousedown"
    @mousemove="handleSearchBarMousemove"
    @mouseleave="handleSearchBarMouseleave"
  >
    <!-- 搜索图标 -->
    <div class="search-icon" aria-hidden="true">
      <Search :size="18" :stroke-width="2" />
    </div>

    <!-- 输入框 -->
    <div class="search-input-wrapper">
      <input
        ref="inputRef"
        type="text"
        :value="modelValue"
        :placeholder="placeholder"
        class="search-input"
        @input="(e) => emit('update:modelValue', (e.target as HTMLInputElement).value)"
        @keydown="onKeydown"
        @focus="focused = true"
        @blur="focused = false"
      />
    </div>

    <!-- 清空按钮 -->
    <Transition name="fade">
      <button
        v-if="modelValue"
        class="search-clear"
        type="button"
        @mousedown.stop.prevent
        @click.stop="emit('update:modelValue', '')"
        aria-label="清空"
      >
        <X :size="14" :stroke-width="2.5" />
      </button>
    </Transition>

    <!-- Logo -->
    <div
      class="logo-area"
      @mousedown.stop.prevent
      @click.stop="onLogoClick"
      @contextmenu.stop="onLogoContextMenu"
    >
      <img
        src="/logo/logo-only.png"
        alt="MonoTools"
        class="logo-img"
        draggable="false"
      />
    </div>

    <!-- Logo 右键菜单 -->
    <Teleport to="body">
      <Transition name="menu-fade">
        <div
          v-if="showMenu"
          class="logo-menu-container"
          :style="{ left: menuPos.x + 'px', top: menuPos.y + 'px' }"
        >
          <div class="logo-menu">
            <button
              v-for="item in menuItems"
              :key="item.key"
              class="menu-item"
              :class="{ 'menu-item--danger': item.danger }"
              @click="onMenuSelect(item.key)"
            >
              <component :is="item.icon" :size="16" :stroke-width="2" />
              <span class="menu-label">{{ item.label }}</span>
            </button>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.search-bar {
  display: flex;
  align-items: center;
  padding: 0 var(--sp-6);
  gap: var(--sp-3);
  flex-shrink: 0;
  position: relative;
  user-select: none;
  height: 52px;
  background: transparent;
  -webkit-app-region: drag;
}

/* 搜索图标 */
.search-icon {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  color: var(--text-secondary);
  pointer-events: none;
  z-index: 1;
  opacity: 0.8;
  transition: all 0.12s var(--ease-out, ease);
}

.search-bar.is-focused .search-icon {
  color: var(--text-secondary);
  opacity: 1;
}

/* 输入框容器 */
.search-input-wrapper {
  flex: 1;
  min-width: 0;
  -webkit-app-region: no-drag;
  overflow: hidden;
}

/* 输入框 */
.search-input {
  width: 100%;
  height: 100%;
  padding: 0;
  font-family: var(--font-sans);
  font-size: var(--text-xl);
  font-weight: 400;
  line-height: 1.4;
  color: var(--text-primary);
  background: transparent;
  border: none;
  outline: none;
  caret-color: var(--accent);
  transition: all 0.12s var(--ease-out, ease);
}

.search-input::placeholder {
  color: var(--text-tertiary);
  transition: all 0.12s var(--ease-out, ease);
}

.search-input:focus::placeholder {
  opacity: 0.5;
}

/* 清空按钮 */
.search-clear {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.12s var(--ease-out, ease);
}

.search-clear:hover {
  background: var(--surface-overlay);
  color: var(--text-primary);
}

/* Logo */
.logo-area {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: all 0.2s var(--ease-out, ease);
  -webkit-app-region: no-drag;
  z-index: 10;
  margin-left: var(--sp-3);
}

.logo-area:hover {
  background: transparent;
}

.logo-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  filter: grayscale(100%) opacity(0.6);
  transition: all 0.2s var(--ease-out, ease);
}

.logo-area:hover .logo-img {
  filter: grayscale(0%) opacity(1)
          drop-shadow(0 0 8px rgba(94, 106, 210, 0.6))
          drop-shadow(0 0 16px rgba(94, 106, 210, 0.3));
  transform: scale(1.05);
}

.logo-area:active .logo-img {
  transform: scale(0.95);
  transition: all 0.12s var(--ease-out, ease);
}

/* Logo 菜单容器 */
.logo-menu-container {
  position: fixed;
  z-index: 9999;
  pointer-events: none;
  max-width: 320px;
  max-height: 400px;
  overflow: hidden;
}

/* Logo 菜单 */
.logo-menu {
  min-width: 220px;
  padding: var(--sp-2);
  background: rgba(35, 35, 35, 0.85);
  -webkit-backdrop-filter: blur(40px) saturate(180%);
  backdrop-filter: blur(40px) saturate(180%);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: var(--radius-lg);
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.15),
    0 8px 24px rgba(0, 0, 0, 0.3),
    0 0 48px rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
  gap: 1px;
  overflow-y: auto;
  max-height: 400px;
  pointer-events: auto;
}

/* 菜单项 */
.menu-item {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  width: 100%;
  padding: var(--sp-2) var(--sp-3);
  border: none;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-primary);
  font-family: var(--font-sans);
  font-size: var(--text-sm);
  font-weight: 400;
  line-height: 1.4;
  cursor: pointer;
  transition: background-color 0.12s var(--ease-out, ease);
  text-align: left;
  white-space: nowrap;
}

.menu-item:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.08);
}

.menu-item:active:not(:disabled) {
  background: rgba(255, 255, 255, 0.12);
  transform: scale(0.98);
}

.menu-item svg {
  flex-shrink: 0;
  width: 16px;
  height: 16px;
  color: var(--text-secondary);
  transition: color 0.12s var(--ease-out, ease);
}

.menu-item:hover svg {
  color: var(--text-primary);
}

.menu-item--danger .menu-label {
  color: #ff5f57;
}

.menu-item--danger:hover {
  background: rgba(255, 95, 87, 0.15);
}

.menu-item--danger:active {
  background: rgba(255, 95, 87, 0.2);
}

.menu-label {
  flex: 1;
  color: var(--text-primary);
  font-weight: 400;
}

/* 过渡动画 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.12s var(--ease-out, ease);
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.menu-fade-enter-active,
.menu-fade-leave-active {
  transition: opacity 0.12s var(--ease-out, ease), transform 0.12s var(--ease-out, ease);
}

.menu-fade-enter-from,
.menu-fade-leave-to {
  opacity: 0;
  transform: scale(0.95) translateY(-4px);
}
</style>
