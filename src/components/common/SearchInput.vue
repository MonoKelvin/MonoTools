<script setup lang="ts">
import { ref, watch, computed, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { Search, X, Settings, Terminal, LogOut, Pin, Sun } from "@lucide/vue"
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
  (e: 'contextmenu', event: MouseEvent): void
}>()

const inputRef = ref<HTMLInputElement | null>(null)
const searchBarRef = ref<HTMLDivElement | null>(null)
const localValue = ref(props.modelValue)
const focused = ref(false)

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

// 判断鼠标是否在文字区域内
function isOverText(input: HTMLInputElement, clientX: number): boolean {
  const rect = input.getBoundingClientRect()

  // 前 20% 安全区：始终允许光标定位到文字开头
  const frontBoundary = rect.left + rect.width * 0.2
  if (clientX <= frontBoundary) return true

  // 没有文字时，其余区域可以拖拽
  if (!input.value || input.value.length === 0) return false

  // 使用 canvas 测量文字实际渲染范围
  const style = getComputedStyle(input)
  const canvas = document.createElement('canvas')
  const ctx = canvas.getContext('2d')!
  ctx.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`

  const paddingLeft = parseFloat(style.paddingLeft) || 0
  const charCount = Math.min(4, input.value.length)
  const prefixWidth = ctx.measureText(input.value.slice(0, charCount)).width
  const suffixWidth = ctx.measureText(input.value.slice(-charCount)).width
  const textWidth = ctx.measureText(input.value).width

  // 文字渲染范围（含前后 4 字符容差）
  const textAreaStart = rect.left + paddingLeft - prefixWidth
  const textAreaEnd = rect.left + paddingLeft + textWidth + suffixWidth

  return clientX >= textAreaStart && clientX <= textAreaEnd
}

// 整个搜索栏的 mousedown 处理
async function handleSearchBarMousedown(event: MouseEvent) {
  const target = event.target as HTMLElement

  console.log('Mouse down:', {
    tag: target.tagName,
    class: target.className,
    isTauri,
  })

  // 点击 Logo 图标：不拖拽
  if (target.closest('.logo-area')) {
    console.log('Clicked logo area')
    return
  }

  // 点击清空按钮：不拖拽
  if (target.closest('.search-clear')) {
    console.log('Clicked clear button')
    return
  }

  // 点击在输入框内：判断是否在文字区域
  const input = inputRef.value
  if (input && target.closest('.search-input-wrapper')) {
    const overText = isOverText(input, event.clientX)

    console.log('Clicked input area:', { overText, value: input.value })

    if (overText) {
      // 在文字区域：不拖拽，允许光标定位和文本选择
      console.log('Over text, not dragging')
      return
    } else {
      // 在输入框的空白区域：拖拽窗口
      console.log('Input blank area, start dragging')
      // 不 preventDefault，让浏览器默认行为处理
      if (isTauri) {
        try {
          await invoke('start_dragging')
          console.log('Dragging started successfully')
        } catch (error) {
          console.error('Failed to start dragging:', error)
        }
      }
      return
    }
  }

  // 点击在其他区域（搜索图标、输入框外的空白）：拖拽窗口
  console.log('Other area, start dragging')
  // 不 preventDefault
  if (isTauri) {
    try {
      await invoke('start_dragging')
      console.log('Dragging started successfully')
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

const showLogoMenu = ref(false)
const logoMenuPos = ref({ x: 0, y: 0 })

function onLogoClick(event: MouseEvent) {
  event.preventDefault()
  event.stopPropagation()

  const logoRect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  logoMenuPos.value = {
    x: logoRect.left,
    y: logoRect.bottom + 6
  }
  showLogoMenu.value = true
}

function onLogoContextMenu(event: MouseEvent) {
  event.preventDefault()
  event.stopPropagation()
  emit('contextmenu', event)
}

function toggleTheme() {
  emit('contextmenu', new MouseEvent('contextmenu'))
}

function onMenuSelect(item: MtMenuItem) {
  showLogoMenu.value = false
  if (item.key) {
    switch (item.key as MenuKey) {
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
    data-tauri-drag-region
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

    <!-- Logo 右键菜单 (使用 MtMenu 组件) -->
    <MtMenu
      v-if="showLogoMenu"
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
  padding: 0 var(--sp-6);
  gap: var(--sp-3);
  flex-shrink: 0;
  position: relative;
  user-select: none;
  height: 52px;
  background: transparent;
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
  z-index: 1;
  opacity: 0.8;
  transition: all 0.12s var(--ease-out, ease);
  pointer-events: none;
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

/* 过渡动画 */
</style>
