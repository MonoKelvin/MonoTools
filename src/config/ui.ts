/**
 * UI 通用配置 —— 字号 / ActionBar / 窗口尺寸.
 *
 * 这些值是 SCSS (`--text-md` 等) 与 TS 的"双重真源". 修改时两边都改.
 * 长期计划: 通过 Vite `additionalData` 把 TS 变量注入 SCSS, 消除双重.
 * 详见 CLAUDE.md "编码规范" 1.4.
 */

/**
 * 字号阶梯 (单位 px).
 *
 * 数值与 SCSS `--text-*` 变量一一对应:
 * - `--text-xxs` ↔ FONT_SIZES.xxs (kbd / 角标)
 * - `--text-xs`  ↔ FONT_SIZES.xs  (小 / type badge)
 * - `--text-sm`  ↔ FONT_SIZES.sm  (副标题 / path)
 * - `--text-md`  ↔ FONT_SIZES.base (body / 列表项)
 * - `--text-lg`  ↔ FONT_SIZES.appResultTitle (AppResultItem 标题)
 */
export const FONT_SIZES = {
  /** 极小 (kbd / 角标) */
  xxs: 10,
  /** 小 (文件大小 / type badge) */
  xs: 10.5,
  /** 副标题 (path) */
  sm: 11.5,
  /** body / 列表项 */
  base: 12,
  /** ResultItem 标题 (略小) */
  resultTitle: 13.5,
  /** AppResultItem 标题 (略大) */
  appResultTitle: 14,
  /** ActionBar 状态文本 */
  status: 11.5,
} as const

export type FontSizes = typeof FONT_SIZES

/**
 * ActionBar 状态自动隐藏时长.
 *
 * 语义:
 * - completed 后 N ms 隐藏, 让用户看够"完成状态" 又不占视觉空间
 * - error 后更久, 让用户看够错误详情
 */
export const ACTION_BAR_TIMEOUTS = {
  /** 完成后多少 ms 隐藏 */
  completedMs: 5000,
  /** 错误后多少 ms 隐藏 (略长) */
  errorMs: 8000,
} as const

/**
 * 搜索窗口尺寸 (像素).
 *
 * 关键约束 (来自 product spec):
 * - 宽固定 640px (光标定位 / 移动计算稳定)
 * - 高范围 320-580px (内容区最大 460px)
 * - 拖动边界 8px (WebView 拖拽响应宽度)
 */
export const WINDOW_DIMENSIONS = {
  /** 主窗口固定宽度 */
  fixedWidth: 640,
  /** 内容区最大高度 */
  contentAreaMax: 460,
  /** 默认宽度 (同 fixedWidth) */
  defaultWidth: 640,
  /** 最小高度 */
  minHeight: 320,
  /** 最大高度 */
  maxHeight: 580,
  /** 头部输入框 + 底部状态栏 合计高度 (用于计算 contentHeight) */
  headerFooterHeight: 88,
} as const

/**
 * UI 时间常量 (ms). 任何 `setTimeout(..., NN)` 字面量都应放这里.
 */
export const UI_DELAYS = {
  /** 窗口尺寸同步防抖 */
  resizeSyncMs: 50,
  /** 图标批量预取防抖 (displayList 变化后多久拉一次) */
  iconBatchDebounceMs: 200,
} as const
