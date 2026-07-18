import { computed, ref, type Ref } from 'vue'
import type { SearchResult } from '@/modules/search'
import { SEARCH_LIMITS_VISIBLE } from '@/core/config'
import { getFileKind } from '../utils/fileKinds'
import type { SortMode } from '@/core/config/sorting'
import {
    SMART_WEIGHTS,
    APP_CATEGORIES,
    RECOMMENDATION_MAP,
    DEFAULT_SORT_BY_GROUP,
} from '@/core/config/sorting'
import { GROUP_ID, type GroupId, type DisplayGroup } from '../store'

/** 固定项目最多展示多少个 (避免分组过高). */
const PINNED_MAX = SEARCH_LIMITS_VISIBLE.pinnedMax
/** 最近访问展示多少个. */
const RECENT_MAX = SEARCH_LIMITS_VISIBLE.recentMax

export interface UseSearchResultsDeps {
    /** 搜索结果列表 (来自 useSearchQuery). */
    results: Ref<SearchResult[]>
    /** 查询词 (用于智能排序打分). */
    query: Ref<string>
    /** 用户手动固定的 id 列表 (来自 usePins). */
    pinnedIds: Ref<string[]>
}

/**
 * 结果管理 composable.
 * 负责结果分组、分类筛选、文件类型过滤、智能排序、display pipeline.
 */
export function useSearchResults(deps: UseSearchResultsDeps) {
    const { results, query, pinnedIds } = deps

    const activeCategory = ref<'all' | 'apps' | 'files' | 'commands'>('all')

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
     * 用户已折叠的分组 ID 集合. 默认全部展开. VGR 通过 props 读取,
     * store 是状态的唯一所有者, 避免 view 层和 store 状态漂移.
     */
    const collapsedGroups = ref<Set<GroupId>>(new Set())

    /** 文件类型过滤: 由 VGR 通过 `setFileKindFilter` 控制. */
    const fileKindFilter = ref<Set<string>>(new Set())

    /**
     * 计算当前已打开应用的类别分布, 用于智能推荐.
     * 基于 recent (按 launch_count 排序的前 N 项) 推断用户偏好.
     *
     * 改进点:
     * - 使用对数缩放 launch_count, 避免单一类别垄断
     * - 考虑类别多样性: 如果某类别已经很强 (>= 3 个应用), 降低其权重
     * - 时间衰减: 最近使用的应用权重更高
     */
    const activeAppCategories = computed<Record<string, number>>(() => {
        const cats: Record<string, number> = {}
        const recentApps = recent.value.filter((r) => r.category === 'apps')
        recentApps.forEach((app, idx) => {
            const titleLower = app.title.toLowerCase()
            // 排名权重: 越靠前权重越高, 使用对数缩放
            const rankWeight = Math.log(recentApps.length - idx + 1)
            // 启动次数权重: 对数缩放
            const countWeight = Math.log((app.score || 0) + 1)
            const combinedWeight = rankWeight * countWeight
            for (const [cat, keywords] of Object.entries(APP_CATEGORIES)) {
                if (keywords.some((kw) => titleLower.includes(kw.toLowerCase()))) {
                    cats[cat] = (cats[cat] || 0) + combinedWeight
                }
            }
        })
        return cats
    })

    /**
     * 根据用户已打开应用的类别, 计算推荐类别集合.
     * 返回: Map<resultId, bonusScore>
     *
     * 改进点:
     * - 推荐分数与用户对该类别的偏好强度成正比
     * - 避免过度推荐: 如果某推荐类别已经有 >= 2 个应用在 recent 中, 降低推荐分数
     * - 支持多级推荐: 强偏好类别推荐分数更高
     */
    const recommendationScores = computed<Map<string, number>>(() => {
        const scores = new Map<string, number>()
        const activeCats = activeAppCategories.value
        if (Object.keys(activeCats).length === 0) return scores

        const userCats = Object.keys(activeCats).filter((c) => activeCats[c] > 0)
        const targetCats = new Set<string>()
        for (const cat of userCats) {
            const recs = RECOMMENDATION_MAP[cat] || []
            for (const r of recs) targetCats.add(r)
        }
        for (const cat of userCats) {
            if (activeCats[cat] >= 3) targetCats.delete(cat)
        }
        if (targetCats.size === 0) return scores

        // 计算每个推荐类别的"已有数量", 用于降权
        const recentApps = recent.value.filter((r) => r.category === 'apps')
        const catCounts: Record<string, number> = {}
        recentApps.forEach((app) => {
            const titleLower = app.title.toLowerCase()
            for (const cat of targetCats) {
                const keywords = APP_CATEGORIES[cat] || []
                if (keywords.some((kw) => titleLower.includes(kw.toLowerCase()))) {
                    catCounts[cat] = (catCounts[cat] || 0) + 1
                }
            }
        })

        const baseBonus = SMART_WEIGHTS.recommendation
        for (const app of results.value) {
            if (app.category !== 'apps') continue
            const titleLower = app.title.toLowerCase()
            for (const cat of targetCats) {
                const keywords = APP_CATEGORIES[cat] || []
                if (keywords.some((kw) => titleLower.includes(kw.toLowerCase()))) {
                    // 如果该推荐类别已经有应用在 recent 中, 降低推荐分数
                    const existingCount = catCounts[cat] || 0
                    const penalty = existingCount >= 2 ? 0.4 : existingCount >= 1 ? 0.7 : 1.0
                    // 偏好强度加成: 用户对该源类别的偏好越强, 推荐分数越高
                    const intensityBonus = Math.min(1.5, 1 + (activeCats[Object.keys(activeCats).find((c) => RECOMMENDATION_MAP[c]?.includes(cat)) || ''] || 0) / 20)
                    const finalBonus = baseBonus * penalty * intensityBonus
                    scores.set(app.id, (scores.get(app.id) || 0) + finalBonus)
                    break
                }
            }
        }
        return scores
    })

    /**
     * 智能排序打分.
     * 综合: 访问次数 (对数缩放) + 名称匹配 (位置敏感) + 目录访问 + 推荐权重 + 时间衰减.
     *
     * 设计原则:
     * - 后端 score 字段包含 launch_count 加权, 这里做对数缩放避免高频应用垄断
     * - 名称匹配: 前缀匹配 > 子串匹配, 匹配位置越靠前加分越多
     * - 推荐加分: 基于用户已打开应用的类别, 推荐相关类别
     * - 时间衰减: 最近使用的应用获得额外加分, 久未使用的逐渐降权
     */
    function smartSortScore(item: SearchResult, queryStr: string): number {
        let score = 0
        const w = SMART_WEIGHTS

        // 1) 访问次数: 使用对数缩放, 避免高频应用垄断
        // ln(count + 1) * weight: 1次=0, 10次=2.3, 100次=4.6, 1000次=6.9
        const countFactor = Math.log((item.score || 0) + 1)
        score += countFactor * w.launchCount

        // 2) 名称匹配 (查询词命中): 位置敏感
        if (queryStr) {
            const q = queryStr.toLowerCase()
            const title = item.title.toLowerCase()
            const pos = title.indexOf(q)
            if (pos === 0) {
                // 前缀匹配: 最高加分
                score += w.nameMatch * 3
            } else if (pos > 0) {
                // 子串匹配: 根据位置递减加分
                const posBonus = pos <= 3 ? 2 : 1
                score += w.nameMatch * posBonus
            }
        }

        // 3) 推荐加分
        const recScore = recommendationScores.value.get(item.id)
        if (recScore) score += recScore

        // 4) 时间衰减: 基于 item 的 score 字段近似推断"新鲜度"
        // 后端在空查询时按 launch_count 排序, 有查询时 score 已含时间因素
        // 这里简单处理: 如果 score 较高 (说明最近使用频繁), 额外加分
        if (item.score && item.score > 50) {
            score += w.launchCount * 0.5
        }

        return score
    }

    /** 按指定模式排序 items */
    function sortItems(items: SearchResult[], mode: SortMode, _groupId: string): SearchResult[] {
        switch (mode) {
            case 'name':
                return [...items].sort((a, b) => a.title.localeCompare(b.title))
            case 'recent':
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

    const filteredResults = computed(() => {
        if (activeCategory.value === 'all') return results.value
        return results.value.filter((r) => r.category === activeCategory.value)
    })

    /**
     * "固定项目" 分组: 仅含用户**手动** pin 的项.
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
     */
    const recent = computed<SearchResult[]>(() => {
        if (activeCategory.value === 'commands') return []
        const items = results.value.slice(0, RECENT_MAX)
        return sortItems(items, groupSortModes.value[GROUP_ID.recent], GROUP_ID.recent)
    })

    /** 文件类型过滤辅助函数 */
    function applyFileKindFilter(items: SearchResult[], kinds: Set<string>): SearchResult[] {
        if (kinds.size === 0) return items
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

    const filesAllUnfiltered = computed<SearchResult[]>(() => {
        return filteredResults.value.filter((r) => r.category === 'files')
    })

    const allAppsItems = computed<SearchResult[]>(() => allAppsSorted.value)

    const systemAppsItems = computed<SearchResult[]>(() => systemAppsSorted.value)

    /**
     * 6 个分组的完整数据 —— 单一真源, VGR 不再自己算分组.
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
        const appsItems = allAppsItems.value
        out.push({
            id: GROUP_ID.apps,
            title: '应用程序',
            items: appsItems,
            visibleItems: isUserCollapsed(GROUP_ID.apps) ? [] : appsItems,
            collapsed: isUserCollapsed(GROUP_ID.apps),
            kind: 'apps',
        })

        // 5) 所有文件 - 折叠影响 + 文件类型过滤
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
     * 切换一个分组的折叠状态.
     */
    function toggleGroupCollapse(id: GroupId) {
        const group = displayGroups.value.find((g) => g.id === id)
        if (!group || group.items.length === 0) return
        const next = new Set(collapsedGroups.value)
        if (next.has(id)) next.delete(id)
        else next.add(id)
        collapsedGroups.value = next
    }

    /**
     * 通知 store 前端文件类型过滤状态.
     */
    function setFileKindFilter(kinds: Set<string>) {
        fileKindFilter.value = kinds
    }

    /** 分类切换 */
    function setCategory(c: typeof activeCategory.value) {
        activeCategory.value = c
    }

    return {
        activeCategory,
        groupSortModes,
        collapsedGroups,
        fileKindFilter,
        filteredResults,
        pinned,
        recent,
        allAppsSorted,
        systemAppsSorted,
        commandsItems,
        filesAllUnfiltered,
        allAppsItems,
        systemAppsItems,
        displayGroups,
        displayList,
        displayMax,
        activeAppCategories,
        recommendationScores,
        smartSortScore,
        sortItems,
        setGroupSortMode,
        setCategory,
        setFileKindFilter,
        toggleGroupCollapse,
    }
}
