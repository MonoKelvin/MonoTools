/**
 * KnownIconSource —— 静态 knownAppIcons 查表.
 *
 * 调用 `lookupKnownIcon(title, path)` 直接拿到 IconState. 返回 null 让下一个 source 接手.
 *
 * 注意: lookupKnownIcon 已经返回完整的 IconState (kind: 'component'),
 * 不需要二次包装.
 */

import { lookupKnownIcon } from './knownAppIcons'
import type { IconSource, IconState } from './types'
import type { SearchResult } from '@/core/types/search'

export class KnownIconSource implements IconSource {
    readonly name = 'known'

    resolve(item: SearchResult): IconState | null {
        const path = extractPath(item)
        return lookupKnownIcon(item?.title ?? '', path ?? undefined)
    }
}

/**
 * 从 SearchResult 提取可调 IPC / 静态查表的 path.
 *
 * 关键: 同时支持 `launch` / `open` / `navigate` 与 `run` 四种 action,
 * 因为 UWP 应用走 `run` 动作 (command="explorer.exe", args=["shell:AppsFolder\\..."]),
 * 需要从中提取 shell:AppsFolder\\... 路径供图标源使用.
 *
 * 与 `useAppIcon.ts::extractPath` 行为一致, 抽到 iconSources/ 共享.
 */
export function extractPath(item: SearchResult): string {
    const a = item?.action
    if (!a) {
        return ''
    }
    if ((a.type === 'launch' || a.type === 'open') && typeof a.data === 'string') {
        return a.data
    }
    if (a.type === 'navigate' && typeof a.data === 'string') {
        return a.data
    }
    // UWP / 特殊快捷方式: run 动作, command="explorer.exe", args=["shell:AppsFolder\\..."]
    if (a.type === 'run' && typeof a.data === 'object' && a.data !== null) {
        const args = (a.data as { command: string; args: string[] }).args
        if (args && args.length > 0) {
            return args[0]
        }
    }
    return ''
}
