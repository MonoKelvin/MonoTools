/**
 * useAppIcon —— 应用图标的客户端编排组合式函数.
 *
 * 5 级加载链 (从快到慢, 从便宜到昂贵):
 *   1. 模块级缓存: 同 id 直接命中 -> 0ms
 *   2. 静态命中:   `lookupKnownIcon(title, path)` -> Lucide 组件 -> 0ms
 *   3. LobeHub CDN: `lobehubFuzzyMatch(title, path)` -> 彩色 SVG data URL -> ~50-200ms
 *   4. 后端 IPC:    `appIconApi.get(path)` -> 32x32 PNG base64 -> 异步 IPC
 *   5. 通用兜底:    `fallbackIconForResultType(rt)` -> Lucide 通用图标
 *
 * 错误处理协议 (永不抛错):
 * - 任何一级失败: 自动进入下一级.
 * - 最终兜底一定返回, 组件可以直接渲染.
 * - **每级失败都会 console.warn 详细诊断** (含 id/title/path/level/原因),
 *   方便排查"图标怎么是空白"问题.
 *
 * 缓存:
 * - 模块级 Map<id, IconState> 缓存最终结果, 避免重复 IPC / 网络请求.
 */

import type { Component } from 'vue'
import { AppWindow } from '@lucide/vue'
import type { SearchResult } from '@/types/search'
import {
  lookupKnownIcon,
  fallbackIconForResultType,
  getMonogram,
  getMonogramColor,
} from '@/utils/knownAppIcons'
import { lobehubFuzzyMatch } from '@/utils/lobehubIcons'
import { appIconApi } from '@/services/api'
import { isTauri } from '@/services/env'
import {
  logIconFailure,
  logIconTrace,
  setIconDebugEnabled,
  dumpIconTraceSummary,
  resetIconTrace,
} from '@/stores/iconLog'

/**
 * 四种图标状态:
 * - 'svg'      : 内联 SVG data URL, 用 <img src> 渲染.
 * - 'png'      : 后端提取的 base64 PNG, 用 <img src> 渲染.
 * - 'component': Lucide 通用图标组件, 用 <component :is> 渲染.
 * - 'monogram' : 单字母占位符 (无任何真实图标时), 用纯 CSS div 渲染.
 */
export type IconKind = 'svg' | 'png' | 'component' | 'monogram'

export type IconState =
  | { kind: 'svg'; value: string }
  | { kind: 'png'; value: string }
  | { kind: 'component'; value: Component }
  | { kind: 'monogram'; letter: string; color: string }

/** 模块级缓存, key = SearchResult.id. 跨组件复用, 避免重复 IPC. */
const cache = new Map<string, Promise<IconState>>()

/** 已确认的 path 列表, 用于去重 batch IPC. */
const knownMissingPaths = new Set<string>()

/** 调试日志: 通过 useIconLog 集中管理, 不再单独使用 localStorage. */
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
 * 从 SearchResult 中提取可执行文件路径 (用于图标检索).
 *
 * 关键: 同时支持 `launch` 与 `open` 两种 action, 因为很多 .lnk
 * 快捷方式 / .bat 脚本在分类里会走 `open` 而非 `launch`.
 * 旧版只支持 launch, 导致一批"开始菜单"图标的 .lnk 走兜底, 视觉上变空白.
 */
function extractPath(result: SearchResult): string {
  const a = result.action
  if (!a) return ''
  if ((a.type === 'launch' || a.type === 'open') && typeof a.data === 'string') {
    return a.data
  }
  if (a.type === 'navigate' && typeof a.data === 'string') {
    return a.data
  }
  return ''
}

/** 通用兜底: 根据 resultType 拿到 Lucide 组件; 解析失败时回退 AppWindow. */
function makeFallback(result: SearchResult): IconState {
  let comp: Component
  try {
    comp = fallbackIconForResultType(result.resultType)
  } catch (e) {
    debugWarn('fallback-resolve', result, String(e))
    comp = AppWindow
  }
  if (!comp) comp = AppWindow
  return { kind: 'component', value: comp }
}

/**
 * 单字母 monogram 占位符 —— 当无任何真实图标可用, 且 result 是一个
 * "应用" 类型结果 (让字母比通用窗口图标更有视觉识别度) 时使用.
 * 32x32 暖色背景 + 大写首字母, 与整体黑白灰 UI 协调.
 */
function makeMonogram(result: SearchResult): IconState {
  try {
    const letter = getMonogram(result.title || '', result.resultType)
    const color = getMonogramColor(result.title || result.id || '?')
    return { kind: 'monogram', letter, color }
  } catch (e) {
    debugWarn('monogram-resolve', result, String(e))
    return { kind: 'monogram', letter: '?', color: 'rgba(245, 241, 232, 0.18)' }
  }
}

/**
 * 决定 fallback 形态: 应用类用 monogram (更强视觉锚点),
 * 其他类 (文件/命令/系统应用) 用 Lucide 通用组件.
 */
function chooseFallback(result: SearchResult): IconState {
  // 仅对"普通应用" (非系统/非 UWP) 用 monogram. 系统应用通常 Windows
  // 自带合理图标, 用 Lucide 通用图标就够; 文件类用扩展名更合适.
  if (result.category === 'apps' && result.resultType !== 'system-app' && result.resultType !== 'uwp-app') {
    return makeMonogram(result)
  }
  return makeFallback(result)
}

/**
 * 加载一个应用项的图标. 返回 Promise<IconState>.
 *
 * 该函数是幂等的: 同一 result.id 多次调用只触发一次实际加载, 后续直接命中缓存.
 *
 * 监控:
 * - 缓存命中 → trace 记 level='cache', durationMs=0
 * - 各级成功   → trace 记对应 level + 实际耗时
 * - 失败降级   → logIconFailure 记 stage, 然后 trace 记 fallback level
 *   (这样既能看到"为什么失败", 也能看到"最终用什么兜底")
 */
export function useAppIcon() {
  /**
   * 主入口: 编排四级加载链.
   * 即使内部任何一步抛错, 也会返回最终兜底 (Lucide 通用图标), 永不 reject.
   */
  async function loadIcon(result: SearchResult): Promise<IconState> {
    const id = result?.id
    const startTs =
      typeof performance !== 'undefined' ? performance.now() : Date.now()
    if (!id) {
      debugWarn('empty-id', result, 'id 为空, 走兜底')
      const fb = makeFallback(result)
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
      // 缓存命中: 复制上一轮的 trace 元数据(level 已经在首次记录时确定),
      // 这里只递增 counts.cache 计数, 便于看"二次渲染几乎免费".
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

    // 2) 启动加载 promise 并立即写入缓存, 防止并发重复请求
    // 共享状态: 闭包对象, 供 .then() trace 读取"最终命中哪一级".
    const trace: { level: 'known' | 'lobehub' | 'ipc' | 'fallback'; note?: string } = {
      level: 'fallback',
    }
    const promise = (async () => {
      const path = extractPath(result)
      const title = result.title || ''

      // 2.1 静态命中 (同步, Lucide 组件 / 旧版 SVG data URL 二者之一)
      try {
        const known = lookupKnownIcon(title, path)
        if (known) {
          // lookupKnownIcon 现在直接返回 IconState (新: component / 旧: svg).
          trace.level = 'known'
          trace.note = `static:${known.kind}`
          return known
        }
      } catch (e) {
        debugWarn('lookupKnownIcon', result, String(e))
      }

      // 2.2 LobeHub 彩色图标 (异步, 仅在 Tauri 环境)
      //     浏览器 mock 模式下跳过, 避免外网延迟阻塞首屏.
      if (isTauri && title) {
        try {
          const lobe = await lobehubFuzzyMatch(title, path)
          if (lobe) {
            trace.level = 'lobehub'
            trace.note = 'cdn-hit'
            return lobe
          }
          debugWarn('lobehub-miss', result, '所有 slug 都未命中 (404/超时/格式错)')
        } catch (e) {
          debugWarn('lobehub-throw', result, String(e))
        }
      }

      // 2.3 后端 IPC 提取 PNG (仅在 Tauri 环境)
      if (isTauri && path) {
        try {
          const base64 = await appIconApi.get(path)
          if (base64 && typeof base64 === 'string' && base64.length > 0) {
            // 后端返回纯 base64 字符串, 拼上 data URL 前缀
            trace.level = 'ipc'
            trace.note = path.length > 60 ? `…${path.slice(-60)}` : path
            return {
              kind: 'png',
              value: `data:image/png;base64,${base64}`,
            } satisfies IconState
          }
          debugWarn('appIconApi-empty', result, '后端返回 null/空字符串 (文件不存在 / 非 PE / 权限被拒)')
        } catch (e) {
          debugWarn('appIconApi-throw', result, String(e))
        }
      }

      // 2.4 兜底: 应用类 → monogram, 其他 → Lucide 通用组件
      const fb = chooseFallback(result)
      trace.level = 'fallback'
      trace.note = fb.kind === 'monogram' ? 'monogram' : 'lucide-fallback'
      return fb
    })()

    cache.set(id, promise)

    // 即使 promise 内部抛错也消化, 缓存兜底
    promise.catch((e) => {
      debugWarn('unexpected-throw', result, String(e))
      cache.set(id, Promise.resolve(chooseFallback(result)))
    })

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
          level: trace.level,
          durationMs: Math.round(duration * 100) / 100,
          kind: state.kind,
          note: trace.note,
        })
      })
      .catch(() => {
        // 已由上面的 promise.catch 记录过 unexpected-throw, 不再重复 trace
      })

    return promise
  }

  /** 清理缓存 (调试 / 内存压力时使用). */
  function clear(): void {
    cache.clear()
  }

  /** 启用/禁用调试日志 (不依赖 localStorage, 适合临时切换). */
  function setDebug(on: boolean) {
    // 委托给 iconLog 集中管理.
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
      for (let i = 0; i < targets.length; i++) {
        const t = targets[i]
        const raw = raws[i]
        if (raw && typeof raw === 'string' && raw.length > 0) {
          const dataUrl = `data:image/png;base64,${raw}`
          const state: IconState = { kind: 'png', value: dataUrl }
          cache.set(t.item.id, Promise.resolve(state))
        } else {
          // 后端返回空: 文件不存在 / 空白图标 / 提取失败
          knownMissingPaths.add(t.path)
          debugWarn(
            'appIconApi-empty',
            t.item,
            'batch 后端返回 null/空字符串',
          )
          cache.set(t.item.id, Promise.resolve(chooseFallback(t.item)))
        }
      }
    } catch (e) {
      // batch 整体失败: 退回到单条模式让各 loadIcon 自己处理
      for (const t of targets) {
        cache.set(t.item.id, Promise.resolve(chooseFallback(t.item)))
      }
      // eslint-disable-next-line no-console
      console.warn('[useAppIcon] batch failed, fell back to per-item:', e)
    }
  }

  return { loadIcon, loadIconsBatch, clear, setDebug, dumpSummary, resetTrace }
}
