/**
 * resultType 元数据集中管理 —— label + icon + color 单点定义.
 *
 * 用途: ResultItem (角标短标签 + Lucide icon + 低饱和色彩) 与 ActionBar (状态栏
 * 完整标签) 都从这张表查, 避免两处分别定义导致 drift.
 *
 * 设计原则:
 * - **可扩展**: 新增 resultType 只需在 RESULT_TYPE_META 加一行,
 *   组件 / Composable 自动获取最新 label + icon + color.
 * - **集中**: 任何 `record<resultType, ...>` 映射都应放这里, 不在
 *   组件内联 (除非是单点使用的兜底).
 * - **无副作用**: 纯数据 + 工具函数, 不依赖 Vue runtime.
 *
 * 字段含义:
 * - `label`: 短标签 (ResultItem 角标). 例: "文件夹".
 * - `labelFull`: 完整标签 (ActionBar 状态栏). 例: "文件夹目录".
 * - `icon`: 兜底 Lucide 组件 (PNG 加载失败或未尝试时显示).
 * - `color`: 低饱和 CSS 颜色 (HSL), 用于图标染色和 badge 背景.
 */

import type { Component } from 'vue'
import {
  Monitor, AppWindow, Package, Terminal, FileText, FolderOpen, FileImage, FileVideo,
  FileAudio, FileCode, FileArchive, File, Command, Code2, FileCode2,
  FileType, FileType2, Link2, Settings2, Boxes, FileSymlink
} from '@lucide/vue'

export interface ResultTypeMeta {
  /** 短标签 (ResultItem 角标). 例: "文件夹" */
  label: string
  /** 完整标签 (ActionBar 状态栏). 例: "文件夹目录" */
  labelFull: string
  /** 兜底 Lucide 组件 */
  icon: Component
  /** 低饱和 CSS 颜色 (HSL), 用于图标染色和 badge 背景 */
  color: string
}

/**
 * resultType → 元数据 全量映射.
 *
 * 颜色设计原则:
 * - **类型区分度最大化**: 每种类型使用显著不同的 hue (色相), 避免色相接近 (间隔 ≥ 30°)
 * - **深色/浅色主题统一可读**: 饱和度 30-50%, 亮度 45-60%, 确保在深色和浅色背景上都有足够对比度
 * - **语义一致性**: 蓝色=系统/应用, 绿色=媒体/文档, 橙色=配置/快捷方式, 紫色=特殊类型
 *
 * 命名约定:
 * - `system-app` / `user-app` / `uwp-app`: 应用类, 走 AppResultItem 渲染
 *   (这里 icon 主要作为兜底, AppResultItem 用 AppWindow 默认)
 * - `directory` / `document` / `image` / `video` / `audio` / `executable`
 *   / `archive` / `other-file`: 文件类, 走 ResultItem 渲染
 * - `command`: 自定义命令, 走 ResultItem (icon=Terminal)
 *
 * 新增 type 时: 在 app_search / file_search 后端 SearchResult 的
 * `resultType` 字段用同样的字符串, 即可自动匹配.
 */
export const RESULT_TYPE_META: Record<string, ResultTypeMeta> = {
  'system-app':   { label: '系统',    labelFull: '系统程序',     icon: Monitor,   color: 'hsl(220, 40%, 55%)' },   // 蓝色
  'user-app':     { label: '用户',    labelFull: '用户程序',     icon: AppWindow, color: 'hsl(210, 40%, 58%)' },   // 天蓝
  'uwp-app':      { label: 'UWP',    labelFull: 'UWP 程序',     icon: Package,   color: 'hsl(270, 40%, 62%)' },   // 紫色
  'directory':    { label: '文件夹',  labelFull: '文件夹目录',   icon: FolderOpen, color: 'hsl(40, 40%, 55%)' },    // 浅橙色
  'document':     { label: '文档',    labelFull: '文档文件',     icon: FileText,  color: 'hsl(160, 40%, 48%)' },   // 青绿
  'image':        { label: '图片',    labelFull: '图片文件',     icon: FileImage, color: 'hsl(340, 40%, 58%)' },   // 玫红
  'svg':          { label: 'SVG',     labelFull: 'SVG 矢量图',   icon: FileType,  color: 'hsl(30, 40%, 55%)' },      // 橙色
  'video':        { label: '视频',    labelFull: '视频文件',     icon: FileVideo, color: 'hsl(280, 40%, 58%)' },   // 紫红
  'audio':        { label: '音频',    labelFull: '音频文件',     icon: FileAudio, color: 'hsl(320, 40%, 58%)' },   // 品红
  'executable':   { label: '程序',    labelFull: '程序文件',     icon: FileCode,  color: 'hsl(130, 40%, 50%)' },   // 绿色
  'library':      { label: '库',      labelFull: '库文件',       icon: Boxes,     color: 'hsl(15, 40%, 52%)' },      // 暖橙
  'static-lib':   { label: '静态库',  labelFull: '静态库文件',   icon: Boxes,     color: 'hsl(260, 20%, 55%)' },   // 浅灰紫
  'dynamic-lib':  { label: '动态库',  labelFull: '动态库文件',   icon: Boxes,     color: 'hsl(30, 40%, 52%)' },    // 亮橙
  'archive':      { label: '压缩',    labelFull: '压缩文件',     icon: FileArchive, color: 'hsl(45, 40%, 50%)' },  // 琥珀色
  'shortcut':     { label: '快捷方式', labelFull: '快捷方式',    icon: Link2,     color: 'hsl(210, 40%, 55%)' },   // 浅蓝色
  'html':         { label: '网页',    labelFull: '网页文件',     icon: FileType,  color: 'hsl(190, 40%, 50%)' },   // 青色
  'font':         { label: '字体',    labelFull: '字体文件',     icon: FileType2, color: 'hsl(300, 40%, 55%)' },   // 紫粉色
  'config':       { label: '配置',    labelFull: '配置文件',     icon: Settings2, color: 'hsl(20, 40%, 55%)' },    // 橙色
  'code':         { label: '代码',    labelFull: '代码文件',     icon: Code2,     color: 'hsl(175, 40%, 48%)' },    // 翠绿色
  'other-file':   { label: '其他',    labelFull: '其他文件',     icon: File,      color: 'hsl(0, 0%, 55%)' },      // 灰色
  'command':      { label: '命令',    labelFull: '自定义命令',   icon: Terminal,  color: 'hsl(25, 40%, 55%)' },      // 亮橙色
}

/**
 * 查 resultType 元数据. 没查到返回 undefined, 调用方决定兜底.
 *
 * 用法:
 * ```ts
 * const m = resultTypeMeta(rt)
 * const icon = m?.icon ?? Command   // 兜底
 * const label = m?.label ?? ''      // 空字符串兜底
 * ```
 */
export function resultTypeMeta(rt: string | undefined | null): ResultTypeMeta | undefined {
  if (!rt) return undefined
  return RESULT_TYPE_META[rt]
}

/**
 * 将 HSL 颜色转为带 alpha 的背景色 (用于 badge/图标背景).
 * @param hslColor - HSL 字符串 (如 "hsl(220, 45%, 55%)")
 * @param alpha - 透明度, 默认 0.2
 * @returns HSLA 字符串
 */
export function hslToAlpha(hslColor: string, alpha = 0.25): string {
  return hslColor.replace('hsl(', 'hsla(').replace(')', `, ${alpha})`)
}
