<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Search, X } from 'lucide-vue-next'
import type { SearchResult } from '../types/search'

const props = defineProps<{
  placeholder?: string
  maxResults?: number
}>()

const emit = defineEmits<{
  select: [item: SearchResult]
  action: [item: SearchResult, action: string]
}>()

const query = ref('')
const results = ref<SearchResult[]>([])
const isLoading = ref(false)
const selectedIndex = ref(0)
const isFocused = ref(false)
const inputRef = ref<HTMLInputElement>()

const placeholderText = computed(() => props.placeholder || '搜索应用、文件、工作区...')
const resultCount = computed(() => results.value.length)
const modeBadge = computed(() => {
  if (query.value.startsWith('>')) return '命令'
  if (query.value.startsWith('=')) return '计算器'
  if (query.value.startsWith('?')) return '帮助'
  return '搜索'
})

// 搜索防抖
let searchTimeout: ReturnType<typeof setTimeout> | null = null

async function performSearch(queryText: string) {
  if (queryText.length < 2) {
    results.value = []
    return
  }

  isLoading.value = true
  try {
    const response = await invoke<SearchResult[]>('execute_command', {
      input: `search:all query="${queryText}" limit=${props.maxResults || 50}`
    })

    if (response.success) {
      results.value = response.data || []
    } else {
      console.error('Search failed:', response.error)
      results.value = []
    }
  } catch (error) {
    console.error('Search error:', error)
    results.value = []
  } finally {
    isLoading.value = false
  }
}

function onInput(event: Event) {
  const target = event.target as HTMLInputElement
  const value = target.value

  // 清除之前的定时器
  if (searchTimeout) {
    clearTimeout(searchTimeout)
  }

  // 80ms 防抖
  searchTimeout = setTimeout(() => {
    performSearch(value)
  }, 80)

  // 重置选中项
  selectedIndex.value = 0
}

function onKeydown(event: KeyboardEvent) {
  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault()
      selectedIndex.value = Math.min(selectedIndex.value + 1, results.value.length - 1)
      break

    case 'ArrowUp':
      event.preventDefault()
      selectedIndex.value = Math.max(selectedIndex.value - 1, 0)
      break

    case 'Enter':
      event.preventDefault()
      if (results.value.length > 0) {
        const selected = results.value[selectedIndex.value]
        onSelectItem(selected)
      }
      break

    case 'Escape':
      event.preventDefault()
      if (query.value) {
        // 第一次按 Escape：清空输入
        clearQuery()
      } else {
        // 第二次按 Escape：隐藏窗口
        window.__TAURI__?.hideWindow()
      }
      break

    case 'Tab':
      if (results.value.length > 0) {
        event.preventDefault()
        // TODO: 切换动作（如果有多个动作）
      }
      break
  }
}

function onSelectItem(item: SearchResult) {
  emit('select', item)
  // 执行默认动作
  if (item.actions && item.actions.length > 0) {
    emit('action', item, item.actions[0].id)
  }
  clearQuery()
}

function clearQuery() {
  query.value = ''
  results.value = []
  selectedIndex.value = 0
  inputRef.value?.focus()
}

function focusInput() {
  inputRef.value?.focus()
}

// 监听搜索结果变化，自动聚焦
watch(results, (newResults) => {
  if (newResults.length > 0 && selectedIndex.value >= newResults.length) {
    selectedIndex.value = 0
  }
})

onMounted(() => {
  focusInput()

  // 监听来自后端的搜索建议
  listen('search:suggestion', (event) => {
    // TODO: 处理实时搜索建议
  })
})

onUnmounted(() => {
  if (searchTimeout) {
    clearTimeout(searchTimeout)
  }
})
</script>

<template>
  <div
    class="search-box search-box-enter"
    :class="{ 'is-active': isFocused }"
    @click="focusInput"
  >
    <!-- 输入区域 -->
    <div class="search-input-wrapper">
      <Search :size="18" class="search-icon" />

      <input
        ref="inputRef"
        v-model="query"
        class="search-input"
        :placeholder="placeholderText"
        @input="onInput"
        @keydown="onKeydown"
        @focus="isFocused = true"
        @blur="isFocused = false"
      />

      <button
        v-if="query"
        class="search-clear"
        type="button"
        @click.stop="clearQuery"
        title="清空"
      >
        <X :size="14" />
      </button>

      <div v-if="query" class="search-mode-badge">
        {{ modeBadge }}
      </div>
    </div>

    <!-- 结果区域 -->
    <div v-if="results.length > 0" class="search-results">
      <div
        v-for="(item, index) in results"
        :key="item.id"
        class="result-item"
        :class="{ 'is-selected': index === selectedIndex }"
        :style="{ animationDelay: `${index * 30}ms` }"
        @mouseenter="selectedIndex = index"
        @click.stop="onSelectItem(item)"
      >
        <div class="result-icon">
          <img v-if="item.icon" :src="item.icon" :alt="item.title" />
          <span v-else class="result-icon-placeholder">{{ item.title[0]?.toUpperCase() }}</span>
        </div>

        <div class="result-content">
          <div class="result-title" v-html="item.title" />
          <div v-if="item.subtitle" class="result-subtitle truncate">
            {{ item.subtitle }}
          </div>
        </div>

        <div class="result-meta">
          <span class="result-source">{{ item.source }}</span>
        </div>
      </div>
    </div>

    <!-- 空状态 -->
    <div v-else-if="query && !isLoading" class="search-empty">
      <p class="empty-text">未找到相关结果</p>
    </div>

    <!-- 加载状态 -->
    <div v-else-if="isLoading" class="search-loading">
      <div class="loading-spinner" />
    </div>

    <!-- 底部状态栏 -->
    <div class="search-footer">
      <span class="footer-hint">
        <template v-if="results.length > 0">
          {{ resultCount }} 个结果
        </template>
        <template v-else>
          输入关键词开始搜索
        </template>
      </span>
      <span class="footer-shortcuts">
        <kbd>↑↓</kbd> 导航 <kbd>Enter</kbd> 打开 <kbd>Esc</kbd> 关闭
      </span>
    </div>
  </div>
</template>

<style scoped>
.search-box {
  width: 800px;
  max-width: 90vw;
  background: var(--mt-surface-1);
  border: 1px solid var(--mt-hairline);
  border-radius: var(--mt-radius-xl);
  padding: var(--mt-space-md);
  box-shadow: 0 24px 48px rgba(0, 0, 0, 0.4);
  overflow: hidden;
}

.search-input-wrapper {
  display: flex;
  align-items: center;
  gap: var(--mt-space-sm);
  padding: var(--mt-space-sm) var(--mt-space-md);
  background: var(--mt-canvas);
  border: 1px solid var(--mt-hairline);
  border-radius: var(--mt-radius-md);
  transition: border-color 0.15s ease;
}

.search-input-wrapper:focus-within {
  border-color: var(--mt-primary-focus);
  box-shadow: 0 0 0 2px rgba(94, 106, 210, 0.25);
}

.search-icon {
  color: var(--mt-ink-subtle);
  flex-shrink: 0;
}

.search-input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: var(--mt-ink);
  font-family: var(--mt-font-body);
  font-size: 16px;
  line-height: 1.5;
}

.search-input::placeholder {
  color: var(--mt-ink-subtle);
}

.search-clear {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  background: var(--mt-surface-2);
  border: none;
  border-radius: var(--mt-radius-pill);
  color: var(--mt-ink-subtle);
  cursor: pointer;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.search-clear:hover {
  background: var(--mt-surface-3);
  color: var(--mt-ink);
}

.search-mode-badge {
  padding: 2px 8px;
  background: var(--mt-surface-2);
  border-radius: var(--mt-radius-pill);
  font-size: 12px;
  color: var(--mt-ink-subtle);
  font-family: var(--mt-font-mono);
  flex-shrink: 0;
}

.search-results {
  max-height: 360px;
  overflow-y: auto;
  margin-top: var(--mt-space-sm);
}

.result-item {
  display: flex;
  align-items: center;
  gap: var(--mt-space-sm);
  padding: 10px 14px;
  border-radius: var(--mt-radius-md);
  cursor: pointer;
  transition: background-color 0.1s ease;
}

.result-item:hover,
.result-item.is-selected {
  background: var(--mt-surface-2);
}

.result-icon {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--mt-surface-3);
  border-radius: var(--mt-radius-sm);
  flex-shrink: 0;
  overflow: hidden;
}

.result-icon img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.result-icon-placeholder {
  font-size: 16px;
  font-weight: 600;
  color: var(--mt-ink-muted);
}

.result-content {
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.result-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--mt-ink);
  line-height: 1.4;
}

.result-subtitle {
  font-size: 12px;
  color: var(--mt-ink-subtle);
  line-height: 1.4;
  margin-top: 2px;
}

.result-meta {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
  flex-shrink: 0;
}

.result-source {
  font-size: 11px;
  color: var(--mt-ink-tertiary);
  padding: 2px 6px;
  background: var(--mt-surface-3);
  border-radius: var(--mt-radius-xs);
}

.search-empty,
.search-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--mt-space-xl) 0;
  color: var(--mt-ink-subtle);
}

.empty-text {
  font-size: 14px;
}

.loading-spinner {
  width: 24px;
  height: 24px;
  border: 2px solid var(--mt-surface-3);
  border-top-color: var(--mt-primary);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.search-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: var(--mt-space-sm);
  margin-top: var(--mt-space-sm);
  border-top: 1px solid var(--mt-hairline);
  font-size: 12px;
  color: var(--mt-ink-tertiary);
}

.footer-hint {
  font-family: var(--mt-font-mono);
}

.footer-shortcuts {
  display: flex;
  gap: var(--mt-space-xs);
}

.footer-shortcuts kbd {
  padding: 2px 6px;
  background: var(--mt-surface-2);
  border: 1px solid var(--mt-hairline);
  border-radius: var(--mt-radius-xs);
  font-family: var(--mt-font-mono);
  font-size: 11px;
}
</style>
