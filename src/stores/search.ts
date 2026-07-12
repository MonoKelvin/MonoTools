import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import type { SearchResult, SearchOptions } from '@/types/search'
import { searchApi } from '@/services/searchApi'
import { pinApi } from '@/services/api'
import { SEARCH_DEBOUNCE_MS, SEARCH_LIMITS, SEARCH_LIMITS_VISIBLE } from '@/config'
import { getFileKind } from '@/utils/fileKinds'

export type ActiveCategory = 'all' | 'apps' | 'files' | 'commands'
export type IndexStatus = 'idle' | 'building' | 'completed' | 'error'

/**
 * 输入防抖: 30ms. 既消除每键击穿的连发 IPC, 又几乎无感.
 * 0ms 时用户连打 "chrome" 6 个字符 = 6 次 IPC, 在弱机上会卡顿;
 * 30ms 是 "字符级" 体验的甜蜜点 (低于人眼可感知延迟).
 *
 * 常量值来自 `src/config/search.ts::SEARCH_DEBOUNCE_MS`, 集中管理.
 */
const DEBOUNCE_MS = SEARCH_DEBOUNCE_MS

/** 固定项目最多展示多少个 (避免分组过高). */
const PINNED_MAX = SEARCH_LIMITS_VISIBLE.pinnedMax
/** 最近访问展示多少个. */
const RECENT_MAX = SEARCH_LIMITS_VISIBLE.recentMax

// ============================================================================
// 分组 (group) 标识 —— 单一真源. VGR 和 store 共享同一组 ID, 确保
// 折叠状态与可见列表能保持一致.
// ============================================================================

export const GROUP_ID = {
  pinned: 'group.pinned',
  recent: 'group.recent',
  system: 'group.system',
  commands: 'group.commands',
  apps: 'group.apps',
  files: 'group.files',
} as const

export type GroupId = (typeof GROUP_ID)[keyof typeof GROUP_ID]

/** 分组显示结构: 给 VGR 用来渲染 section. */
export interface DisplayGroup {
  id: GroupId
  title: string
  /** 折叠时, items 仍然按"已过滤+排序"准备, VGR 决定渲染哪些. */
  items: SearchResult[]
  /** 该组是否被折叠 (来自 store.collapsedGroups). */
  collapsed: boolean
  /** 命中时, 真正要显示的 items (未折叠 = 全部, 已折叠 = []). */
  visibleItems: SearchResult[]
  /** 该组类型, VGR 据此选用图标与色板. */
  kind: 'pinned' | 'recent' | 'system' | 'commands' | 'apps' | 'files'
  /** 未搜索时, "所有文件" 分组的可见项上限 (UI 性能保护). */
  fileVisibleLimit?: number
  /** 未搜索时, "所有文件" 分组总命中数 (用于"显示更多 (+N)"). */
  hiddenCount?: number
}

export const useSearchStore = defineStore('search', () => {
  const query = ref('')
  const results = ref<SearchResult[]>([])
  const loading = ref(false)
  const activeCategory = ref<ActiveCategory>('all')
  const selectedIndex = ref(0)
  /**
   * 选中项的全局 ID 锚点 —— 搜索 / 折叠 / 分类变化时,
   * 用 ID 而不是 index 来追踪"用户选中的是哪个"，
   * 避免结果列表重排后 selectedIndex 指向错误位置.
   *
   * 规则:
   * - 单击 / 上下方向键 / Enter → 同步更新 selectedIndex 与 selectedGlobalId
   * - displayList 变化 → 在 watcher 中按 ID 重新解析 selectedIndex
   * - 找不到 ID 时 → 落到 displayList 末尾, 并清空 selectedGlobalId
   */
  const selectedGlobalId = ref<string | null>(null)
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

  /**
   * 用户手动固定的 SearchResult.id 列表 (按用户添加顺序).
   * 不在 results 里出现时, 渲染会自动过滤掉 (避免 stale 引用).
   *
   * 持久化: 启动时调用 loadPinned() 从后端 SQLite 拉取, 任何变更
   * 立即调用 pinApi.add/remove 同步. 永远不会回退到 launch_count 模拟.
   */
  const pinnedIds = ref<string[]>([])

  /**
   * 用户已折叠的分组 ID 集合. 默认全部展开. VGR 通过 props 读取,
   * store 是状态的唯一所有者, 避免 view 层和 store 状态漂移.
   *
   * 重要: 当折叠状态变化时, 必须立即 clamp selectedIndex 防止指向
   * 不可见项造成 UI 无响应.
   */
  const collapsedGroups = ref<Set<GroupId>>(new Set())

  let debounceHandle: ReturnType<typeof setTimeout> | null = null

  const filteredResults = computed(() => {
    if (activeCategory.value === 'all') return results.value
    return results.value.filter((r) => r.category === activeCategory.value)
  })

  /**
   * "固定项目" 分组: 仅含用户**手动** pin 的项. 不再从 launch_count 推算.
   *   - 空列表时 → 分组区域不显示 (VGR 已实现).
   *   - 引用已不在 results 中的 id → 自动跳过, 避免显示陈旧数据.
   *   - 上限 PINNED_MAX = 8.
   */
  const pinned = computed<SearchResult[]>(() => {
    if (pinnedIds.value.length === 0) return []
    const map = new Map(results.value.map((r) => [r.id, r]))
    return pinnedIds.value
      .map((id) => map.get(id))
      .filter((r): r is SearchResult => !!r)
      .slice(0, PINNED_MAX)
  })

  /**
   * "最近访问" 分组: 排除 pinned 后的剩余 results, 保持 launch_count 顺序.
   * 这是"次重要"列表, 给用户**主动 pin 之前**浏览过/用过的应用和文件做临时推荐.
   * 同样: 空列表时分组区域不显示.
   */
  const recent = computed<SearchResult[]>(() => {
    if (activeCategory.value === 'commands') return []
    const pinnedSet = new Set(pinnedIds.value)
    return results.value
      .filter((r) => !pinnedSet.has(r.id))
      .slice(0, RECENT_MAX)
  })

  /** 某个 id 是否被 pin. */
  function isPinned(id: string): boolean {
    return pinnedIds.value.includes(id)
  }

  /**
   * 切换 pin 状态: 立即更新本地, 后端持久化 (失败则回滚).
   * 调用方无需 await, 但 UI 已经乐观更新.
   */
  async function togglePin(id: string): Promise<void> {
    const idx = pinnedIds.value.indexOf(id)
    const wasPinned = idx >= 0
    // 乐观更新
    if (wasPinned) {
      pinnedIds.value = pinnedIds.value.filter((x) => x !== id)
    } else {
      pinnedIds.value = [id, ...pinnedIds.value].slice(0, PINNED_MAX)
    }
    try {
      if (wasPinned) await pinApi.remove(id)
      else await pinApi.add(id)
    } catch {
      // 回滚
      if (wasPinned) {
        pinnedIds.value = [id, ...pinnedIds.value].slice(0, PINNED_MAX)
      } else {
        pinnedIds.value = pinnedIds.value.filter((x) => x !== id)
      }
    }
  }

  /** 从后端加载 pinned 列表. 应在 initialLoad 之后调用. */
  async function loadPinned(): Promise<void> {
    try {
      const ids = await pinApi.list()
      pinnedIds.value = Array.isArray(ids) ? ids : []
    } catch {
      pinnedIds.value = []
    }
  }

  function setQuery(next: string) {
    query.value = next
    selectedIndex.value = 0
    // 清空 ID 锚点: 搜索新内容时, 之前选中的项可能不再相关.
    selectedGlobalId.value = null
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

  /** 文件类型过滤: 由 VGR 通过 `setFileKindFilter` 控制. */
  const fileKindFilter = ref<Set<string>>(new Set())

  /**
   * 通知 store 前端文件类型过滤状态, 让 `displayList` 与 VGR 渲染严格一致.
   * 当 VGR 内部 `selectedFileKinds` 变化时调用.
   */
  function setFileKindFilter(kinds: Set<string>) {
    fileKindFilter.value = kinds
  }

  // ==========================================================================
  // Display pipeline: query / pinned / collapsed 三者共同决定"实际可见"列表
  // ==========================================================================

  /**
   * 进入 VGR 的"原始"分组数据. 每个组含该类的全部命中, 不受折叠影响.
   * VGR 只需决定渲染哪些 (根据 collapsed), 不用再算 groups.
   *
   * 注: 文件分组在未搜索状态下做了 `fileVisibleLimit` 截断, 避免 500+
   * 个 DOM 节点同屏 paint. hiddenCount 用来展示"显示更多 (+N)".
   */
  const fileVisibleLimit = ref<number>(SEARCH_LIMITS_VISIBLE.fileVisibleInitial)

  /**
   * 对 files 分组应用文件类型过滤: 仅保留命中 selectedFileKinds 的项.
   * 未搜索时过滤生效; 搜索时 (query 非空) 也过滤 (保持用户偏好).
   */
  function applyFileKindFilter(items: SearchResult[], kinds: Set<string>): SearchResult[] {
    if (kinds.size === 0) return items
    // 统一调用 fileKinds.getFileKind, 避免在此处重复定义 typeMap / ext Set.
    return items.filter((r) => kinds.has(getFileKind(r)))
  }

  const allAppsSorted = computed<SearchResult[]>(() => {
    return filteredResults.value
      .filter((r) => r.category === 'apps' && r.resultType !== 'system-app')
      .sort((a, b) => a.title.localeCompare(b.title))
  })

  const systemAppsSorted = computed<SearchResult[]>(() => {
    return filteredResults.value
      .filter((r) => r.category === 'apps' && r.resultType === 'system-app')
      .sort((a, b) => a.title.localeCompare(b.title))
  })

  const commandsItems = computed<SearchResult[]>(() => {
    return filteredResults.value
      .filter((r) => r.category === 'commands')
      .slice(0, SEARCH_LIMITS_VISIBLE.commandsMax)
  })

  const filesAllUnfiltered = computed<SearchResult[]>(() => {
    return filteredResults.value
      .filter((r) => r.category === 'files')
      .slice(0, query.value ? SEARCH_LIMITS.realtime : SEARCH_LIMITS.emptyQuery)
  })

  const allAppsItems = computed<SearchResult[]>(() => {
    // 搜索时仍然按全量返回; 未搜索时 = 全量 (因为后端 ALL_FILES_EMPTY_QUERY_CAP
    // 已限制 2000, 这里不再二次截断).
    if (query.value) return allAppsSorted.value
    return allAppsSorted.value
  })

  const systemAppsItems = computed<SearchResult[]>(() => {
    if (query.value) return systemAppsSorted.value
    return systemAppsSorted.value
  })

  /**
   * 6 个分组的完整数据 —— 单一真源, VGR 不再自己算分组.
   * 关键: query 模式下, pinned/recent 分组为空 (不显示).
   */
  const displayGroups = computed<DisplayGroup[]>(() => {
    const out: DisplayGroup[] = []
    const q = query.value
    const isCollapsed = (id: GroupId) => collapsedGroups.value.has(id)

    // 1) 固定项目 (未搜索才显示)
    const pinnedItems = q ? [] : pinned.value
    out.push({
      id: GROUP_ID.pinned,
      title: '固定项目',
      items: pinnedItems,
      visibleItems: isCollapsed(GROUP_ID.pinned) ? [] : pinnedItems,
      collapsed: isCollapsed(GROUP_ID.pinned),
      kind: 'pinned',
    })

    // 2) 最近访问 (未搜索才显示)
    const recentItems = q ? [] : recent.value
    out.push({
      id: GROUP_ID.recent,
      title: '最近访问',
      items: recentItems,
      visibleItems: isCollapsed(GROUP_ID.recent) ? [] : recentItems,
      collapsed: isCollapsed(GROUP_ID.recent),
      kind: 'recent',
    })

    // 3) 系统应用
    const sysItems = systemAppsItems.value
    out.push({
      id: GROUP_ID.system,
      title: '系统应用',
      items: sysItems,
      visibleItems: isCollapsed(GROUP_ID.system) ? [] : sysItems,
      collapsed: isCollapsed(GROUP_ID.system),
      kind: 'system',
    })

    // 4) 命令
    const cmdItems = commandsItems.value
    out.push({
      id: GROUP_ID.commands,
      title: '命令',
      items: cmdItems,
      visibleItems: isCollapsed(GROUP_ID.commands) ? [] : cmdItems,
      collapsed: isCollapsed(GROUP_ID.commands),
      kind: 'commands',
    })

    // 5) 所有应用
    const appsItems = allAppsItems.value
    out.push({
      id: GROUP_ID.apps,
      title: '所有应用',
      items: appsItems,
      visibleItems: isCollapsed(GROUP_ID.apps) ? [] : appsItems,
      collapsed: isCollapsed(GROUP_ID.apps),
      kind: 'apps',
    })

    // 6) 所有文件 - 受 fileVisibleLimit 控制 + 折叠影响 + 文件类型过滤
    const allFiles = filesAllUnfiltered.value
    let filesItems: SearchResult[]
    let hiddenCount: number
    if (q) {
      filesItems = applyFileKindFilter(allFiles, fileKindFilter.value)
      hiddenCount = 0
    } else {
      const limit = Math.min(fileVisibleLimit.value, SEARCH_LIMITS_VISIBLE.fileVisibleHardCap)
      filesItems = applyFileKindFilter(allFiles.slice(0, limit), fileKindFilter.value)
      hiddenCount = Math.max(0, allFiles.length - limit)
    }
    out.push({
      id: GROUP_ID.files,
      title: '所有文件',
      items: allFiles,
      visibleItems: isCollapsed(GROUP_ID.files) ? [] : filesItems,
      collapsed: isCollapsed(GROUP_ID.files),
      kind: 'files',
      fileVisibleLimit: fileVisibleLimit.value,
      hiddenCount,
    })

    return out
  })

  /**
   * 实际可见的扁平列表. 用于键盘上下方向键 + Enter 选中.
   * 与 VGR 渲染的内容严格 1:1, 杜绝 selectedIndex 指向不可见项.
   */
  const displayList = computed<SearchResult[]>(() => {
    const out: SearchResult[] = []
    for (const g of displayGroups.value) {
      if (!g.collapsed) {
        for (const it of g.visibleItems) out.push(it)
      }
    }
    return out
  })

  /** keyboard nav / Enter 选中的下标上限. */
  const displayMax = computed(() => Math.max(0, displayList.value.length))

  /**
   * 切换一个分组的折叠状态. 当折叠时若 selectedIndex 指向被隐藏的项,
   * 主动 clamp 到 0 / 末尾, 避免高亮看不见.
   */
  function toggleGroupCollapse(id: GroupId) {
    const next = new Set(collapsedGroups.value)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    collapsedGroups.value = next
  }

  /**
   * "所有文件" 分组增量展开: 每次 +50 个 (硬上限由 SEARCH_LIMITS_VISIBLE.fileVisibleHardCap 控制).
   */
  function showMoreFiles() {
    fileVisibleLimit.value = Math.min(
      fileVisibleLimit.value + SEARCH_LIMITS_VISIBLE.fileVisibleStep,
      SEARCH_LIMITS_VISIBLE.fileVisibleHardCap,
    )
  }

  function selectNext() {
    if (selectedIndex.value < displayMax.value - 1) {
      selectedIndex.value++
      // 同步 ID 锚点: 切换时记录当前选中项的 ID, 用于 displayList 变化后重新定位.
      const item = displayList.value[selectedIndex.value]
      selectedGlobalId.value = item?.id ?? null
    }
  }

  function selectPrev() {
    if (selectedIndex.value > 0) {
      selectedIndex.value--
      const item = displayList.value[selectedIndex.value]
      selectedGlobalId.value = item?.id ?? null
    }
  }

  /**
   * 直接按 index 选中, 同时设置 ID 锚点. 给单击 / 双击 / hover 场景用.
   * 边界保护: 越界时 clamp 到 [0, max-1].
   */
  function selectByIndex(idx: number) {
    if (displayMax.value === 0) {
      selectedIndex.value = 0
      selectedGlobalId.value = null
      return
    }
    const clamped = Math.max(0, Math.min(idx, displayMax.value - 1))
    selectedIndex.value = clamped
    selectedGlobalId.value = displayList.value[clamped]?.id ?? null
  }

  /**
   * 当 displayList 变化时 (搜索 / 索引刷新 / 折叠切换 / 分类筛选), 主动
   * 重新定位 selectedIndex:
   * 1) 如果有 ID 锚点 → 在新 displayList 中按 ID 查找, 找到了就定位过去;
   * 2) 找不到 (被折叠 / 被过滤 / 列表为空) → 落到末尾, 清空 ID 锚点;
   * 3) 都没 ID 锚点时, 仅做边界 clamp.
   *
   * 关键改进: 之前 selectedIndex 可能停在越界位置, 视觉上看不到高亮 +
   * Enter 无反应. 现在即使 displayList 剧烈重排, 选中态也会"跟着 ID 走".
   *
   * `flush: 'sync'` 同步触发 (默认 'pre' 是 microtask).
   * 选 'sync' 的原因: displayList 变化常常在同一 tick 内被 selectByIndex 紧随读取,
   * 默认 'pre' 会在 selectByIndex 之后才跑, 留下 selectedIndex 越界的窗口.
   * 同步触发保证 clamp 立即生效, 行为对调用方完全可预测.
   */
  watch(
    displayMax,
    () => {
      if (displayMax.value === 0) {
        selectedIndex.value = 0
        selectedGlobalId.value = null
        return
      }
      // 优先按 ID 锚点重新定位
      if (selectedGlobalId.value) {
        const idx = displayList.value.findIndex((r) => r.id === selectedGlobalId.value)
        if (idx >= 0) {
          selectedIndex.value = idx
          return
        }
        // 锚点丢失: 落到末尾, 保留上一个 ID 作为"上一次选中"的记忆 (便于后续 expand 恢复)
        selectedIndex.value = displayMax.value - 1
        return
      }
      // 无锚点: 单纯 clamp 边界
      if (selectedIndex.value > displayMax.value - 1) {
        selectedIndex.value = displayMax.value - 1
      } else if (selectedIndex.value < 0) {
        selectedIndex.value = 0
      }
    },
    { flush: 'sync' },
  )

  async function executeSelected(): Promise<SearchResult | null> {
    const item = displayList.value[selectedIndex.value]
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
      selectedGlobalId.value = null
    } else {
      // 已有数据时仅重置选区到第一项, 不动 query / results.
      selectedIndex.value = 0
      // 用 displayList 第一项的 ID 作为新锚点, 避免 ID 残留造成重定位错误.
      selectedGlobalId.value = displayList.value[0]?.id ?? null
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
    /** 选中项的全局 ID 锚点 (供 VGR / 测试 / hover 等场景同步使用). */
    selectedGlobalId,
    visible,
    indexStatus,
    indexMessage,
    indexStats,
    indexVolumesTotal,
    indexVolumeIndex,
    indexCurrentVolume,
    appReady,
    pinnedIds,
    collapsedGroups,
    filteredResults,
    pinned,
    recent,
    isPinned,
    togglePin,
    loadPinned,
    setQuery,
    runSearch,
    initialLoad,
    buildIndex,
    loadIndexStatus,
    setIndexProgress,
    setCategory,
    selectNext,
    selectPrev,
    /** 按 index 选中, 自动同步 ID 锚点 + 边界 clamp. */
    selectByIndex,
    executeSelected,
    executeItem,
    show,
    hide,
    toggle,
    // 新增的显示层 API
    displayGroups,
    displayList,
    displayMax,
    toggleGroupCollapse,
    setFileKindFilter,
    showMoreFiles,
  }
})
