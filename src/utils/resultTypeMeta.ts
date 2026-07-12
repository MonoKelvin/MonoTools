/**
 * resultType 元数据集中管理 —— label + icon 单点定义.
 *
 * 用途: ResultItem (角标短标签 + Lucide icon) 与 ActionBar (状态栏
 * 完整标签) 都从这张表查, 避免两处分别定义导致 drift.
 *
 * 设计原则:
 * - **可扩展**: 新增 resultType 只需在 RESULT_TYPE_META 加一行,
 *   组件 / Composable 自动获取最新 label + icon.
 * - **集中**: 任何 `record<resultType, ...>` 映射都应放这里, 不在
 *   组件内联 (除非是单点使用的兜底).
 * - **无副作用**: 纯数据 + 工具函数, 不依赖 Vue runtime.
 *
 * 字段含义:
 * - `label`: 短标签 (ResultItem 角标). 例: "文件夹".
 * - `labelFull`: 完整标签 (ActionBar 状态栏). 例: "文件夹目录".
 * - `icon`: 兜底 Lucide 组件 (PNG 加载失败或未尝试时显示).
 */

import type { Component } from 'vue'
import {
  Monitor, AppWindow, Package, Terminal, FileText, FolderOpen, FileImage, FileVideo,
  FileAudio, FileCode, FileArchive, File, Command,
} from '@lucide/vue'

export interface ResultTypeMeta {
  /** 短标签 (ResultItem 角标). 例: "文件夹" */
  label: string
  /** 完整标签 (ActionBar 状态栏). 例: "文件夹目录" */
  labelFull: string
  /** 兜底 Lucide 组件 */
  icon: Component
}

/**
 * resultType → 元数据 全量映射.
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
  'system-app':  { label: '系统',     labelFull: '系统程序',     icon: Monitor },
  'user-app':    { label: '用户',     labelFull: '用户程序',     icon: AppWindow },
  'uwp-app':     { label: 'UWP',     labelFull: 'UWP 程序',     icon: Package },
  'directory':   { label: '文件夹',   labelFull: '文件夹目录',   icon: FolderOpen },
  'document':    { label: '文档',     labelFull: '文档文件',     icon: FileText },
  'image':       { label: '图片',     labelFull: '图片文件',     icon: FileImage },
  'video':       { label: '视频',     labelFull: '视频文件',     icon: FileVideo },
  'audio':       { label: '音频',     labelFull: '音频文件',     icon: FileAudio },
  'executable':  { label: '可执行',   labelFull: '可执行文件',   icon: FileCode },
  'archive':     { label: '压缩',     labelFull: '压缩文件',     icon: FileArchive },
  'other-file':  { label: '其他',     labelFull: '其他文件',     icon: File },
  'command':     { label: '命令',     labelFull: '自定义命令',   icon: Terminal },
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
