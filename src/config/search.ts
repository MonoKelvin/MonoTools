/**
 * 搜索行为配置 —— 防抖 / limit / 可见上限.
 *
 * 与后端 `src-tauri/src/config.rs::search::*` 同步, 改这里须同步改后端.
 * 详见 CLAUDE.md "编码规范" 1.3.
 */

/**
 * 搜索防抖 (ms).
 *
 * 经验值: 弱机 6 键连击 → 6 IPC, 加 30ms 防抖后 6 IPC → 1 IPC.
 * 30ms 对人感知不到, 但对 IPC 调度节省明显.
 */
export const SEARCH_DEBOUNCE_MS = 30

/**
 * 搜索 limit (结果数量上限).
 *
 * 语义:
 * - realtime: 用户有 query 时后端返回上限
 * - emptyQuery: 空 query (首屏全量) 后端返回上限
 * - maxClientOverride: 客户端 override 最大值
 * - loadMore / loadMoreMax: "显示更多" 按钮的默认 / 最大步长
 */
export const SEARCH_LIMITS = {
  /** 实时搜索 (有 query) */
  realtime: 200,
  /** 空查询 (首屏全量) */
  emptyQuery: 2000,
  /** 客户端 override 最大值 */
  maxClientOverride: 2000,
  /** search_more 默认 limit */
  loadMore: 50,
  /** search_more 最大 limit */
  loadMoreMax: 500,
} as const

/**
 * 钉选 / 最近访问 / 文件组 / 命令的可见上限.
 */
export const SEARCH_LIMITS_VISIBLE = {
  /** Pin (固定项目) 最多显示几条 */
  pinnedMax: 8,
  /** Recent (最近访问) 最多显示几条 */
  recentMax: 10,
  /** "所有文件" 分组首屏显示条数 (空查询时, 避免一次渲染 500+ DOM) */
  fileVisibleInitial: 80,
  /** "显示更多" 按钮每次增加条数 */
  fileVisibleStep: 50,
  /** "所有文件" 分组硬上限 (再点击"显示更多"也不会超过) */
  fileVisibleHardCap: 1000,
  /** "命令" 分组显示条数上限 */
  commandsMax: 12,
  /** 首屏图标批量预取条数 (displayList slice(0, N)) */
  iconBatchPrefetch: 60,
} as const
