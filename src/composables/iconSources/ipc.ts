/**
 * IpcIconSource —— 后端 IPC 提取真实 exe / lnk 图标.
 *
 * 关键修复:
 * 1. path 提取走 `extractPath` 共享函数 (来自 known.ts), 与原 useAppIcon 行为一致.
 * 2. 严格校验 base64: 长度 + 字符集 + PNG magic, 防止 Chromium 静默失败.
 * 3. 失败一律返回 null, 不抛错, 让下一个 source 接手.
 * 4. 仅在 Tauri 上下文启用, 浏览器 mock 直接跳过.
 */

import { appIconApi } from '@/services/api'
import { isTauri } from '@/services/env'
import { ICON_CONFIG } from '@/config/icon'
import { logIconFailure } from '@/stores/iconLog'
import { extractPath } from './known'
import type { IconSource, IconState } from './types'
import type { SearchResult } from '@/types/search'

export class IpcIconSource implements IconSource {
    readonly name = 'ipc'

    async resolve(item: SearchResult): Promise<IconState | null> {
        if (!isTauri) return null
        const path = extractPath(item)
        if (!path) return null

        try {
            const base64 = await appIconApi.get(path)
            
            if (
                typeof base64 !== 'string' ||
                base64.length < ICON_CONFIG.minBase64Length ||
                !/^[A-Za-z0-9+/=]+$/.test(base64) ||
                !base64.startsWith(ICON_CONFIG.pngMagicBase64)
            ) {
                logIconFailure({
                    stage: 'appIconApi-empty',
                    id: item?.id ?? '',
                    title: item?.title ?? '',
                    path,
                    resultType: item?.resultType ?? '',
                    reason: `无效 base64: type=${typeof base64} length=${(base64 as any)?.length ?? 'null'} (期望 ≥ ${ICON_CONFIG.minBase64Length})`,
                })
                return null
            }
            return { kind: 'png', value: `data:image/png;base64,${base64}` }
        } catch (e) {
            console.error(`[IpcIconSource] ERROR for path=${path}:`, e)
            logIconFailure({
                stage: 'appIconApi-throw',
                id: item?.id ?? '',
                title: item?.title ?? '',
                path,
                resultType: item?.resultType ?? '',
                reason: String(e),
            })
            return null
        }
    }
}
