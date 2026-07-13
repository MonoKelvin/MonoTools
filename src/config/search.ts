/**
 * 搜索行为配置 —— 防抖 / 可见上限.
 *
 * 设计变更 (2026-07-13): 不再设置数量上限. 后端 search_cmd 默认走 u32::MAX
 * 返回所有命中, 渲染侧由 vue-virtual-scroller 处理百万级数据.
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
 * 钉选 / 最近访问 / 文件组 / 命令的可见上限.
 *
 * 注: 这些不是"总数截断", 而是"UI 偏好 / 防误点击". 后端返回全量,
 * 这里只是控制 VGR / ActionBar / 侧栏的首屏行为.
 */
export const SEARCH_LIMITS_VISIBLE = {
  /** Pin (固定项目) 最多显示几条 */
  pinnedMax: 8,
  /** Recent (最近访问) 最多显示几条 */
  recentMax: 10,
  /** "命令" 分组显示条数上限 */
  commandsMax: 12,
  /** 首屏图标批量预取条数 (displayList slice(0, N)) */
  iconBatchPrefetch: 60,
} as const
