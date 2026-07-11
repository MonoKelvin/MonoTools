import { describe, expect, it, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useSearchStore } from '@/stores/search'
import type { SearchResult } from '@/types/search'

const DEBOUNCE_MS = 80

function mk(over: Partial<SearchResult> = {}): SearchResult {
  return {
    id: 'r:' + Math.random().toString(36).slice(2, 8),
    title: 'sample',
    subtitle: '',
    icon: null,
    category: 'apps',
    resultType: 'system-app',
    action: { type: 'launch', data: 'x' },
    score: 0.5,
    ...over,
  }
}

describe('useSearchStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('topResults limits to 8', () => {
    const s = useSearchStore()
    s.results = Array.from({ length: 20 }, (_, i) => mk({ title: String(i) }))
    expect(s.topResults).toHaveLength(8)
  })

  it('filteredResults filters by activeCategory', () => {
    const s = useSearchStore()
    s.results = [
      mk({ category: 'apps' }),
      mk({ category: 'files' }),
      mk({ category: 'commands' }),
    ]
    s.activeCategory = 'files'
    expect(s.filteredResults).toHaveLength(1)
    s.activeCategory = 'all'
    expect(s.filteredResults).toHaveLength(3)
  })

  it('setCategory resets selectedIndex', () => {
    const s = useSearchStore()
    s.results = [mk(), mk()]
    s.selectedIndex = 1
    s.activeCategory = 'apps'
    s.setCategory('files')
    expect(s.selectedIndex).toBe(0)
  })

  it('selectNext / selectPrev', () => {
    const s = useSearchStore()
    s.results = [mk(), mk(), mk()]
    s.selectedIndex = 0
    s.selectNext()
    expect(s.selectedIndex).toBe(1)
    s.selectNext()
    s.selectNext()
    expect(s.selectedIndex).toBe(2) // stays at end
    s.selectPrev()
    expect(s.selectedIndex).toBe(1)
  })

  it('setQuery triggers debounce once', async () => {
    const s = useSearchStore()
    // 用 searchApi.search 计数：setQuery 会 debounce 调 → runSearch → searchApi.search
    const api = await import('@/services/searchApi')
    const search = vi.spyOn(api.searchApi, 'search').mockResolvedValue([] as any)
    s.setQuery('a')
    s.setQuery('ab')
    s.setQuery('abc')
    await new Promise((r) => setTimeout(r, DEBOUNCE_MS + 80))
    expect(search).toHaveBeenCalledTimes(1)
  })
})
