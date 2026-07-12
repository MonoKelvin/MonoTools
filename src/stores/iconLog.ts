/**
 * useIconLog —— 图标加载诊断日志.
 *
 * 两类记录:
 * - failure: 各级加载失败 (lookupKnownIcon / LobeHub / appIconApi)
 *   走 logIconFailure(), 入队 + console.warn.
 * - trace:   各级"成功"信息 (进入哪一级 / 用了多少 ms / 命中哪种图标).
 *   走 logIconTrace(), 仅在 enabled 时输出, 用来确认修复是否生效.
 *
 * 集中收集前端 useAppIcon 全流程, 排查"图标为什么是空白"问题.
 * 设置 localStorage.mono_icon_debug = '1' 即开启所有日志, 跨刷新保留.
 */
import { reactive, computed } from 'vue'

export type IconLogStage =
  | 'empty-id'
  | 'lookupKnownIcon'
  | 'lobehub-miss'
  | 'lobehub-throw'
  | 'appIconApi-empty'
  | 'appIconApi-throw'
  | 'fallback-resolve'
  | 'unexpected-throw'
  | 'img-load-error'

/**
 * trace 用的 level 标签 —— 标识"图标最终是从哪一级拿到的".
 * - cache:    模块级 Map 命中, 0ms, 复用历史结果.
 * - known:    lookupKnownIcon 命中 (Lucide 组件 / 静态 SVG).
 * - lobehub:  LobeHub CDN 命中 (彩色 SVG data URL).
 * - ipc:      后端 appIconApi 命中 (PNG data URL).
 * - fallback: 走到兜底, 包含 monogram (无图标) 和 component (Lucide 通用).
 */
export type IconLogLevel = 'cache' | 'known' | 'lobehub' | 'ipc' | 'fallback'

export interface IconLogEntry {
  /** 时间戳 (ms). */
  ts: number
  /** 阶段标签. */
  stage: IconLogStage
  /** 关联的 SearchResult.id, 用于 groupCollapsed 输出. */
  id: string
  /** 标题 (人读). */
  title: string
  /** 可执行文件路径 (若有). */
  path: string
  /** 后端 resultType (apps / files / commands ...). */
  resultType: string
  /** 失败原因. */
  reason: string
}

export interface IconTraceEntry {
  /** 时间戳 (ms). */
  ts: number
  /** 关联的 SearchResult.id. */
  id: string
  /** 标题 (人读). */
  title: string
  /** 路径 (若有). */
  path: string
  /** 结果类型. */
  resultType: string
  /** 图标最终从哪一级拿到. */
  level: IconLogLevel
  /** 本次加载总耗时 (ms). 缓存命中为 0. */
  durationMs: number
  /** 图标渲染形态 (component / svg / png / monogram). */
  kind: IconKind
  /** 备注 (额外上下文, 比如 lobehub slug / 后端 path 等). */
  note?: string
}

export type IconKind = 'svg' | 'png' | 'component' | 'monogram'

interface State {
  enabled: boolean
  /** 环形 buffer, 最多保留 200 条. 满了之后丢最旧的. */
  entries: IconLogEntry[]
  /** trace 记录独立 buffer, 用于生成"图标加载来源分布"统计. */
  traces: IconTraceEntry[]
  /** 累计计数器 (从本页加载到现在的总命中数, 方便看"修复后图标覆盖率"). */
  counts: Record<IconLogLevel, number>
}

const MAX_ENTRIES = 200
const STORAGE_KEY = 'mono_icon_debug'

const state = reactive<State>({
  enabled: typeof localStorage !== 'undefined' && localStorage.getItem(STORAGE_KEY) === '1',
  entries: [],
  traces: [],
  counts: { cache: 0, known: 0, lobehub: 0, ipc: 0, fallback: 0 },
})

function push(entry: IconLogEntry) {
  state.entries.push(entry)
  if (state.entries.length > MAX_ENTRIES) {
    state.entries.splice(0, state.entries.length - MAX_ENTRIES)
  }
  // 输出到 console, 按 id 分组避免刷屏
  if (typeof console !== 'undefined' && state.enabled) {
    const tag = `[icon-log:${entry.stage}]`
    if (entry.id) {
      console.groupCollapsed(
        `${tag} ${entry.title || entry.id} (${entry.reason})`,
      )
      console.log('id:', entry.id)
      console.log('title:', entry.title)
      console.log('path:', entry.path)
      console.log('resultType:', entry.resultType)
      console.log('reason:', entry.reason)
      console.groupEnd()
    } else {
      console.warn(tag, entry)
    }
  }
}

function pushTrace(entry: IconTraceEntry) {
  state.traces.push(entry)
  if (state.traces.length > MAX_ENTRIES) {
    state.traces.splice(0, state.traces.length - MAX_ENTRIES)
  }
  state.counts[entry.level] = (state.counts[entry.level] ?? 0) + 1
  if (typeof console !== 'undefined' && state.enabled) {
    const tag = `[icon-trace:${entry.level}]`
    console.log(
      `${tag} ${entry.title || entry.id} → ${entry.kind} (${entry.durationMs}ms)` +
        (entry.note ? ` · ${entry.note}` : ''),
    )
  }
}

/** 记录一条图标失败. */
export function logIconFailure(entry: Omit<IconLogEntry, 'ts'>) {
  push({ ...entry, ts: Date.now() })
}

/** 记录一条图标成功加载的 trace. */
export function logIconTrace(entry: Omit<IconTraceEntry, 'ts'>) {
  pushTrace({ ...entry, ts: Date.now() })
}

/**
 * 在控制台打印"图标加载来源分布"汇总.
 *
 * 用途: 开启 mono_icon_debug 后, 在首屏搜索结果出来时调用一次, 一眼
 * 看出"修复前 vs 修复后"的差距. 例如:
 *   cache 120, known 8, lobehub 0, ipc 3, fallback 2
 * 意味着 90%+ 命中缓存或静态, 兜底极少, 修复有效.
 */
export function dumpIconTraceSummary(): void {
  if (typeof console === 'undefined') return
  const total = Object.values(state.counts).reduce((a, b) => a + b, 0)
  const fallbackPct = total > 0 ? ((state.counts.fallback / total) * 100).toFixed(1) : '0.0'
  console.groupCollapsed(
    `[icon-trace:summary] total=${total}, fallback=${state.counts.fallback} (${fallbackPct}%)`,
  )
  console.table({
    cache: state.counts.cache,
    known: state.counts.known,
    lobehub: state.counts.lobehub,
    ipc: state.counts.ipc,
    fallback: state.counts.fallback,
  })
  // 最近 10 条 trace, 帮助定位具体哪些项走了兜底
  const recent = state.traces.slice(-10)
  if (recent.length) {
    console.log('recent traces (last 10):')
    console.table(
      recent.map((t) => ({
        level: t.level,
        kind: t.kind,
        title: t.title,
        resultType: t.resultType,
        ms: t.durationMs,
      })),
    )
  }
  console.groupEnd()
}

/** 重置 trace 计数 (用于"在某个搜索操作前后对比"的场景). */
export function resetIconTrace(): void {
  state.traces = []
  state.counts = { cache: 0, known: 0, lobehub: 0, ipc: 0, fallback: 0 }
}

/** 切换调试模式. 持久化到 localStorage, 跨刷新保留. */
export function setIconDebugEnabled(on: boolean) {
  state.enabled = on
  if (typeof localStorage !== 'undefined') {
    if (on) localStorage.setItem(STORAGE_KEY, '1')
    else localStorage.removeItem(STORAGE_KEY)
  }
}

export function useIconLog() {
  return {
    enabled: computed(() => state.enabled),
    entries: computed(() => state.entries),
    traces: computed(() => state.traces),
    counts: computed(() => state.counts),
    setEnabled: setIconDebugEnabled,
    /** 清空环形 buffer. */
    clear: () => {
      state.entries = []
    },
    resetTrace: resetIconTrace,
    dumpSummary: dumpIconTraceSummary,
  }
}
