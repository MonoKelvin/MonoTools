import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import type { SearchResult, SearchOptions } from './types'
import { searchApi } from '@/services/api'
import { pinApi, pinTopApi, windowMonitorApi } from '@/services/api'
import { SEARCH_DEBOUNCE_MS, SEARCH_LIMITS_VISIBLE, SEARCH_LIMITS } from '@/core/config'
import { getFileKind } from './utils/fileKinds'
import type { SortMode } from '@/core/config/sorting'
import {
    SMART_WEIGHTS,
    APP_CATEGORIES,
    RECOMMENDATION_MAP,
    DEFAULT_SORT_BY_GROUP,
} from '@/core/config/sorting'
import { isTauri } from '@/services/env'

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
    const alwaysOnTop = ref(false)

    /**
     * 多选模式的当前激活分组 ID.
     * 每个分组的选中状态独立且互斥: 切换到新分组时清空旧分组的选中.
     */
    const activeSelectionGroupId = ref<string | null>(null)
    /**
     * 当前激活分组的选中索引集合 (本地索引, 即分组内的索引).
     * - 单击: 清空其他选中, 选中当前
     * - Ctrl+单击: 切换当前项目的选中状态
     * - Shift+单击: 选中从 lastSelectedLocalIndex 到当前的所有项目
     * - Ctrl+Shift+单击: 反选从 lastSelectedLocalIndex 到当前的所有项目
     */
    const selectedIndexes = ref<Set<number>>(new Set())
    /**
     * 上一次选中的本地索引, 用于 Shift 范围选择.
     */
    const lastSelectedLocalIndex = ref(0)

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
     * 当前激活窗口的进程路径. 由后端 WindowMonitorService 推送 `window_changed` 事件更新.
     * 切换前台应用 → 此值变化 → 所有依赖它的 computed 重算 → 推荐排序立刻变化.
     *
     * 空字符串代表"尚未收到任何信号"或"非 Windows 平台". 这种情况
     * activeAppCategories 会回退到最近列表推断.
     */
    const activeAppPath = ref<string>('')
    /** 当前激活窗口的应用标题, 供 UI 状态栏/调试使用. */
    const activeAppTitle = ref<string>('')
    /**
     * WindowMonitorService 记录的最近切换过的应用, 用于次级推荐信号.
     * 同时也是冷启动场景的兜底数据源.
     */
    const activeAppRecent = ref<Array<{ path: string; title: string }>>([])

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

    /**
     * 每个分组的排序模式. 默认值来自 DEFAULT_SORT_BY_GROUP.
     * 用户可通过 GroupSection 右侧的排序 combobox 修改.
     * key 使用 GROUP_ID 的短名 (不含 "group." 前缀).
     */
    const groupSortModes = ref<Record<GroupId, SortMode>>({
        [GROUP_ID.pinned]: DEFAULT_SORT_BY_GROUP.pinned,
        [GROUP_ID.recent]: DEFAULT_SORT_BY_GROUP.recent,
        [GROUP_ID.system]: DEFAULT_SORT_BY_GROUP.system,
        [GROUP_ID.commands]: DEFAULT_SORT_BY_GROUP.commands,
        [GROUP_ID.apps]: DEFAULT_SORT_BY_GROUP.apps,
        [GROUP_ID.files]: DEFAULT_SORT_BY_GROUP.files,
    })

    /**
     * 计算当前已打开应用的类别分布, 用于智能推荐.
     *
     * 核心改进:
     * - **Foreground-first**: 以前只依赖 recent (launch_count 排序后固化).
     *   现在核心信号是当前激活窗口 (`activeAppPath`), 切换前台应用立刻改变推荐.
     * - **Recent context**: 仅在 foreground 信号缺失时回退.
     * - **可观测**: 后端 emit `window_changed`, 前端 store 监听 → 触发重算.
     */
    const activeAppCategories = computed<Record<string, number>>(() => {
        const cats: Record<string, number> = {}

        // 1) 前台信号, 当前激活应用是首选
        const foreground = activeAppPath.value
        if (foreground) {
            const matched = matchAppCategoryByPath(foreground)
            if (matched) cats[matched] = (cats[matched] || 0) + 100 // 主导信号, 高权重
        }

        // 2) 次要信号: 最近窗口监控历史, 按访问顺序加权
        const recentAppsList = activeAppRecent.value
        recentAppsList.forEach((entry, idx) => {
            const cat = matchAppCategoryByPath(entry.path)
            if (!cat) return
            // 越新的窗口权重越高
            const weight = Math.max(1, recentAppsList.length - idx)
            cats[cat] = (cats[cat] || 0) + weight
        })

        // 3) 最后回退: 最近启动 (launch_count) 列表
        if (Object.keys(cats).length === 0) {
            const recentApps = recent.value.filter((r) => r.category === 'apps')
            recentApps.forEach((app, idx) => {
                const titleLower = app.title.toLowerCase()
                const weight = Math.max(1, recentApps.length - idx)
                for (const [cat, keywords] of Object.entries(APP_CATEGORIES)) {
                    if (keywords.some((kw) => titleLower.includes(kw.toLowerCase()))) {
                        cats[cat] = (cats[cat] || 0) + weight
                    }
                }
            })
        }

        return cats
    })

    /**
     * 预计算: 缓存 APP_CATEGORIES 的关键词匹配结果.
     * 避免每次排序时对每个 item 都遍历整个 APP_CATEGORIES 对象.
     * key = 路径/标题的 lowercase, value = 匹配的类别.
     */
    const categoryCache = new Map<string, string | null>()

    /**
     * 带缓存的类别匹配. 同一路径多次查询只遍历一次 APP_CATEGORIES.
     */
    function matchAppCategoryByPathCached(path: string): string | null {
        const key = path.toLowerCase()
        // 先查缓存
        if (categoryCache.has(key)) {
            return categoryCache.get(key) ?? null
        }
        // 未命中, 执行匹配并缓存
        const result = matchAppCategoryByPath(path)
        categoryCache.set(key, result)
        return result
    }

    /**
     * 根据用户已打开应用的类别, 计算推荐类别集合.
     * 返回: Map<resultId, bonusScore>
     *
     * 优化: 预构建"类别 → 关键词集合"的反向索引, 避免对每条结果遍历所有类别.
     */
    const recommendationScores = computed<Map<string, number>>(() => {
        const scores = new Map<string, number>()
        const activeCats = activeAppCategories.value
        if (Object.keys(activeCats).length === 0) return scores

        // 找出用户活跃的主要类别 (计数 > 0)
        const userCats = Object.keys(activeCats).filter((c) => activeCats[c] > 0)

        // 收集需要推荐的类别 (去重)
        const targetCats = new Set<string>()
        for (const cat of userCats) {
            const recs = RECOMMENDATION_MAP[cat] || []
            for (const r of recs) targetCats.add(r)
        }
        // 移除用户已经在大量使用的类别 (不推荐用户已经在用的)
        for (const cat of userCats) {
            if (activeCats[cat] >= 30) targetCats.delete(cat)
        }

        if (targetCats.size === 0) return scores

        // 预构建: 目标类别 → 关键词集合 (小写)
        const targetKeywords = new Map<string, Set<string>>()
        for (const cat of targetCats) {
            const keywords = APP_CATEGORIES[cat] || []
            const kwSet = new Set(keywords.map((kw) => kw.toLowerCase()))
            targetKeywords.set(cat, kwSet)
        }

        const recBonus = SMART_WEIGHTS.recommendation

        // 单次遍历 results, 用缓存的类别匹配
        for (const app of results.value) {
            if (app.category !== 'apps') continue
            const id = app.title.toLowerCase() + '|' + (app.subtitle || app.title).toLowerCase()

            // 用缓存的类别匹配
            const appCat = matchAppCategoryByPathCached(app.subtitle || app.title)
            if (appCat && targetCats.has(appCat)) {
                scores.set(app.id, (scores.get(app.id) || 0) + recBonus)
            }
        }

        return scores
    })

    /**
     * 智能排序打分.
     * 综合: 访问次数 + 名称匹配 + 目录访问 + 推荐权重 + **前台上下文加权**.
     *
     * 设计:
     * - **Foreground bonus**: 与当前激活应用属于相同大类的应用获得显著加分.
     *   这是"切换激活应用后排序立刻变化"的关键信号.
     * - 用户当前激活 dev → IDE、终端、浏览器加分; 切换到 communication →
     *   slack/teams/discord 立即兑现.
     */
    function smartSortScore(item: SearchResult, queryStr: string): number {
        let score = 0
        const w = SMART_WEIGHTS

        // 1) 访问次数 (对数缩放避免高频应用垄断)
        const countFactor = Math.log((item.score || 0) + 1)
        score += countFactor * w.launchCount

        // 2) 名称匹配 (位置敏感, 前缀 > 子串)
        if (queryStr) {
            const q = queryStr.toLowerCase()
            const title = item.title.toLowerCase()
            const pos = title.indexOf(q)
            if (pos === 0) {
                score += w.nameMatch * 3
            } else if (pos > 0) {
                const posBonus = pos <= 3 ? 2 : 1
                score += w.nameMatch * posBonus
            }
        }

        // 3) 推荐加分
        const recScore = recommendationScores.value.get(item.id)
        if (recScore) score += recScore

        // 4) **前台上下文加权**: 当前激活应用所属类别直接加分, 让排序"立刻可见"
        //    切换前台应用 → 此项立即变化 → 排序立即重排.
        const fgBoost = getForegroundCategoryBoost(item)
        if (fgBoost) score += fgBoost

        // 5) 路径 catBoost 配套: 即便搜索未命中, 前台大类的应用依然靠前
        return score
    }

    /**
     * 当前激活应用所属的类别. 若路径含已知类别关键字, 返回该类别; 否则 null.
     *
     * 使用 pathLower (subtitle/exe 路径), 比 title 更稳定 (标题包含文件名时变化).
     */
    function matchAppCategoryByPath(path: string): string | null {
        const lower = path.toLowerCase()
        let best: { cat: string; len: number } | null = null
        for (const [cat, keywords] of Object.entries(APP_CATEGORIES)) {
            for (const kw of keywords) {
                const k = kw.toLowerCase()
                if (!k) continue
                if (lower.includes(k)) {
                    // 取最长关键字作为"更精确"的类别信号
                    if (!best || k.length > best.len) {
                        best = { cat, len: k.length }
                    }
                }
            }
        }
        return best?.cat ?? null
    }

    /**
     * 前台类别加权: 与当前激活应用同类别的应用获得显著加分.
     * 切前台应用时, 此函数返回值立即变化 → smartSortScore 重排.
     *
     * 注意: 这里用普通函数 + activeAppPath.value, 不用 computed,
     * 因为 computed 返回函数时, 调用方必须 .value() 才能执行, 容易出错.
     */
    function getForegroundCategoryBoost(item: SearchResult): number {
        const fgPath = activeAppPath.value
        if (!fgPath) return 0
        const fgCat = matchAppCategoryByPathCached(fgPath)
        if (!fgCat) return 0
        const itemCat = matchAppCategoryByPathCached(item.subtitle || item.title)
        if (itemCat === fgCat) return SMART_WEIGHTS.launchCount * 5
        // 弱推荐类别也轻微加分, 避免排序完全二元
        const recs = RECOMMENDATION_MAP[fgCat] || []
        if (recs.includes(itemCat || '')) return SMART_WEIGHTS.launchCount * 1.5
        return 0
    }

    /** 按指定模式排序 items */
    function sortItems(items: SearchResult[], mode: SortMode, groupId: string): SearchResult[] {
        switch (mode) {
            case 'name':
                return [...items].sort((a, b) => a.title.localeCompare(b.title))
            case 'recent':
                // 按 score (launch_count) 降序
                return [...items].sort((a, b) => (b.score || 0) - (a.score || 0))
            case 'path':
                return [...items].sort((a, b) =>
                    (a.subtitle || '').localeCompare(b.subtitle || ''),
                )
            case 'smart':
            default:
                return [...items].sort((a, b) =>
                    smartSortScore(b, query.value) - smartSortScore(a, query.value),
                )
        }
    }

    /** 设置分组排序模式 */
    function setGroupSortMode(groupId: GroupId, mode: SortMode) {
        groupSortModes.value[groupId] = mode
    }

    let debounceHandle: ReturnType<typeof setTimeout> | null = null
    /** 增量搜索防抖: 防止索引过程中频繁重搜导致 UI 卡顿 */
    let incrementalRefreshTimer: ReturnType<typeof setTimeout> | null = null
    /** 增量刷新间隔（毫秒）: 索引过程中每 200ms 最多刷新一次列表 */
    const INCREMENTAL_REFRESH_INTERVAL = 200
    /** 搜索请求序号，用于竞态处理，确保旧请求不会覆盖新结果 */
    let searchRequestSeq = 0

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
        const items = pinnedIds.value
            .map((id) => map.get(id))
            .filter((r): r is SearchResult => !!r)
            .slice(0, PINNED_MAX)
        return sortItems(items, groupSortModes.value[GROUP_ID.pinned], GROUP_ID.pinned)
    })

    /**
     * "最近访问" 分组: 按 launch_count 排序的前 N 个结果.
     * 与 pinned 不互斥 —— 同一项目可同时出现在固定和最近中, 各自独立.
     * 空列表时分组区域不显示.
     */
    const recent = computed<SearchResult[]>(() => {
        if (activeCategory.value === 'commands') return []
        const items = results.value.slice(0, RECENT_MAX)
        return sortItems(items, groupSortModes.value[GROUP_ID.recent], GROUP_ID.recent)
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
        const currentSeq = ++searchRequestSeq
        loading.value = true
        try {
            const effectiveLimit = query.value === ''
                ? SEARCH_LIMITS.emptyQueryLimit
                : SEARCH_LIMITS.defaultLimit
            const mergedOptions: Partial<SearchOptions> = {
                limit: effectiveLimit,
                ...options,
            }
            const newResults = await searchApi.search(query.value, mergedOptions)
            // 竞态检查: 只在当前请求是最新的才更新结果
            if (currentSeq === searchRequestSeq) {
                results.value = newResults
            }
        } catch {
            if (currentSeq === searchRequestSeq) {
                results.value = []
            }
        } finally {
            if (currentSeq === searchRequestSeq) {
                loading.value = false
            }
        }
    }

    /**
     * 启动那一刻就把 UI 填满: 立即触发一次空查询,
     * 让后端的 `recent_files/list` 走完,前端立刻展示最近文件 / 应用.
     * `await` 在调用方处理;不阻塞 UI 渲染。
     *
     * 同时: 启动后端 WindowMonitor 监听器 (有 `window_changed` 事件流),
     * 并拉取当前快照, 冷启动也能拿到当前激活应用上下文.
     */
    async function initialLoad() {
        // 不阻塞搜索, 并行拉监控状态
        void syncWindowMonitor().catch(() => undefined)
        await runSearch()
    }

    /**
     * 同步后端 WindowMonitor 当前状态 (进程路径 + 标题 + 最近历史).
     * 在冷启动 / 已经存在但需要拿到首屏数据时调用, 秒级完成.
     */
    async function syncWindowMonitor(): Promise<void> {
        if (!isTauri) return
        try {
            const snap = await windowMonitorApi.getState()
            if (snap.activeAppPath) {
                activeAppPath.value = snap.activeAppPath
                activeAppTitle.value = snap.activeAppTitle
            }
            if (Array.isArray(snap.recentApps)) {
                activeAppRecent.value = snap.recentApps
            }
            logWindowMonitorSync()
        } catch {
            // ignore
        }
    }

    function logWindowMonitorSync() {
        logRecommendSignal(
            `foreground='${activeAppTitle.value || '?'}' (${activeAppPath.value}) recent=${activeAppRecent.value.length}`,
        )
    }

    /** 内部日志, 内联避免新增 logger 模块. */
    function logRecommendSignal(msg: string): void {
        // 控制台统一加 [recommendation] 前缀, 便于按 tag grep 关闭.
        // eslint-disable-next-line no-console
        console.info('[recommendation]', msg)
    }

    /**
     * 应用 `window_changed` 事件 payload, 更新本地 ref 触发重算.
     * 暴露给业务侧手动调用; 也可被 `listenWindowMonitor` 内部使用.
     */
    function applyWindowChanged(payload: {
        path?: string
        title?: string
        recent_count?: number
    }): void {
        if (typeof payload.path === 'string') {
            if (activeAppPath.value !== payload.path) {
                activeAppPath.value = payload.path
            }
        }
        if (typeof payload.title === 'string') {
            activeAppTitle.value = payload.title
        }
        // 后端只推 recent_count, 计数变化时拉一次最新快照拿完整路径列表
        const targetCount = payload.recent_count ?? 0
        if (activeAppRecent.value.length !== targetCount) {
            void syncWindowMonitor().catch(() => undefined)
        }
        logWindowMonitorSync()
    }

    /**
     * 订阅后端 `window_changed` 事件, 持续更新推荐上下文.
     * 返回 unlisten, 调用方应在组件卸载时清理.
     */
    async function listenWindowMonitor(): Promise<() => void> {
        if (!isTauri) return () => { }
        try {
            return await windowMonitorApi.listenChanged(applyWindowChanged)
        } catch {
            return () => { }
        }
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
            if (typeof progress.apps === 'number') {
                indexStats.value.apps = progress.apps
            }
            if (indexStatus.value !== 'building') {
                indexStatus.value = progress.status as IndexStatus
                indexMessage.value = progress.message || ''
            }
            // 增量刷新: 每收到应用索引进度就刷新一次列表（带防抖）
            triggerIncrementalRefresh()
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
                // 增量刷新: 每收到文件索引进度就刷新一次列表（带防抖）
                triggerIncrementalRefresh()
                break
            case 'completed':
                indexStatus.value = 'completed'
                if (progress.files) {
                    indexStats.value.files = progress.files
                    indexMessage.value = `索引完成，共 ${progress.files.toLocaleString()} 个文件`
                } else {
                    indexMessage.value = '索引完成'
                }
                // 完成后再刷新一次, 确保拿到最终结果
                triggerIncrementalRefresh()
                break
            case 'error':
                indexStatus.value = 'error'
                indexMessage.value = progress.message || '索引构建失败'
                break
            default:
                break
        }
    }

    /**
     * 触发增量刷新。带 200ms 防抖，避免索引过程中频繁重搜。
     * 仅在空查询时刷新，因为有查询词的搜索依赖 FTS5，FTS5 要最后才建好。
     */
    function triggerIncrementalRefresh() {
        // 有查询词时不刷新: FTS5 索引最后才重建, 此时刷新意义不大
        if (query.value !== '') return
        if (incrementalRefreshTimer) {
            clearTimeout(incrementalRefreshTimer)
        }
        incrementalRefreshTimer = setTimeout(() => {
            runSearch().catch(() => undefined)
        }, INCREMENTAL_REFRESH_INTERVAL)
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
     * 对 files 分组应用文件类型过滤: 仅保留命中 selectedFileKinds 的项.
     * 未搜索时过滤生效; 搜索时 (query 非空) 也过滤 (保持用户偏好).
     */
    function applyFileKindFilter(items: SearchResult[], kinds: Set<string>): SearchResult[] {
        if (kinds.size === 0) return items
        // 统一调用 fileKinds.getFileKind, 避免在此处重复定义 typeMap / ext Set.
        return items.filter((r) => kinds.has(getFileKind(r)))
    }

    const allAppsSorted = computed<SearchResult[]>(() => {
        const items = filteredResults.value.filter((r) => r.category === 'apps')
        return sortItems(items, groupSortModes.value[GROUP_ID.apps], GROUP_ID.apps)
    })

    const systemAppsSorted = computed<SearchResult[]>(() => {
        const items = filteredResults.value
            .filter((r) => r.category === 'apps' && r.resultType === 'system-app')
        return sortItems(items, groupSortModes.value[GROUP_ID.system], GROUP_ID.system)
    })

    const commandsItems = computed<SearchResult[]>(() => {
        return filteredResults.value
            .filter((r) => r.category === 'commands')
            .slice(0, SEARCH_LIMITS_VISIBLE.commandsMax)
    })

    // 不再截断: 后端返回全量, vue-virtual-scroller 负责渲染.
    const filesAllUnfiltered = computed<SearchResult[]>(() => {
        return filteredResults.value.filter((r) => r.category === 'files')
    })

    // allAppsSorted 已在外部定义, 无需额外 computed 包装.
    // 直接使用 allAppsSorted.value 即可.

    // systemAppsSorted 已在外部定义, 无需额外 computed 包装.
    // 直接使用 systemAppsSorted.value 即可.

    /**
     * 6 个分组的完整数据 —— 单一真源, VGR 不再自己算分组.
     * 关键: query 模式下, pinned/recent 分组为空 (不显示).
     *
     * 折叠规则 (2026-07):
     * - 空组 (items.length === 0): 不渲染. 由 VGR.virtualRows 直接 `continue`
     *   跳过, 既不显示 header 也不显示 items. 既然没有内容, 折叠展开与分割线
     *   都没有视觉意义. 默认 = "收起来的" (完全不显示), 达成产品诉求.
     * - 有内容的组: 跟随 user-collapsed 集合, 默认 false (展开).
     *   展开后 selectedIndex 落在该组首项, 上下方向键可导航.
     *
     * 注意: 即使用户之前手动折叠过一个组, 当该组清空时, 我们也无需清理
     * `collapsedGroups` 集合. 下次该组重新有内容时, 用户仍能保持"折叠"
     * 偏好, 不会突然跳出来. 这是 sticky 折叠偏好的小细节.
     */
    const displayGroups = computed<DisplayGroup[]>(() => {
        const out: DisplayGroup[] = []
        const q = query.value
        const isUserCollapsed = (id: GroupId) => collapsedGroups.value.has(id)

        // 1) 固定项目 (未搜索才显示)
        const pinnedItems = q ? [] : pinned.value
        out.push({
            id: GROUP_ID.pinned,
            title: '固定项目',
            items: pinnedItems,
            visibleItems: isUserCollapsed(GROUP_ID.pinned) ? [] : pinnedItems,
            collapsed: isUserCollapsed(GROUP_ID.pinned),
            kind: 'pinned',
        })

        // 2) 最近访问 (未搜索才显示)
        const recentItems = q ? [] : recent.value
        out.push({
            id: GROUP_ID.recent,
            title: '最近访问',
            items: recentItems,
            visibleItems: isUserCollapsed(GROUP_ID.recent) ? [] : recentItems,
            collapsed: isUserCollapsed(GROUP_ID.recent),
            kind: 'recent',
        })

        // 3) 命令
        const cmdItems = commandsItems.value
        out.push({
            id: GROUP_ID.commands,
            title: '命令',
            items: cmdItems,
            visibleItems: isUserCollapsed(GROUP_ID.commands) ? [] : cmdItems,
            collapsed: isUserCollapsed(GROUP_ID.commands),
            kind: 'commands',
        })

        // 4) 应用程序
        const appsItems = allAppsSorted.value
        out.push({
            id: GROUP_ID.apps,
            title: '应用程序',
            items: appsItems,
            visibleItems: isUserCollapsed(GROUP_ID.apps) ? [] : appsItems,
            collapsed: isUserCollapsed(GROUP_ID.apps),
            kind: 'apps',
        })

        // 5) 所有文件 - 折叠影响 + 文件类型过滤 (无截断, vue-virtual-scroller 渲染)
        const allFiles = filesAllUnfiltered.value
        const filesItems = applyFileKindFilter(allFiles, fileKindFilter.value)
        out.push({
            id: GROUP_ID.files,
            title: '所有文件',
            items: allFiles,
            visibleItems: isUserCollapsed(GROUP_ID.files) ? [] : filesItems,
            collapsed: isUserCollapsed(GROUP_ID.files),
            kind: 'files',
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
     *
     * 空组 (items.length === 0) 不响应 toggle —— 既然没有内容, 折叠展开
     * 也就没有意义. 由 displayGroups 强制 collapsed = true 维持该行为,
     * store 这里再防一道, 防止 UI 误触穿透.
     */
    function toggleGroupCollapse(id: GroupId) {
        // 找到对应 group, 空组不响应. 找不到 (id 不存在) 也直接返回.
        const group = displayGroups.value.find((g) => g.id === id)
        if (!group || group.items.length === 0) return
        const next = new Set(collapsedGroups.value)
        if (next.has(id)) next.delete(id)
        else next.add(id)
        collapsedGroups.value = next
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
     *
     * @param fromClick - 是否来自用户点击；点击时抑制跨分组自动滚动.
     */
    function selectByIndex(idx: number, fromClick = false) {
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
     * 多选模式下的选择方法.
     * @param groupId 分组 ID
     * @param localIndex 分组内的本地索引
     * @param ctrl 是否按住 Ctrl
     * @param shift 是否按住 Shift
     */
    function selectWithModifiers(groupId: string, localIndex: number, ctrl: boolean, shift: boolean) {
        const group = displayGroups.value.find(g => g.id === groupId)
        if (!group || group.items.length === 0) return

        // 计算全局索引用于 selectedIndex (键盘焦点)
        const groupStartIdx = getGroupStartIndex(groupId)
        const globalIdx = groupStartIdx + localIndex
        const clampedGlobal = Math.max(0, Math.min(globalIdx, displayMax.value - 1))
        selectedIndex.value = clampedGlobal
        const item = displayList.value[clampedGlobal]
        selectedGlobalId.value = item?.id ?? null

        // 切换分组时清空旧选中 (互斥)
        if (activeSelectionGroupId.value !== groupId) {
            activeSelectionGroupId.value = groupId
            selectedIndexes.value = new Set()
        }

        const clamped = Math.max(0, Math.min(localIndex, group.visibleItems.length - 1))

        if (ctrl && shift) {
            selectRangeToggle(lastSelectedLocalIndex.value, clamped)
        } else if (shift) {
            selectRange(lastSelectedLocalIndex.value, clamped)
        } else if (ctrl) {
            toggleSelect(clamped)
        } else {
            clearSelection()
            addToSelection(clamped)
        }

        lastSelectedLocalIndex.value = clamped
    }

    /**
     * 获取分组的起始全局索引.
     */
    function getGroupStartIndex(groupId: string): number {
        let idx = 0
        for (const g of displayGroups.value) {
            if (g.id === groupId) return idx
            idx += g.visibleItems.length
        }
        return 0
    }

    /**
     * 选中指定范围的项目 (当前激活分组内的本地索引).
     */
    function selectRange(from: number, to: number) {
        clearSelection()
        const start = Math.min(from, to)
        const end = Math.max(from, to)
        for (let i = start; i <= end; i++) {
            addToSelection(i)
        }
    }

    /**
     * 反选指定范围的项目 (当前激活分组内的本地索引).
     */
    function selectRangeToggle(from: number, to: number) {
        const start = Math.min(from, to)
        const end = Math.max(from, to)
        for (let i = start; i <= end; i++) {
            toggleSelect(i)
        }
    }

    /**
     * 切换指定项目的选中状态 (当前激活分组内的本地索引).
     */
    function toggleSelect(localIndex: number) {
        const group = displayGroups.value.find(g => g.id === activeSelectionGroupId.value)
        if (!group || localIndex < 0 || localIndex >= group.visibleItems.length) return

        const indexes = selectedIndexes.value
        if (indexes.has(localIndex)) {
            indexes.delete(localIndex)
        } else {
            indexes.add(localIndex)
        }
        selectedIndexes.value = new Set(indexes)
    }

    /**
     * 添加项目到选中集合 (当前激活分组内的本地索引).
     */
    function addToSelection(localIndex: number) {
        const group = displayGroups.value.find(g => g.id === activeSelectionGroupId.value)
        if (!group || localIndex < 0 || localIndex >= group.visibleItems.length) return

        const indexes = selectedIndexes.value
        indexes.add(localIndex)
        selectedIndexes.value = new Set(indexes)
    }

    /**
     * 从选中集合中移除项目 (当前激活分组内的本地索引).
     */
    function removeFromSelection(localIndex: number) {
        const indexes = selectedIndexes.value
        indexes.delete(localIndex)
        selectedIndexes.value = new Set(indexes)
    }

    /**
     * 清空所有选中.
     */
    function clearSelection() {
        selectedIndexes.value = new Set()
    }

    /**
     * 判断指定分组内的指定索引是否被选中.
     */
    function isSelectedInGroup(groupId: string, localIndex: number): boolean {
        if (activeSelectionGroupId.value !== groupId) return false
        return selectedIndexes.value.has(localIndex)
    }

    /**
     * 获取当前激活分组中所有选中的项目.
     */
    const selectedItems = computed(() => {
        if (!activeSelectionGroupId.value) return []
        const group = displayGroups.value.find(g => g.id === activeSelectionGroupId.value)
        if (!group) return []
        const result: SearchResult[] = []
        for (const idx of selectedIndexes.value) {
            const item = group.items[idx]
            if (item) result.push(item)
        }
        return result
    })

    /**
     * 当 displayList 变化时 (搜索 / 索引刷新 / 折叠切换 / 分类筛选),
     * 仅做边界 clamp, 不按 ID 重定位.
     *
     * 设计说明: 同一 SearchResult 可能出现在多个分组中 (pinned / recent / apps),
     * 每个分组中的项都是独立的. 若按 ID 追踪会导致选中跳转到第一个匹配的分组,
     * 违反"列表项独立"的交互原则.
     *
     * - query 变化时: setQuery 中已主动重置 selectedIndex = 0
     * - 增量刷新 (列表变长): 保持当前索引不动, 越界则 clamp
     * - 折叠/展开分组: 仅做边界保护
     *
     * `flush: 'sync'` 同步触发, 避免同一 tick 内读取越界的 selectedIndex.
     */
    watch(
        displayMax,
        () => {
            if (displayMax.value === 0) {
                selectedIndex.value = 0
                selectedGlobalId.value = null
                return
            }
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
            // 置顶模式下打开文件不隐藏窗口, 方便连续操作.
            if (!alwaysOnTop.value) {
                visible.value = false
            }
        }
    }

    async function refreshAlwaysOnTop() {
        try {
            alwaysOnTop.value = await pinTopApi.get()
        } catch {
            // ignore
        }
    }

    async function setAlwaysOnTop(value: boolean) {
        try {
            await pinTopApi.set(value)
            alwaysOnTop.value = value
        } catch {
            // ignore
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
        alwaysOnTop,
        indexStatus,
        indexMessage,
        indexStats,
        indexVolumesTotal,
        indexVolumeIndex,
        indexCurrentVolume,
        appReady,
        /** 当前激活窗口路径 (WindowMonitor 推送). */
        activeAppPath,
        /** 当前激活窗口标题 (WindowMonitor 推送). */
        activeAppTitle,
        /** 最近激活应用历史, 给推荐算法当次级信号. */
        activeAppRecent,
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
        refreshAlwaysOnTop,
        setAlwaysOnTop,
        show,
        hide,
        toggle,
        // 新增的显示层 API
        displayGroups,
        displayList,
        displayMax,
        toggleGroupCollapse,
        setFileKindFilter,
        setGroupSortMode,
        groupSortModes,
        // 多选相关 API
        activeSelectionGroupId,
        selectedIndexes,
        selectedItems,
        isSelectedInGroup,
        selectWithModifiers,
        toggleSelect,
        clearSelection,
        // 窗口监控 / 推荐上下文 API
        syncWindowMonitor,
        listenWindowMonitor,
        applyWindowChanged,
        /** 当前激活应用所属类别 (诊断用). */
        matchAppCategoryByPath: (p: string) => matchAppCategoryByPath(p),
        // 智能排序打分, 暴露给 composables / 测试使用
        smartSortScore: (item: SearchResult, q: string) => smartSortScore(item, q),
        recommendationScores,
    }
})
