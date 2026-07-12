/**
 * KnownIconSource —— 静态 knownAppIcons 查表.
 *
 * 调用 `lookupKnownIcon(title, path)` 直接拿到 IconState. 返回 null 让下一个 source 接手.
 *
 * 注意: lookupKnownIcon 已经返回完整的 IconState (kind: 'component'),
 * 不需要二次包装.
 */

import { lookupKnownIcon } from '@/utils/knownAppIcons'
import type { IconSource, IconState } from './types'
import type { SearchResult } from '@/types/search'

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
 * 关键: 同时支持 `launch` 与 `open` 两种 action, 因为很多 .lnk 快捷方式
 * / .bat 脚本在分类里会走 `open` 而非 `launch`. 旧版只支持 launch,
 * 导致一批"开始菜单"图标的 .lnk 走兜底, 视觉上变空白.
 *
 * 与 `useAppIcon.ts::extractPath` 行为一致, 抽到 iconSources/ 共享.
 */
export function extractPath(item: SearchResult): string {
  const a = item?.action
  if (!a) return ''
  if ((a.type === 'launch' || a.type === 'open') && typeof a.data === 'string') {
    return a.data
  }
  if (a.type === 'navigate' && typeof a.data === 'string') {
    return a.data
  }
  return ''
}
