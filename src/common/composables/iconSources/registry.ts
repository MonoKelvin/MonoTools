/**
 * IconSource registry —— 集中管理所有数据源.
 *
 * 默认顺序 (从高优先级到低):
 * 1. KnownIconSource    —— 静态 knownAppIcons 查表 (零网络, 零 IPC)
 * 2. LobehubIconSource  —— LobeHub 模糊匹配 (可能网络, 仅 Tauri)
 * 3. IpcIconSource      —— 后端 IPC 提取真实 exe / lnk 图标 (仅 Tauri)
 * 4. FallbackIconSource —— iconForFileKind 兜底 (永不返回 null)
 *
 * 未来加新 source: push 到此数组即可, 不动 useAppIcon 主体逻辑.
 */

import type { IconSource } from './types'
import { KnownIconSource } from './known'
import { LobehubIconSource } from './lobehub'
import { IpcIconSource } from './ipc'
import { FallbackIconSource } from './fallback'

/** 单一 registry 数组 —— useAppIcon 在创建图标状态时按顺序遍历. */
export const iconSourceRegistry: IconSource[] = [
    new IpcIconSource(),
    new LobehubIconSource(),
    new KnownIconSource(),
    new FallbackIconSource(),
]

/**
 * 单个 source 解析结果 —— 既返回 IconState 也返回 source name,
 * 让编排器 (useAppIcon) 能写出准确的 "icon-trace:known / lobehub / ipc / fallback" 日志.
 */
export interface IconSourceHit {
    /** source 名称, 与 IconLogLevel 兼容. */
    source: string
    state: import('./types').IconState
}

/**
 * 顺序遍历 registry, 返回第一个非 null 的结果. 全部返回 null 时返回 null
 * (调用方应自己给一个默认 fallback).
 */
export async function resolveIconByRegistry(
    item: import('@/types/search').SearchResult,
    sources: IconSource[] = iconSourceRegistry,
): Promise<IconSourceHit | null> {
    for (const src of sources) {
        try {
            const r = await src.resolve(item)
            if (r != null) {
                return { source: src.name, state: r as import('./types').IconState }
            }
        } catch (e) {
            console.error(`[resolveIconByRegistry] source=${src.name} threw error: ${e}`)
        }
    }
    return null
}

// 类型重导出 (避免循环 import)
export type { IconState, IconSource, IconKind } from './types'
