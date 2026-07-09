<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
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

watch(() => props.modelValue, (v) => {})

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter') { e.preventDefault(); emit('enter') }
  else if (e.key === 'ArrowDown') { e.preventDefault(); emit('arrowDown') }
  else if (e.key === 'ArrowUp') { e.preventDefault(); emit('arrowUp') }
  else if (e.key === 'Escape') { e.preventDefault(); emit('escape') }
}

onMounted(() => {
  inputRef.value?.focus()
})

function isOverText(input: HTMLInputElement, clientX: number): boolean {
  const rect = input.getBoundingClientRect()
  if (!input.value || input.value.length === 0) {
    return clientX <= rect.left + rect.width * 0.4
  }

  const style = getComputedStyle(input)
  const canvas = document.createElement('canvas')
  const ctx = canvas.getContext('2d')!
  ctx.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`

  const paddingLeft = parseFloat(style.paddingLeft) || 0
  const charWidth = ctx.measureText('m').width
  const bufferWidth = charWidth * 4
  const textWidth = ctx.measureText(input.value).width

  const textAreaStart = rect.left + paddingLeft - bufferWidth
  const textAreaEnd = rect.left + paddingLeft + textWidth + bufferWidth

  return clientX >= textAreaStart && clientX <= textAreaEnd
}

async function handleSearchBarMousedown(event: MouseEvent) {
  const target = event.target as HTMLElement

  if (target.closest('.logo-area')) {
    return
  }

  if (showLogoMenu.value) {
    showLogoMenu.value = false
  }

  if (target.closest('.search-clear')) {
    return
  }

  const input = inputRef.value
  if (input && target.closest('.search-input-wrapper')) {
    const overText = isOverText(input, event.clientX)
    if (overText) {
      return
    } else {
      event.stopPropagation()
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
      return
    }
  }

  if (isTauri) {
    try {
      await invoke('start_dragging')
    } catch {}
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
  logoMenuPos.value = {
    x: event.clientX + offsetX,
    y: event.clientY + offsetY
  }
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
        if (isTauri) invoke('quit_app').catch(() => {})
        break
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
    data-tauri-drag-region
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
  padding: 0 var(--sp-5);
  height: 56px;
  flex-shrink: 0;
  position: relative;
  user-select: none;
  background: var(--surface);
  border-bottom: 1px solid var(--border-subtle);
  transition: all var(--dur-fast) var(--ease-out);
}

.search-bar--focused {
  background: var(--surface-raised);
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
  opacity: 0.7;
  transition: all var(--dur-fast) var(--ease-out);
}

.search-bar--focused .search-bar__icon {
  color: var(--text-secondary);
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
  font-size: var(--text-lg);
  font-weight: 400;
  line-height: 1.4;
  color: var(--text-primary);
  background: transparent;
  border: none;
  outline: none;
  caret-color: var(--accent);
}

.search-bar__input::placeholder {
  color: var(--text-quaternary);
  transition: opacity var(--dur-fast) var(--ease-out);
}

.search-bar--focused .search-bar__input::placeholder {
  opacity: 0.6;
}

.search-bar__clear {
  flex-shrink: 0;
  width: 24px;
  height: 24px;
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
  background: var(--surface-overlay);
  color: var(--text-secondary);
}

.search-bar__logo {
  flex-shrink: 0;
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: all var(--dur-fast) var(--ease-out);
  -webkit-app-region: no-drag;
}

.search-bar__logo:hover {
  background: var(--surface-overlay);
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
