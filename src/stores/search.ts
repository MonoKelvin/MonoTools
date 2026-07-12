import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { SearchResult, SearchOptions, SearchCategory } from '@/types/search'
import { searchApi } from '@/services/searchApi'

export type ActiveCategory = 'all' | 'apps' | 'files' | 'commands'
export type IndexStatus = 'idle' | 'building' | 'completed' | 'error'

const DEBOUNCE_MS = 0

export const useSearchStore = defineStore('search', () => {
  const query = ref('')
  const results = ref<SearchResult[]>([])
  const loading = ref(false)
  const activeCategory = ref<ActiveCategory>('all')
  const selectedIndex = ref(0)
  const visible = ref(false)

  const indexStatus = ref<IndexStatus>('idle')
  const indexMessage = ref('初始化...')
  const indexStats = ref({ files: 0, apps: 0, commands: 0 })
  /** 索引进度扩展字段: 多盘符索引时可观察当前卷与总卷数. */
  const indexVolumesTotal = ref(0)
  const indexVolumeIndex = ref(0)
  const indexCurrentVolume = ref('')
  /** 应用索引是否已就绪 (后台 refresh_index 完成). 用于首屏自动重搜触发. */
  const appReady = ref(false)

  let debounceHandle: ReturnType<typeof setTimeout> | null = null

  const filteredResults = computed(() => {
    if (activeCategory.value === 'all') return results.value
    return results.value.filter((r) => r.category === activeCategory.value)
  })

  const topResults = computed(() => filteredResults.value.slice(0, 8))

  /**
   * 未搜索状态下"固定项目"分组: 优先取 launch_count 最高的前 4 个应用
   * (后端 app_search.search("") 已经按 launch_count 倒序排好, 这里取前 4 个)。
   * 若应用不足 4 个, 用最近访问的文件补齐。
   *
   * 行为规则:
   *   - category === 'all'        → 优先 apps, 不足用 files 补
   *   - category === 'apps'       → 仅 apps
   *   - category === 'files'      → 仅 files
   *   - category === 'commands'   → 空 (commands 分类无 pinned 概念)
   *
   * 使用 `filteredResults` 而非 `results` 让 pinned 自动跟随 category 过滤,
   * 避免在"文件" tab 下还显示应用这种割裂体验.
   */
  const pinned = computed<SearchResult[]>(() => {
    if (activeCategory.value === 'commands') return []
    const base = activeCategory.value === 'all' ? results.value : filteredResults.value

    if (activeCategory.value === 'files') {
      // 文件 tab: 直接取最近文件
      return base.filter((r) => r.category === 'files').slice(0, 4)
    }

    // 'all' 或 'apps' tab: 优先 apps
    const apps = base.filter((r) => r.category === 'apps').slice(0, 4)
    if (apps.length >= 4) return apps
    // 补足到 4 个: 填入文件
    const files = base.filter((r) => r.category === 'files').slice(0, 4 - apps.length)
    return [...apps, ...files]
  })

  /**
   * 未搜索状态下的"最近访问"分组.
   *   - 'all'        → 剩余应用 + 文件, 总共 10 个
   *   - 'apps'       → 剩余应用
   *   - 'files'      → 剩余文件
   *   - 'commands'   → 空
   */
  const recent = computed<SearchResult[]>(() => {
    if (activeCategory.value === 'commands') return []
    const usedIds = new Set(pinned.value.map((r) => r.id))
    const base = activeCategory.value === 'all' ? results.value : filteredResults.value
    return base.filter((r) => !usedIds.has(r.id)).slice(0, 10)
  })

  function setQuery(next: string) {
    query.value = next
    selectedIndex.value = 0
    if (debounceHandle) clearTimeout(debounceHandle)
    debounceHandle = setTimeout(() => runSearch(), DEBOUNCE_MS)
  }

  async function runSearch(options?: Partial<SearchOptions>) {
    loading.value = true
    try {
      results.value = await searchApi.search(query.value, options)
    } catch {
      results.value = []
    } finally {
      loading.value = false
    }
  }

  /**
   * 启动那一刻就把 UI 填满: 立即触发一次空查询,
   * 让后端的 `recent_files/list` 走完,前端立刻展示最近文件 / 应用.
   * `await` 在调用方处理;不阻塞 UI 渲染。
   */
  async function initialLoad() {
    await runSearch()
  }

  async function buildIndex() {
    if (indexStatus.value === 'building') return
    indexStatus.value = 'building'
    indexMessage.value = '正在构建索引...'
    try {
      await searchApi.buildIndex()
    } catch {
      indexStatus.value = 'error'
      indexMessage.value = '索引构建失败'
    }
  }

  /** 轮询拉取索引状态,用于初始 UI 已就绪但后台还在构建的场景. */
  async function loadIndexStatus() {
    try {
      const stats = await searchApi.getIndexStatus()
      indexStats.value = stats
      if (stats.files > 0) {
        indexStatus.value = 'completed'
        indexMessage.value = `已索引 ${stats.files.toLocaleString()} 个文件`
      }
    } catch {
      /* 静默 */
    }
  }

  function setIndexProgress(progress: {
    status: string
    message?: string
    files?: number
    volumes?: number
    current_volume?: string
    current_index?: number
    phase?: string
    apps?: number
  }) {
    const phase = progress.phase
    // 应用索引阶段: 更新 appReady; 仅当文件索引未在 building 时才动 indexStatus,
    // 避免应用 completed 覆盖文件索引的 building 进度 (文件索引是长任务, 优先级更高)。
    if (phase === 'apps') {
      if (progress.status === 'completed') {
        appReady.value = true
        if (typeof progress.apps === 'number') {
          indexStats.value.apps = progress.apps
        }
      }
      if (indexStatus.value !== 'building') {
        indexStatus.value = progress.status as IndexStatus
        indexMessage.value = progress.message || ''
      }
      return
    }
    // 文件索引阶段 (phase === 'files' 或缺省): 原逻辑
    switch (progress.status) {
      case 'building':
        indexStatus.value = 'building'
        indexMessage.value = progress.message || '正在构建索引...'
        if (typeof progress.files === 'number') {
          indexStats.value.files = progress.files
        }
        if (typeof progress.volumes === 'number') {
          indexVolumesTotal.value = progress.volumes
        }
        if (typeof progress.current_index === 'number') {
          indexVolumeIndex.value = progress.current_index
        }
        if (typeof progress.current_volume === 'string') {
          indexCurrentVolume.value = progress.current_volume
        }
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
    // 只在还没有结果时清空 (例如冷启动). 已经有数据时保持结果,
    // 让用户再次唤起窗口时立即看到熟悉的推荐列表, 无需等待 IPC.
    if (results.value.length === 0) {
      query.value = ''
      selectedIndex.value = 0
    } else {
      // 已有数据时仅重置选区到第一项, 不动 query / results.
      selectedIndex.value = 0
    }
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
    indexVolumesTotal,
    indexVolumeIndex,
    indexCurrentVolume,
    appReady,
    filteredResults,
    topResults,
    pinned,
    recent,
    setQuery,
    runSearch,
    initialLoad,
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
