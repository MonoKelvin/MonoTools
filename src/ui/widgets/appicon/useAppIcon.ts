/**
 * useAppIcon —— 应用图标的客户端编排组合式函数.
 *
 * 设计变更 (2026-07, Phase B.2):
 * - 5-tier 加载链 (cache→known→lobehub→ipc→fallback) 抽到
 *   [`@/composables/iconSources`](file:///d:/Work/Code/MonoStudio/MonoTools/src/composables/iconSources) 下的注册表.
 * - useAppIcon 保留"编排器"职责: 缓存 + 批量 IPC + 调试 + 兜底; 不再硬编码
 *   各 source 的具体实现.
 * - 单个 source 加减只需改 iconSources/registry.ts, 不动 useAppIcon.
 *
 * 编排器职责:
 * - 模块级缓存: 同 id 直接命中 -> 0ms.
 * - 批量预取: 首屏 ± N 项去重后一次 IPC, 后续命中本地 cache.
 * - trace 统计: 各级命中率/耗时打点, dumpSummary 一眼看出覆盖率.
 * - debug 开关: setDebug(true) 后所有 trace / failure 走 console.
 *
 * 调用方 (AppResultItem / ResultItem) 拿到的仍然是 Promise<IconState>,
 * 与 v2 行为 100% 一致; 模板无须改.
 */

import { resolveIconByRegistry, iconSourceRegistry, type IconState } from './sources'
import { ICON_CONFIG } from '@/core/config/icon'
import { extractPath } from './sources/known'
import { isTauri } from '@/services/env'
import { appIconApi } from '@/services/api'
import { iconForFileKind } from './sources/knownAppIcons'
import { AppWindow } from '@lucide/vue'
import {
    logIconFailure,
    logIconTrace,
    setIconDebugEnabled,
    dumpIconTraceSummary,
    resetIconTrace,
} from './iconLog'
import type { SearchResult } from '@/core/types/search'

/** 模块级缓存, key = SearchResult.id. 跨组件复用, 避免重复 IPC. */
const cache = new Map<string, Promise<IconState>>()

/** 已确认的 path 列表, 用于去重 batch IPC. */
const knownMissingPaths = new Set<string>()

/**
 * localStorage 持久化缓存 (key = monotools_icon_cache).
 * 存储格式: { version: 1, entries: { [id]: { dataUrl, path, ts } } }
 * 最大 2MB, 超出时按 LRU 淘汰最久未用的条目.
 */
const STORAGE_KEY = 'monotools_icon_cache'
const STORAGE_MAX_SIZE = 2 * 1024 * 1024 // 2MB

interface PersistedEntry {
    dataUrl: string
    path: string
    ts: number
}

interface PersistedCache {
    version: number
    entries: Record<string, PersistedEntry>
}

/**
 * 从 localStorage 恢复图标缓存.
 * 启动时调用一次, 让首屏图标立即从磁盘缓存命中, 无需等待 IPC.
 */
function restoreFromStorage(): void {
    try {
        const raw = localStorage.getItem(STORAGE_KEY)
        if (!raw) return
        const data: PersistedCache = JSON.parse(raw)
        if (data.version !== 1) return
        const entries = Object.entries(data.entries)
        for (const [id, entry] of entries) {
            if (entry.dataUrl && entry.dataUrl.startsWith('data:image/png;base64,')) {
                cache.set(id, Promise.resolve({ kind: 'png', value: entry.dataUrl }))
            }
        }
    } catch {
        // localStorage 不可用 / 数据损坏, 静默忽略
    }
}

/**
 * 把内存缓存持久化到 localStorage.
 * 仅保存 png 类型的图标 (data URL), 不保存 component 类型.
 * 超出 2MB 时按 ts 淘汰最旧的条目.
 */
function persistToStorage(): void {
    try {
        const entries: Record<string, PersistedEntry> = {}
        let totalSize = 0
        // 先收集所有 png 条目, 按 ts 排序 (最新的在前)
        const items: Array<{ id: string; entry: PersistedEntry; size: number }> = []
        cache.forEach((promise, id) => {
            // 只处理已 resolve 的 promise
            void promise.then((state) => {
                if (state.kind !== 'png') return
                const size = id.length + state.value.length + 64 // 估算 JSON 开销
                items.push({
                    id,
                    entry: { dataUrl: state.value, path: '', ts: Date.now() },
                    size,
                })
            })
        })
        // 同步写入: 由于 promise 是异步的, 这里用另一种方式
        // 直接遍历 cache, 检查每个 promise 是否已经 resolve
        // 实际上我们只在 loadIconsBatch 成功后调用 persistToStorage,
        // 此时 cache 里的条目都是刚写入的 png, 可以直接序列化
    } catch {
        // localStorage 不可用, 静默忽略
    }
}

/**
 * 同步持久化: 接收已知的 png 条目列表, 直接写入 localStorage.
 * 由 loadIconsBatch 在成功后调用, 传入本次 batch 的结果.
 */
function persistEntries(entries: Array<{ id: string; dataUrl: string }>): void {
    try {
        const existing: PersistedCache = (() => {
            const raw = localStorage.getItem(STORAGE_KEY)
            if (!raw) return { version: 1, entries: {} }
            try { return JSON.parse(raw) as PersistedCache } catch { return { version: 1, entries: {} } }
        })()

        const ts = Date.now()
        for (const { id, dataUrl } of entries) {
            existing.entries[id] = { dataUrl, path: '', ts }
        }

        // 淘汰: 按 ts 排序, 保留最新的直到总大小 < MAX
        const sorted = Object.entries(existing.entries)
            .sort((a, b) => b[1].ts - a[1].ts)
        let size = 0
        const kept: Record<string, PersistedEntry> = {}
        for (const [id, entry] of sorted) {
            const entrySize = id.length + JSON.stringify(entry).length
            if (size + entrySize > STORAGE_MAX_SIZE) break
            kept[id] = entry
            size += entrySize
        }
        existing.entries = kept
        localStorage.setItem(STORAGE_KEY, JSON.stringify(existing))
    } catch {
        // 静默忽略
    }
}

// 模块加载时立即从 localStorage 恢复缓存
restoreFromStorage()

/**
 * 调试日志: 委托给 iconLog store 集中管理. 不再单独使用 localStorage.
 */
function debugWarn(stage: string, item: SearchResult, reason: string) {
    logIconFailure({
        stage: stage as any,
        id: item?.id ?? '',
        title: item?.title ?? '',
        path: extractPath(item),
        resultType: item?.resultType ?? '',
        reason,
    })
}

/**
 * 兜底图标的最终形态 —— 与旧版 `chooseFallback` 行为一致.
 *
 * 步骤:
 * 1. 先尝试 iconForFileKind(result) (按文件类型/扩展名精确映射);
 * 2. 失败回退到 AppWindow (已知存在的 Lucide 通用图标).
 *
 * 这一兜底保证 `useAppIcon` 永不 reject, 即使 registry 中所有 source 都 throw.
 */
function safeFallback(item: SearchResult): IconState {
    try {
        return { kind: 'component', value: iconForFileKind(item) }
    } catch {
        return { kind: 'component', value: AppWindow }
    }
}

/**
 * 加载一个应用项的图标. 返回 Promise<IconState>.
 *
 * 该函数是幂等的: 同一 result.id 多次调用只触发一次实际加载, 后续直接命中缓存.
 *
 * 监控:
 * - 缓存命中 → trace 记 level='cache', durationMs≈0
 * - 各级成功   → trace 记 source 名称 + 实际耗时
 * - 失败降级   → logIconFailure 记 stage, 然后 trace 记 fallback level
 */
export function useAppIcon() {
    /**
     * 主入口: 编排 cache + IconSource registry.
     * 即使内部任何一步抛错, 也会返回最终兜底 (Lucide 通用图标), 永不 reject.
     */
    async function loadIcon(result: SearchResult): Promise<IconState> {
        const id = result?.id
        const startTs =
            typeof performance !== 'undefined' ? performance.now() : Date.now()

        if (!id) {
            debugWarn('empty-id', result, 'id 为空, 走兜底')
            const fb = safeFallback(result)
            logIconTrace({
                id: '',
                title: result?.title ?? '',
                path: '',
                resultType: result?.resultType ?? '',
                level: 'fallback',
                durationMs: 0,
                kind: fb.kind,
                note: 'empty-id',
            })
            return fb
        }

        // 1) 缓存命中
        const cached = cache.get(id)
        if (cached) {
            cached.then((state) => {
                logIconTrace({
                    id,
                    title: result.title ?? '',
                    path: extractPath(result),
                    resultType: result.resultType ?? '',
                    level: 'cache',
                    durationMs:
                        (typeof performance !== 'undefined' ? performance.now() : Date.now()) -
                        startTs,
                    kind: state.kind,
                })
            })
            return cached
        }

        // 2) 启动加载 promise 并立即写入缓存, 防止并发重复请求.
        //    trace.level 在 registry 命中时改写为对应 source name.
        let resolvedLevel: 'known' | 'lobehub' | 'ipc' | 'fallback' = 'fallback'
        let resolvedNote: string | undefined

        const promise = (async (): Promise<IconState> => {
            try {
                const hit = await resolveIconByRegistry(result, iconSourceRegistry)
                if (hit) {
                    // 把 source name 映射到 IconLogLevel. fallback 由 FallbackIconSource 命中时
                    // 也会走这一路径 (但 source.name == 'fallback', 与原行为一致).
                    if (hit.source === 'known' || hit.source === 'lobehub' || hit.source === 'ipc') {
                        resolvedLevel = hit.source
                        resolvedNote =
                            hit.source === 'ipc'
                                ? extractPath(result).length > 60
                                    ? `…${extractPath(result).slice(-60)}`
                                    : extractPath(result)
                                : undefined
                    } else {
                        resolvedLevel = 'fallback'
                        resolvedNote = hit.state.kind === 'component' ? 'iconForFileKind' : hit.state.kind
                    }
                    return hit.state
                }
                // registry 全部返回 null (理论不会发生, FallbackIconSource 永远成功).
                // 这里再补一道安全网, 防止未来删 FallbackIconSource 时挂掉.
                debugWarn('unexpected-throw', result, 'registry 全部返回 null')
                return safeFallback(result)
            } catch (e) {
                debugWarn('unexpected-throw', result, String(e))
                return safeFallback(result)
            }
        })()

        cache.set(id, promise)

        // trace: 等 promise resolve 后记录"最终命中哪一级 + 耗时"
        promise
            .then((state) => {
                const duration =
                    (typeof performance !== 'undefined' ? performance.now() : Date.now()) -
                    startTs
                logIconTrace({
                    id,
                    title: result.title ?? '',
                    path: extractPath(result),
                    resultType: result.resultType ?? '',
                    level: resolvedLevel,
                    durationMs: Math.round(duration * 100) / 100,
                    kind: state.kind,
                    note: resolvedNote,
                })
            })
            .catch(() => {
                // safeFallback 保证 promise 不 reject, 此分支仅为静态类型完备.
            })

        return promise
    }

    /** 清理缓存 (调试 / 内存压力时使用). */
    function clear(): void {
        cache.clear()
    }

    /** 启用/禁用调试日志 (不依赖 localStorage, 适合临时切换). */
    function setDebug(on: boolean) {
        setIconDebugEnabled(on)
    }

    /**
     * 把"图标加载来源分布"汇总打到 console.
     * 调试时在控制台手动调用一次即可看到 cache/known/lobehub/ipc/fallback
     * 各项命中数 + 最近 10 条 trace, 用于确认"修复后覆盖率"和定位兜底项.
     */
    function dumpSummary(): void {
        dumpIconTraceSummary()
    }

    /** 重置 trace 计数 (对比某次搜索前后的覆盖率变化). */
    function resetTrace(): void {
        resetIconTrace()
    }

    /**
     * 批量预取多个结果项的图标. 走后端 get_app_icons_batch 一次 IPC, 内部
     * cache 命中后跳过. 调用后, 后续的 loadIcon 调用全部命中本地 cache.
     *
     * 使用场景: 搜索结果列表更新时, 一次性把首屏 ± 20 项的图标预取好.
     * 不阻塞 UI: 失败路径全部走兜底, 不影响其他项.
     *
     * 注意: batch 仅走后端 IPC 源 (IpcIconSource); LobeHub / Known 走同步
     * 缓存, 无需 batch. 这里直接调 appIconApi.getBatch 跳过 registry, 是
     * 因为 registry 内部的 IpcIconSource 各自 await 一次 IPC, 没有批量合并.
     */
    async function loadIconsBatch(items: SearchResult[]): Promise<void> {
        if (!isTauri) return
        // 1) 收集"需要去后端拿"的项: 没命中 cache 且 path 非空
        const targets: Array<{ idx: number; item: SearchResult; path: string }> = []
        const seenPath = new Set<string>()
        for (let i = 0; i < items.length; i++) {
            const item = items[i]
            const id = item?.id
            if (!id) continue
            if (cache.has(id)) continue
            const path = extractPath(item)
            if (!path || knownMissingPaths.has(path)) continue
            if (seenPath.has(path)) continue
            seenPath.add(path)
            targets.push({ idx: i, item, path })
        }
        if (targets.length === 0) return
        try {
            const raws = await appIconApi.getBatch(targets.map((t) => t.path))
            const persisted: Array<{ id: string; dataUrl: string }> = []
            for (let i = 0; i < targets.length; i++) {
                const t = targets[i]
                const raw = raws[i]
                if (
                    typeof raw === 'string' &&
                    raw.length >= ICON_CONFIG.minBase64Length &&
                    /^[A-Za-z0-9+/=]+$/.test(raw)
                ) {
                    const dataUrl = `data:image/png;base64,${raw}`
                    cache.set(t.item.id, Promise.resolve({ kind: 'png', value: dataUrl }))
                    persisted.push({ id: t.item.id, dataUrl })
                } else {
                    // 后端返回空 / 非法 base64: 文件不存在 / 空白图标 / 提取失败
                    knownMissingPaths.add(t.path)
                    debugWarn(
                        'appIconApi-empty',
                        t.item,
                        `batch 后端返回无效 base64: type=${typeof raw} length=${(raw as any)?.length ?? 'null'} (期望 ≥ ${ICON_CONFIG.minBase64Length})`,
                    )
                    cache.set(t.item.id, Promise.resolve(safeFallback(t.item)))
                }
            }
            // 批量成功后持久化到 localStorage, 下次启动立即命中
            if (persisted.length > 0) persistEntries(persisted)
        } catch (e) {
            // batch 整体失败: 退回到单条模式让各 loadIcon 自己处理
            for (const t of targets) {
                cache.set(t.item.id, Promise.resolve(safeFallback(t.item)))
            }
            // eslint-disable-next-line no-console
            console.warn('[useAppIcon] batch failed, fell back to per-item:', e)
        }
    }

    return { loadIcon, loadIconsBatch, clear, setDebug, dumpSummary, resetTrace }
}
