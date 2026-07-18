import { computed, ref, type Ref } from 'vue'
import type { SearchResult } from '@/modules/search'
import { SEARCH_LIMITS_VISIBLE } from '@/core/config'
import { getFileKind } from '../utils/fileKinds'
import type { SortMode } from '@/core/config/sorting'
import { DEFAULT_SORT_BY_GROUP } from '@/core/config/sorting'
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
     * 智能排序打分 (已废弃).
     *
     * 此处只保留极简的 fallback 实现, 不再进行独立的"激活类别"和"启动次数"加权;
     * 所有推荐上下文已迁移到 store.smartSortScore. 业务侧应改用 store 中的版本.
     */
    // @deprecated 单一真源已迁移到 store.smartSortScore
    function smartSortScore(item: SearchResult, queryStr: string): number {
        let score = 0
        if (queryStr) {
            const q = queryStr.toLowerCase()
            const title = item.title.toLowerCase()
            const pos = title.indexOf(q)
            if (pos === 0) score += 15
            else if (pos > 0) score += 5
        }
        score += (item.score || 0) * 0.01
        return score
    }

    /**
     * 计算当前已打开应用的类别分布, 用于智能推荐 (已废弃).
     *
     * 单一真源: store.activeAppCategories (基于 WindowMonitor 推送).
     * 这里仅返回空以兼容遗留接口.
     */
    // @deprecated 单一真源已迁移到 store.activeAppCategories
    const activeAppCategories = computed<Record<string, number>>(() => ({}))

    /**
     * 推荐分数表 (已废弃).
     * 单一真源: store.recommendationScores.
     */
    // @deprecated 单一真源已迁移到 store.recommendationScores
    const recommendationScores = computed<Map<string, number>>(() => new Map())

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
