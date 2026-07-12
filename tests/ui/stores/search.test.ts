import { describe, expect, it, beforeEach, afterEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { nextTick } from 'vue'
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
    // 设置 query 让 displayList 只包含搜索结果 (不含 recent/system 额外项)
    s.setQuery('test')
    s.results = [mk(), mk(), mk()]
    s.selectedIndex = 0
    s.selectNext()
    expect(s.selectedIndex).toBe(1)
    s.selectNext()
    s.selectNext()
    // 搜索时, displayMax === 3 (来自搜索结果)
    // 但 setQuery 重置 selectedIndex=0, 然后 selectNext 三次到 3
    // 实际: setQuery 在 debounce 之后才生效, 这里直接 setQuery 不会立即触发
    // 所以 displayMax 仍是当前 results 的全集. 3 次 selectNext 应该到 3
    expect(s.selectedIndex).toBeLessThanOrEqual(s.displayMax - 1)
    s.selectPrev()
    expect(s.selectedIndex).toBeGreaterThanOrEqual(0)
  })

  /**
   * 修复键盘导航: 当 selectedIndex 超出当前 results 上限时, 应主动 clamp.
   * 之前 selectedIndex 可能指向一个"看不见的项"造成 UI 无响应.
   */
  it('selectNext clamps when results shrink', async () => {
    const s = useSearchStore()
    s.setQuery('test') // 进入搜索模式, displayList = filteredResults
    s.results = [mk(), mk(), mk(), mk(), mk()]
    s.selectedIndex = 4
    s.results = [mk(), mk()]
    // 触发 watch: filteredResults 变了 → 自动 clamp
    await Promise.resolve()
    expect(s.selectedIndex).toBeLessThanOrEqual(s.displayMax - 1)
  })

  it('setQuery triggers debounce once', async () => {
    // 用 fake timers 隔离之前测试遗留的 setTimeout, 它们可能引用旧的 store 闭包
    // 并在当前测试的 spy 装配后触发 runSearch, 污染计数.
    vi.useFakeTimers()
    try {
      const s = useSearchStore()
      const api = await import('@/services/searchApi')
      const search = vi.spyOn(api.searchApi, 'search').mockResolvedValue([] as any)
      s.setQuery('a')
      s.setQuery('ab')
      s.setQuery('abc')
      // store 内部 DEBOUNCE_MS = 30, 推进 40ms 足够让最后一次 setTimeout 触发
      vi.advanceTimersByTime(40)
      // 让 spy 返回的 Promise.resolve([]) 落定
      await Promise.resolve()
      expect(search).toHaveBeenCalledTimes(1)
    } finally {
      vi.useRealTimers()
    }
  })

  /**
   * === Section 1 回归测试: selectedGlobalId 锚点 ===
   * 验证 search 列表重排后, 选中态能"跟着 ID 走"而非"跟着 index 走".
   *
   * 注意: 在搜索模式下 (query 非空), displayList = filteredResults,
   * 不会包含 recent/system 等额外分组. 所以 3 个结果 = 3 个显示项.
   */
  it('selectByIndex clamps to displayMax - 1', () => {
    const s = useSearchStore()
    s.query = 'test' // 搜索模式, displayList = filteredResults
    s.results = [mk(), mk(), mk()]
    s.selectByIndex(99)
    expect(s.selectedIndex).toBe(2) // 越界 clamp 到 displayMax-1
    s.selectByIndex(-5)
    expect(s.selectedIndex).toBe(0) // 负数 clamp 到开头
  })

  it('selectByIndex updates selectedGlobalId anchor', () => {
    const s = useSearchStore()
    s.query = 'test'
    const items = [mk({ id: 'a' }), mk({ id: 'b' }), mk({ id: 'c' })]
    s.results = items
    s.selectByIndex(1)
    expect(s.selectedGlobalId).toBe('b')
    s.selectByIndex(0)
    expect(s.selectedGlobalId).toBe('a')
  })

  it('selectedGlobalId survives across results mutation when id persists', async () => {
    const s = useSearchStore()
    s.query = 'test'
    const persistent = mk({ id: 'persistent' })
    const initial = [mk({ id: 'x' }), persistent, mk({ id: 'y' })]
    s.results = initial
    s.selectByIndex(1)
    expect(s.selectedGlobalId).toBe('persistent')

    // 用相同 id 但不同顺序的 results 替换, watcher 应自动重新定位到该 id
    s.results = [mk({ id: 'a' }), mk({ id: 'b' }), persistent, mk({ id: 'c' })]
    await Promise.resolve()
    expect(s.selectedGlobalId).toBe('persistent')
    expect(s.selectedIndex).toBe(2) // 重排后该 id 在新位置
  })

  it('displayList === 0 sets selectedIndex = 0 without crash', async () => {
    const s = useSearchStore()
    s.query = 'test'
    s.results = [mk(), mk(), mk()]
    s.selectByIndex(2)
    // 验证 selectByIndex 真的把 selectedIndex 设到 2 (displayMax 此时为 3)
    expect(s.selectedIndex).toBe(2)
    s.results = []
    // displayMax watcher (flush: 'pre' by default) 在微任务里执行; nextTick
    // 已经足够, 但稳起见再多等一个微任务 + 一个 macrotask.
    await nextTick()
    await Promise.resolve()
    expect(s.displayMax).toBe(0)
    expect(s.selectedIndex).toBe(0)
    expect(s.selectedGlobalId).toBeNull()
  })

  /**
   * === Section 2 回归测试: 分组折叠/展开对 displayList 的影响 ===
   * 折叠某个分组后, 该组的项不应出现在 displayList 中.
   */
  it('toggleGroupCollapse removes group items from displayList', () => {
    const s = useSearchStore()
    s.query = '' // 非搜索模式, 才有 system / apps 等分组
    s.results = Array.from({ length: 12 }, (_, i) =>
      mk({ id: `app-${i}`, title: `App ${i}`, resultType: 'system-app' }),
    )
    const beforeMax = s.displayMax
    expect(beforeMax).toBeGreaterThan(0)
    // 折叠系统应用分组
    s.toggleGroupCollapse('group.system')
    expect(s.displayMax).toBeLessThan(beforeMax)
    // 再展开恢复
    s.toggleGroupCollapse('group.system')
    expect(s.displayMax).toBe(beforeMax)
  })

  it('selectNext respects displayMax on collapsing a group containing the selection', () => {
    const s = useSearchStore()
    s.query = ''
    s.results = Array.from({ length: 8 }, (_, i) =>
      mk({ id: `app-${i}`, title: `App ${i}`, resultType: 'system-app' }),
    )
    s.selectByIndex(2)
    expect(s.selectedIndex).toBe(2)
    // 折叠系统应用分组
    s.toggleGroupCollapse('group.system')
    // selectedIndex 应被 clamp 到有效范围, 不会指向不可见项
    expect(s.selectedIndex).toBeGreaterThanOrEqual(0)
    expect(s.selectedIndex).toBeLessThan(s.displayMax)
  })
})
