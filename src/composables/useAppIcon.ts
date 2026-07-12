/**
 * useAppIcon —— 应用图标的客户端编排组合式函数.
 *
 * 3 级加载链 (从快到慢, 从便宜到昂贵):
 *   1. 静态命中: `lookupKnownIcon(title, path)` -> 内联 SVG data URL (同步, 0 延迟).
 *   2. 后端 IPC:  `appIconApi.get(path)`        -> 32x32 PNG base64 (异步, 1次 IPC).
 *   3. 通用兜底: `fallbackIconForResultType(rt)` -> Lucide 通用图标组件 (同步, 0 资源).
 *
 * 错误处理协议 (永不抛错):
 * - 静态命中失败: 自动进入下一级.
 * - IPC 失败 (null / 抛错 / Tauri 不可用): 自动进入下一级.
 * - 最终兜底一定返回, 组件可以直接渲染.
 *
 * 缓存:
 * - 模块级 Map<id, IconState> 缓存最终结果, 避免重复 IPC.
 * - 同 id 多次调用 `loadIcon` 直接命中缓存.
 */

import type { Component } from 'vue'
import { AppWindow } from '@lucide/vue'
import type { SearchResult } from '@/types/search'
import {
  lookupKnownIcon,
  fallbackIconForResultType,
} from '@/utils/knownAppIcons'
import { appIconApi } from '@/services/api'
import { isTauri } from '@/services/env'

/**
 * 三种图标状态:
 * - 'svg'      : 内联 SVG data URL, 用 <img src> 渲染.
 * - 'png'      : 后端提取的 base64 PNG, 用 <img src> 渲染.
 * - 'component': Lucide 通用图标组件, 用 <component :is> 渲染.
 */
export type IconKind = 'svg' | 'png' | 'component'

export type IconState =
  | { kind: 'svg'; value: string }
  | { kind: 'png'; value: string }
  | { kind: 'component'; value: Component }

/** 模块级缓存, key = SearchResult.id. 跨组件复用, 避免重复 IPC. */
const cache = new Map<string, Promise<IconState>>()

/** 从 SearchResult 中提取可执行文件路径 (用于图标检索). */
function extractPath(result: SearchResult): string {
  const a = result.action
  if (a && a.type === 'launch' && typeof a.data === 'string') {
    return a.data
  }
  return ''
}

/** 通用兜底: 根据 resultType 拿到 Lucide 组件; 解析失败时回退 AppWindow. */
function makeFallback(result: SearchResult): IconState {
  let comp: Component
  try {
    comp = fallbackIconForResultType(result.resultType)
  } catch {
    comp = AppWindow
  }
  if (!comp) comp = AppWindow
  return { kind: 'component', value: comp }
}

/**
 * 加载一个应用项的图标. 返回 Promise<IconState>.
 *
 * 该函数是幂等的: 同一 result.id 多次调用只触发一次实际加载, 后续直接命中缓存.
 */
export function useAppIcon() {
  /**
   * 主入口: 编排三级加载链.
   * 即使内部任何一步抛错, 也会返回最终兜底 (Lucide 通用图标), 永不 reject.
   */
  async function loadIcon(result: SearchResult): Promise<IconState> {
    const id = result?.id
    if (!id) return makeFallback(result)

    // 1) 缓存命中
    const cached = cache.get(id)
    if (cached) return cached

    // 2) 启动加载 promise 并立即写入缓存, 防止并发重复请求
    const promise = (async () => {
      const path = extractPath(result)
      const title = result.title || ''

      // 2.1 静态命中 (同步, Lucide 组件 / 旧版 SVG data URL 二者之一)
      const known = lookupKnownIcon(title, path)
      if (known) {
        // lookupKnownIcon 现在直接返回 IconState (新: component / 旧: svg).
        return known
      }

      // 2.2 后端 IPC 提取 (仅在 Tauri 环境)
      if (isTauri && path) {
        try {
          const base64 = await appIconApi.get(path)
          if (base64 && typeof base64 === 'string' && base64.length > 0) {
            // 后端返回纯 base64 字符串, 拼上 data URL 前缀
            return {
              kind: 'png',
              value: `data:image/png;base64,${base64}`,
            } satisfies IconState
          }
        } catch {
          // IPC 抛错: 静默吞掉, 进入兜底
        }
      }

      // 2.3 通用兜底
      return makeFallback(result)
    })()

    cache.set(id, promise)

    // 即使 promise 内部抛错也消化, 缓存兜底
    promise.catch(() => {
      cache.set(id, Promise.resolve(makeFallback(result)))
    })

    return promise
  }

  /** 清理缓存 (调试 / 内存压力时使用). */
  function clear(): void {
    cache.clear()
  }

  return { loadIcon, clear }
}
