import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { SearchResult, SearchOptions, SearchCategory } from '@/types/search'
import { searchApi } from '@/services/searchApi'

export type ActiveCategory = 'all' | 'apps' | 'files' | 'commands' | 'startup'

const DEBOUNCE_MS = 60

export const useSearchStore = defineStore('search', () => {
  const query = ref('')
  const results = ref<SearchResult[]>([])
  const loading = ref(false)
  const activeCategory = ref<ActiveCategory>('all')
  const selectedIndex = ref(0)
  const visible = ref(false)

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
    if (!query.value.trim()) {
      results.value = []
      return
    }
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
    filteredResults,
    topResults,
    setQuery,
    runSearch,
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
