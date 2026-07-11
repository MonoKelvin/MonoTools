import type { Component } from 'vue'

/**
 * 命令元数据（仅供 UI 消费）。
 *
 * 执行 + 业务逻辑 100% 在后端 —— 见 `src-tauri/src/command/`.
 * 前端拿到一个 `CommandSpec` 后：
 *  - 把它的 `id` 带到 `useCommandsStore().execute(id, args)`：发往后端 IPC `dispatch_command`
 *  - 用 title / shortcut / icon / keywords 渲染 CommandsPanel / HotkeyModal
 *
 * 不再有 `.run()` / `.when()` / `.enabled`：这些都由后端决定；前端只尊重 result.success。
 */
export interface CommandSpec {
  readonly id: string
  readonly title: string
  readonly description?: string
  readonly category: CommandCategory
  /** 本地 keydown 快捷键列表 */
  readonly shortcut?: ReadonlyArray<string> | string
  /** lucide 图标 */
  readonly icon?: Component
  /** 在搜索面板 / 命令面板中的关键字 */
  readonly keywords?: readonly string[]
}

export type CommandCategory =
  | 'search'
  | 'window'
  | 'file'
  | 'app'
  | 'command'
  | 'system'

/**
 * 命令可执行结果：来自后端 [`crate::command::CommandOutput`].
 */
export interface CommandExecutionResult {
  readonly success: boolean
  readonly message: string
  readonly data?: unknown
}
