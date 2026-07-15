/**
 * IconSource 抽象 —— 把 useAppIcon 的 5-tier 链 (cache→known→lobehub→ipc→fallback)
 * 抽象成可注册的数据源.
 *
 * 关键设计:
 * - 每个 source 实现 `resolve()`, 按顺序尝试获取图标 state
 * - registry 集中管理所有 source, 顺序决定优先级
 * - 缓存层 (模块级 Map) 仍由 useAppIcon 统一管理, 不属于 source 范畴
 *
 * 未来扩展: "skip lobehub on low battery" / "user custom pack" 等场景只需
 * push 一个新 IconSource 到 registry, 不动核心逻辑.
 */

import type { Component } from 'vue'
import type { SearchResult } from '@/modules/search'

/** 4 种图标状态 (与 useAppIcon.IconKind 对齐). */
export type IconKind = 'svg' | 'png' | 'component' | 'monogram'

export type IconState =
  | { kind: 'svg'; value: string }
  | { kind: 'png'; value: string }
  | { kind: 'component'; value: Component }
  | { kind: 'monogram'; letter: string; color: string }

/**
 * 单个数据源抽象.
 *
 * `resolve()` 返回 `null` 表示"我不提供, 交给下一个 source".
 * 第一个非 null 返回就是最终结果, 不再 fallback.
 */
export interface IconSource {
  /** source 名称, 用于日志/debug. */
  readonly name: string
  /**
   * 尝试解析 item 的图标. 同步或异步均可; 异步时失败 (抛错) 应返回 null
   * 让下一个 source 接手, 而非 throw.
   */
  resolve(item: SearchResult): Promise<IconState | null> | IconState | null
}
