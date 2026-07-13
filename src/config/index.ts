/**
 * 前端 config 统一出口.
 *
 * 用法:
 * ```ts
 * import { ICON_CONFIG, FONT_SIZES, ACTION_BAR_TIMEOUTS,
 *          WINDOW_DIMENSIONS, SEARCH_DEBOUNCE_MS,
 *          SEARCH_LIMITS_VISIBLE } from '@/config'
 * ```
 *
 * 每个子模块有详细的字段说明与跨前后端同步要求, 详见对应文件.
 *
 * 注: 旧的 `SEARCH_LIMITS` 已被删除 (2026-07-13), 后端 search_cmd 现在走
 * u32::MAX 全量返回, 渲染侧由 vue-virtual-scroller 处理百万级数据.
 */

export { ICON_CONFIG, type IconConfig } from './icon'
export {
  FONT_SIZES, type FontSizes,
  ACTION_BAR_TIMEOUTS,
  WINDOW_DIMENSIONS,
  UI_DELAYS,
} from './ui'
export {
  SEARCH_DEBOUNCE_MS,
  SEARCH_LIMITS_VISIBLE,
  SEARCH_LIMITS,
} from './search'
