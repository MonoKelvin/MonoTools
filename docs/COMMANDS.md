# Command Bus / 命令总线

MonoTools 的所有交互动作都通过一个统一的 Command 总线调度。键盘 / 菜单 / 上下文菜单 / 全局快捷键都只是触发入口，真正要做什么由 `commandRegistry.execute(id)` 决定。

## 概念

```ts
interface Command<TArgs = unknown, TReturn = void> {
  id: string                       // 反向引用；唯一
  title: string                    // UI 中显示
  description?: string
  category: 'search' | 'window' | 'file' | 'app' | 'command' | 'system'
  shortcut?: string | string[]     // 本地 keydown
  icon?: Component
  keywords?: readonly string[]     // 搜索面板 / 命令面板用
  when?: () => boolean             // 触发条件
  enabled?: boolean
  run: (args: TArgs) => TReturn | Promise<TReturn>
}
```

注册：

```ts
import { commandRegistry } from '@/commands'
commandRegistry.register({
  id: 'app.cmd.navigate.settings',
  title: '设置',
  category: 'app',
  shortcut: 'Ctrl + ,',
  run: () => router.push('/settings'),
})
```

触发：

```ts
await commandRegistry.execute('app.cmd.navigate.settings')
```

跳过条件：`when()` 返回 false 或 `enabled === false` 时 `execute()` 是 noop。

错误处理：`run` 抛错时注册表 `onError` 监听器收到；UI 层可以接 toast 等。

## 内置命令

`src/commands/builtins/`：

- **search**：回车、上 / 下选择、Esc 关闭、Ctrl+Tab 切换分类、Ctrl+C 复制路径、Ctrl+Enter 打开所在文件夹、Ctrl+K 清空。
- **system**：navigate settings / commands、quit、show hotkeys、theme toggle。

新增命令时按命名空间 `category.cmd.action` 排，例如 `file.cmd.reveal`、`window.cmd.toggle-launcher`。

## 键盘绑定

`src/commands/bindings.ts` 提供：

- `matchesShortcut(combination, event)`：判断 KeyboardEvent 是否匹配 `"Alt + Space"` 字符串（兼容大小写、缩写 `Cmd`/`Meta`、`Control`/`Ctrl` 等）
- `serializeKey(parsedKey)` / `normalizeShortcut(str)`
- `dispatchKeyEvent(event)`：在搜索面板 `window keydown` 监听器被调用，找到第一个 shortcut 匹配的命令并 execute

`SearchPage` 在 mount 时调用 `registerCommands()` 把内置命令注册到 registry；onBeforeUnmount 时再清掉，名字以 `search.cmd.` 开头的命令。

## 命令面板 UI（`CommandsPanel.vue`）

按 category 分组展示：

- `search`: 数字键盘快捷键
- `window`: 切换、隐藏
- `app`: 路由 + 系统
- `system`: 主题、辅助

右侧"添加"按钮保留原 `commandApi.add` 流程——新命令注册到 commandRegistry 后可在 `SearchResults` 中作为 `SearchResult` 出现。

## 旧 API 兼容

历史代码 `import { searchCommands } from '@/commands/searchCommands'` 仍走工作路径，但实现已迁移到 `commandRegistry`。新代码请直接使用 `@/commands` / `@/commands/registry`。
