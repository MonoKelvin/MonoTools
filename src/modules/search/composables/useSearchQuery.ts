import { ref } from 'vue'
import type { SearchResult, SearchOptions } from '@/modules/search'
import { searchApi } from '@/services/api'
import { SEARCH_DEBOUNCE_MS, SEARCH_LIMITS } from '@/core/config'

/**
 * 输入防抖: 30ms. 既消除每键击穿的连发 IPC, 又几乎无感.
 * 0ms 时用户连打 "chrome" 6 个字符 = 6 次 IPC, 在弱机上会卡顿;
 * 30ms 是 "字符级" 体验的甜蜜点 (低于人眼可感知延迟).
 *
 * 常量值来自 `src/config/search.ts::SEARCH_DEBOUNCE_MS`, 集中管理.
 */
const DEBOUNCE_MS = SEARCH_DEBOUNCE_MS

/**
 * 查询管理 composable.
 * 负责 query 状态、防抖搜索、搜索触发.
 */
export function useSearchQuery() {
    const query = ref('')
    const results = ref<SearchResult[]>([])
    const loading = ref(false)

    let debounceHandle: ReturnType<typeof setTimeout> | null = null
    /** 增量搜索防抖: 防止索引过程中频繁重搜导致 UI 卡顿 */
    let incrementalRefreshTimer: ReturnType<typeof setTimeout> | null = null
    /** 增量刷新间隔（毫秒）: 索引过程中每 200ms 最多刷新一次列表 */
    const INCREMENTAL_REFRESH_INTERVAL = 200

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

    async function runSearch(options?: Partial<SearchOptions>) {
        loading.value = true
        try {
            const effectiveLimit = query.value === ''
                ? SEARCH_LIMITS.emptyQueryLimit
                : SEARCH_LIMITS.defaultLimit
            const mergedOptions: Partial<SearchOptions> = {
                limit: effectiveLimit,
                ...options,
            }
            results.value = await searchApi.search(query.value, mergedOptions)
        } catch {
            results.value = []
        } finally {
            loading.value = false
        }
    }

    /**
     * 设置查询词并触发防抖搜索.
     * 调用方传入新 query, 自动重置选区.
     */
    function setQuery(next: string, resetSelection?: () => void) {
        query.value = next
        if (resetSelection) resetSelection()
        if (debounceHandle) clearTimeout(debounceHandle)
        debounceHandle = setTimeout(() => runSearch(), DEBOUNCE_MS)
    }

    /**
     * 清空查询并立即搜索.
     */
    function clearQuery(resetSelection?: () => void) {
        query.value = ''
        if (resetSelection) resetSelection()
        if (debounceHandle) clearTimeout(debounceHandle)
        runSearch()
    }

    /**
     * 启动那一刻就把 UI 填满: 立即触发一次空查询,
     * 让后端的 `recent_files/list` 走完,前端立刻展示最近文件 / 应用.
     */
    async function initialLoad() {
        await runSearch()
    }

    return {
        query,
        results,
        loading,
        setQuery,
        clearQuery,
        runSearch,
        initialLoad,
        triggerIncrementalRefresh,
    }
}
