import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { SearchResult, SearchOptions, SearchCategory } from '@/types/search'
import { searchApi } from '@/services/searchApi'

export type ActiveCategory = 'all' | 'apps' | 'files' | 'commands'
export type IndexStatus = 'idle' | 'building' | 'completed' | 'error'

const DEBOUNCE_MS = 80

export const useSearchStore = defineStore('search', () => {
  const query = ref('')
  const results = ref<SearchResult[]>([])
  const loading = ref(false)
  const activeCategory = ref<ActiveCategory>('all')
  const selectedIndex = ref(0)
  const visible = ref(false)

  const indexStatus = ref<IndexStatus>('idle')
  const indexMessage = ref('')
  const indexStats = ref({ files: 0, apps: 0, commands: 0 })

  let debounceHandle: number | null = null

  const filteredResults = computed(() => {
    if (activeCategory.value === 'all') return results.value
    return results.value.filter((r) => r.category === activeCategory.value)
  })

  const topResults = computed(() => filteredResults.value.slice(0, 8))

  function setQuery(next: string) {
    query.value = next
    selectedIndex.value = 0
    if (debounceHandle) window.clearTimeout(debounceHandle)
    debounceHandle = window.setTimeout(() => runSearch(), DEBOUNCE_MS)
  }

  async function runSearch(options?: Partial<SearchOptions>) {
    loading.value = true
    try {
      results.value = await searchApi.search(query.value, options)
    } catch (err) {
      console.error('搜索失败：', err)
      results.value = []
    } finally {
      loading.value = false
    }
  }

  async function buildIndex() {
    if (indexStatus.value === 'building') return
    indexStatus.value = 'building'
    indexMessage.value = '正在构建索引...'
    try {
      await searchApi.buildIndex()
    } catch (err) {
      console.error('索引构建失败：', err)
      indexStatus.value = 'error'
      indexMessage.value = '索引构建失败'
    }
  }

  async function loadIndexStatus() {
    try {
      const stats = await searchApi.getIndexStatus()
      indexStats.value = stats
      if (stats.files > 0) {
        indexStatus.value = 'completed'
        indexMessage.value = `已索引 ${stats.files.toLocaleString()} 个文件`
      }
    } catch (err) {
      console.error('获取索引状态失败：', err)
    }
  }

  function setIndexProgress(progress: { status: string; message?: string; files?: number }) {
    switch (progress.status) {
      case 'building':
        indexStatus.value = 'building'
        indexMessage.value = progress.message || '正在构建索引...'
        break
      case 'completed':
        indexStatus.value = 'completed'
        if (progress.files) {
          indexStats.value.files = progress.files
          indexMessage.value = `索引完成，共 ${progress.files.toLocaleString()} 个文件`
        } else {
          indexMessage.value = '索引完成'
        }
        break
      case 'error':
        indexStatus.value = 'error'
        indexMessage.value = progress.message || '索引构建失败'
        break
      default:
        break
    }
  }

  function setCategory(c: ActiveCategory) {
    activeCategory.value = c
    selectedIndex.value = 0
  }

  function selectNext() {
    if (selectedIndex.value < filteredResults.value.length - 1) {
      selectedIndex.value++
    }
  }

  function selectPrev() {
    if (selectedIndex.value > 0) selectedIndex.value--
  }

  async function executeSelected(): Promise<SearchResult | null> {
    const item = filteredResults.value[selectedIndex.value]
    if (!item) return null
    try {
      await searchApi.execute(item)
    } finally {
      visible.value = false
    }
    return item
  }

  async function executeItem(item: SearchResult) {
    try {
      await searchApi.execute(item)
    } finally {
      visible.value = false
    }
  }

  function show() {
    visible.value = true
    query.value = ''
    results.value = []
    selectedIndex.value = 0
  }

  function hide() {
    visible.value = false
  }

  function toggle() {
    visible.value ? hide() : show()
  }

  return {
    query,
    results,
    loading,
    activeCategory,
    selectedIndex,
    visible,
    indexStatus,
    indexMessage,
    indexStats,
    filteredResults,
    topResults,
    setQuery,
    runSearch,
    buildIndex,
    loadIndexStatus,
    setIndexProgress,
    setCategory,
    selectNext,
    selectPrev,
    executeSelected,
    executeItem,
    show,
    hide,
    toggle,
  }
})
