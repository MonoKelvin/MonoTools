/**
 * LobehubIconSource —— LobeHub 模糊匹配.
 *
 * 调用 `lobehubFuzzyMatch(title, path)` 拿到 IconState. 返回 null 让下一个 source 接手.
 *
 * 注意: 必须在 Tauri 上下文才发起外网请求. 浏览器 mock 模式返回 null,
 * 避免外网延迟阻塞首屏. 这一策略与 useAppIcon 旧版一致.
 */

import { lobehubFuzzyMatch } from './lobehubIcons'
import { isTauri } from '@/services/env'
import { extractPath } from './known'
import type { IconSource, IconState } from './types'
import type { SearchResult } from '@/modules/search'

export class LobehubIconSource implements IconSource {
  readonly name = 'lobehub'

  async resolve(item: SearchResult): Promise<IconState | null> {
    if (!isTauri) return null
    const path = extractPath(item)
    const title = item?.title ?? ''
    if (!title) return null
    return await lobehubFuzzyMatch(title, path || undefined)
  }
}
