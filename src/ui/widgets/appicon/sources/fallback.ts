/**
 * FallbackIconSource —— 兜底 Lucide 通用图标.
 *
 * 与 useAppIcon 旧版 `chooseFallback` 行为一致: 不返回裸 AppWindow, 而是
 * 委托 `iconForFileKind(result)` 给"按文件类型/扩展名"的最精确 Lucide
 * 组件, 避免一批"普通文件"走通用占位 (Folder / Image / FileCode / ...).
 *
 * 永远返回非 null.
 */

import { iconForFileKind } from './knownAppIcons'
import type { IconSource, IconState } from './types'
import type { SearchResult } from '@/core/types/search'

export class FallbackIconSource implements IconSource {
  readonly name = 'fallback'

  resolve(item: SearchResult): IconState {
    return { kind: 'component', value: iconForFileKind(item) }
  }
}
