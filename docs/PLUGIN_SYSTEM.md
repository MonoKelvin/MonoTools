# MonoTools 插件系统详细设计

> 版本: v1.0  
> 日期: 2026-07-01  
> 状态: 核心设计文档

---

## 1. 设计目标

1. **热插拔**: 运行时加载、卸载、重载插件，无需重启 MonoTools
2. **自包含**: 每个插件一个目录，含元数据、代码、资源，即插即用
3. **权限隔离**: 基于 Tauri Capability + 自定义权限模型，插件只能访问声明的资源
4. **前后端统一**: 支持纯前端插件（Vue 组件）、纯后端插件（Rust/WASM）、全栈插件
5. **主题即插件**: 视觉主题通过插件实现，默认主题内置且不可卸载

---

## 2. 插件类型

| 类型 | ID 前缀 | 说明 | 示例 |
|------|---------|------|------|
| **Theme** | `theme:` | 提供 CSS 变量、配色方案、字体配置 | `builtin:default-theme` |
| **Provider** | `provider:` | 向搜索框注册结果源 | `builtin:file-search` |
| **Command** | `cmd:` | 提供可执行的快捷指令 | `builtin:command-palette` |
| **View** | `view:` | 提供完整的 Vue 页面/面板 | `builtin:settings` |
| **Integration** | `integration:` | 系统集成（剪贴板、窗口管理等） | 第三方扩展 |
| **Hybrid** | `hybrid:` | 同时包含前端和后端逻辑 | 复杂插件 |

---

## 3. 插件目录结构

每个插件是一个独立目录，结构如下：

```
{plugins-dir}/{plugin-id}/
├── plugin.json              # 插件元数据与配置（必须）
├── README.md                # 插件说明
├── LICENSE                  # 许可证
│
├── frontend/                # 前端代码（可选）
│   ├── index.ts             # 前端入口，导出 activate/deactivate
│   ├── components/          # Vue 组件
│   │   └── MainView.vue
│   ├── composables/         # 组合式函数
│   ├── styles/              # 插件私有样式
│   └── assets/              # 图片、字体等资源
│
├── backend/                 # 后端代码（可选）
│   ├── index.wasm           # WASM 模块（优先）
│   └── sidecar.exe          # 侧载进程（高级）
│
├── themes/                  # 主题资源（仅 Theme 类型）
│   ├── theme.css            # CSS 变量定义
│   ├── dark.css             # 暗色变体
│   └── light.css            # 亮色变体
│
└── locales/                 # 国际化
    ├── zh-CN.json
    └── en-US.json
```

---

## 4. plugin.json 规范

```json
{
  "$schema": "https://monotools.dev/schema/plugin.json",
  "id": "com.example.calculator",
  "name": "Calculator",
  "version": "1.0.0",
  "description": "Quick calculator for MonoTools",
  "author": "Example Author",
  "license": "MIT",
  "type": "command",
  
  "entry": {
    "frontend": "frontend/index.ts",
    "backend": "backend/index.wasm"
  },
  
  "permissions": [
    "clipboard:read",
    "clipboard:write",
    "ui:show-notification"
  ],
  
  "capabilities": {
    "search": {
      "enabled": true,
      "trigger": ["=", "calc"],
      "priority": 100
    },
    "commands": [
      {
        "id": "calc.eval",
        "title": "Evaluate Expression",
        "description": "Calculate mathematical expression",
        "shortcut": "ctrl+shift+c"
      }
    ]
  },
  
  "hooks": {
    "activate": "onActivate",
    "deactivate": "onDeactivate",
    "search": "onSearch",
    "execute": "onExecute"
  },
  
  "config": {
    "schema": {
      "precision": {
        "type": "number",
        "default": 2,
        "description": "Decimal precision"
      }
    }
  },
  
  "dependencies": {
    "monotools": ">=1.0.0",
    "plugins": []
  },
  
  "resources": {
    "memory_limit": "64MB",
    "cpu_limit": "10%"
  }
}
```

### 字段详解

| 字段                    | 类型     | 必填 | 说明                            |
| ----------------------- | -------- | ---- | ------------------------------- |
| `id`                    | string   | ✅    | 唯一标识，反向域名格式          |
| `name`                  | string   | ✅    | 显示名称                        |
| `version`               | string   | ✅    | SemVer 格式                     |
| `type`                  | string   | ✅    | 插件类型                        |
| `entry.frontend`        | string   | ❌    | 前端入口文件相对路径            |
| `entry.backend`         | string   | ❌    | 后端入口（WASM 或 sidecar）     |
| `permissions`           | string[] | ❌    | 所需权限列表                    |
| `capabilities.search`   | object   | ❌    | 搜索提供者配置                  |
| `capabilities.commands` | object[] | ❌    | 提供的命令列表                  |
| `hooks`                 | object   | ❌    | 生命周期钩子映射                |
| `config.schema`         | object   | ❌    | 插件配置 JSON Schema            |
| `dependencies`          | object   | ❌    | 依赖的 MonoTools 版本和其他插件 |

---

## 5. 权限系统 (Capability-based)

### 5.1 权限命名空间

```
core:*          # 核心权限（仅内置插件可申请）
  core:plugin.manage      # 管理其他插件
  core:theme.engine       # 注册主题
  core:search.register    # 注册搜索提供者

fs:*            # 文件系统
  fs:read                 # 读取指定目录
  fs:write                # 写入指定目录
  fs:index                # 访问文件索引数据库

ui:*            # UI 相关
  ui:show-notification    # 显示通知
  ui:show-dialog          # 显示对话框
  ui:register-view        # 注册视图组件
  ui:register-theme       # 注册主题

system:*        # 系统级
  system:exec             # 执行系统命令
  system:registry-read    # 读取注册表
  system:registry-write   # 写入注册表

clipboard:*     # 剪贴板
  clipboard:read
  clipboard:write

network:*       # 网络
  network:http            # HTTP 请求
  network:websocket       # WebSocket 连接
```

### 5.2 权限申请与校验

```rust
// 插件加载时，PluginManager 校验权限
fn load_plugin(manifest: &PluginManifest) -> Result<()> {
    for perm in &manifest.permissions {
        if !PERMISSION_REGISTRY.contains(perm) {
            return Err(PluginError::UnknownPermission(perm.clone()));
        }
        if perm.starts_with("core:") && !manifest.is_builtin {
            return Err(PluginError::ForbiddenPermission(perm.clone()));
        }
    }
    Ok(())
}
```

---

## 6. 生命周期钩子

### 6.1 钩子定义

| 钩子           | 触发时机             | 参数                                        | 返回值              |
| -------------- | -------------------- | ------------------------------------------- | ------------------- |
| `activate`     | 插件被加载/启用时    | `ctx: PluginContext`                        | `Result<(), Error>` |
| `deactivate`   | 插件被卸载/禁用时    | `ctx: PluginContext`                        | `Result<(), Error>` |
| `search`       | 用户输入匹配触发词时 | `query: SearchQuery`                        | `Vec<SearchResult>` |
| `execute`      | 用户选中插件结果时   | `item: SearchResult, ctx: ExecutionContext` | `ExecutionResult`   |
| `configChange` | 插件配置被修改时     | `newConfig: Value`                          | `void`              |
| `themeChange`  | 系统主题切换时       | `theme: ThemeInfo`                          | `void`              |

### 6.2 前端钩子实现示例

```typescript
// frontend/index.ts
import { definePlugin } from '@monotools/plugin-sdk'

export default definePlugin({
  id: 'com.example.calculator',
  
  async onActivate(ctx) {
    // 注册搜索触发器
    ctx.search.registerTrigger('=', { priority: 100 })
    
    // 注册命令
    ctx.commands.register({
      id: 'calc.eval',
      handler: (args) => evaluate(args.expression)
    })
    
    console.log('Calculator plugin activated')
  },
  
  async onDeactivate(ctx) {
    ctx.search.unregisterTrigger('=')
    ctx.commands.unregister('calc.eval')
  },
  
  async onSearch(ctx, query) {
    if (!query.raw.startsWith('=')) return []
    
    const expr = query.raw.slice(1).trim()
    try {
      const result = evaluate(expr)
      return [{
        id: `calc:${expr}`,
        title: `${result}`,
        subtitle: `= ${expr}`,
        icon: 'calculator',
        pluginId: this.id,
        action: 'copy'
      }]
    } catch {
      return []
    }
  },
  
  async onExecute(ctx, item) {
    if (item.action === 'copy') {
      await ctx.clipboard.writeText(item.title)
      ctx.ui.showNotification('已复制到剪贴板')
    }
  }
})
```

### 6.3 后端钩子实现示例（WASM）

```rust
// backend/src/lib.rs (编译为 wasm32-unknown-unknown)
use monotools_plugin_sdk::*;

#[plugin_entry]
fn activate(ctx: &PluginContext) -> Result<()> {
    ctx.register_search_trigger("file", Priority::High)?;
    Ok(())
}

#[search_handler]
fn on_search(query: &SearchQuery) -> Vec<SearchResult> {
    // 执行搜索逻辑
    vec![]
}
```

---

## 7. 内置插件（Builtin Plugins）

### 7.1 默认主题插件 (`builtin:default-theme`)

**类型**: Theme  
**状态**: 编译到主程序，不可卸载、不可禁用

**职责**:
- 提供 Linear 设计系统的 CSS 变量
- 实现深色/浅色/跟随系统三种模式
- 提供 6 种强调色切换

**资源路径**: 内置于 `src-tauri/resources/builtin-plugins/default-theme/`

**加载方式**: 主程序启动时直接注册，不经过文件系统加载。

### 7.2 应用启动器 (`builtin:app-launcher`)

**类型**: Provider  
**职责**:
- 扫描开始菜单、桌面快捷方式、注册表安装路径
- 提供应用名称搜索
- 支持拼音搜索

**索引更新**: 每次启动时增量扫描，后台线程执行。

### 7.3 文件搜索 (`builtin:file-search`)

**类型**: Provider  
**职责**:
- 基于 USN Journal 的全局文件索引
- 支持前缀匹配、模糊匹配、拼音匹配
- 实时文件监控增量更新

### 7.4 工作区管理 (`builtin:workspace-manager`)

**类型**: Hybrid  
**职责**:
- 提供 `workspace:save` 和 `workspace:restore` 命令
- 提供工作区列表视图
- 管理快照 JSON 文件

### 7.5 命令面板 (`builtin:command-palette`)

**类型**: Command  
**职责**:
- 提供系统级快捷命令（重启、关机、锁屏、睡眠）
- 提供 MonoTools 内部命令（打开设置、重新加载插件等）

### 7.6 设置面板 (`builtin:settings`)

**类型**: View  
**职责**:
- 提供设置 UI 界面
- 管理所有插件的配置表单生成（基于 JSON Schema）

---

## 8. 插件加载与热插拔机制

### 8.1 加载流程

```
扫描 plugins/ 目录
    │
    ▼
遍历每个子目录
    │
    ├── 1. 读取 plugin.json
    ├── 2. 校验 JSON Schema
    ├── 3. 检查依赖（MonoTools 版本、其他插件）
    ├── 4. 校验权限声明
    ├── 5. 检查 ID 冲突
    ├── 6. 加载前端代码（Vue 组件编译为 JS，动态 import）
    ├── 7. 加载后端代码（WASM 实例化或 sidecar 启动）
    ├── 8. 注册到 PluginRegistry
    ├── 9. 调用 activate 钩子
    └── 10. 广播 `plugin:loaded` 事件
```

### 8.2 热重载流程

```
文件系统监控 plugins/{id}/ 目录
    │
    ▼
检测到文件变更
    │
    ├── 1. 标记插件为 "stale"
    ├── 2. 等待 500ms 防抖
    ├── 3. 调用 deactivate 钩子
    ├── 4. 卸载前端组件（强制重新渲染）
    ├── 5. 释放后端资源（关闭 WASM 实例 / sidecar）
    ├── 6. 重新加载 plugin.json
    ├── 7. 重新执行加载流程 1-10
    └── 8. 广播 `plugin:reloaded` 事件
```

### 8.3 卸载流程

```
用户点击"卸载插件"
    │
    ▼
确认对话框
    │
    ├── 1. 调用 deactivate 钩子
    ├── 2. 注销搜索提供者
    ├── 3. 注销命令
    ├── 4. 注销视图组件
    ├── 5. 释放后端资源
    ├── 6. 从 PluginRegistry 移除
    ├── 7. 删除 plugins/{id}/ 目录
    └── 8. 广播 `plugin:uninstalled` 事件
```

---

## 9. 插件间通信

### 9.1 事件总线

```typescript
// 插件 A 发布事件
ctx.events.emit('my-plugin:data-updated', { key: 'value' })

// 插件 B 订阅事件
ctx.events.on('my-plugin:data-updated', (data) => {
  console.log(data)
})
```

### 9.2 API 调用

```typescript
// 插件 A 调用插件 B 提供的 API
const result = await ctx.plugins.invoke('com.example.plugin-b', 'customMethod', args)
```

### 9.3 共享状态（只读）

```typescript
// 读取其他插件的公开状态
const state = ctx.plugins.getState('com.example.plugin-b')
```

---

## 10. 主题插件特殊规范

### 10.1 主题 CSS 变量规范

主题插件必须定义以下 CSS 变量：

```css
/* themes/theme.css */
:root {
  /* 画布与表面 */
  --mt-bg-canvas: #010102;
  --mt-bg-surface-1: #0f1011;
  --mt-bg-surface-2: #141516;
  --mt-bg-surface-3: #18191a;
  --mt-bg-surface-4: #191a1b;
  
  /* 边框 */
  --mt-border-hairline: #23252a;
  --mt-border-strong: #34343a;
  
  /* 文字 */
  --mt-text-ink: #f7f8f8;
  --mt-text-muted: #d0d6e0;
  --mt-text-subtle: #8a8f98;
  --mt-text-tertiary: #62666d;
  
  /* 强调色 */
  --mt-accent-primary: #5e6ad2;
  --mt-accent-hover: #828fff;
  --mt-accent-focus: #5e69d1;
  
  /* 语义色 */
  --mt-semantic-success: #27a644;
  
  /* 圆角 */
  --mt-radius-xs: 4px;
  --mt-radius-sm: 6px;
  --mt-radius-md: 8px;
  --mt-radius-lg: 12px;
  --mt-radius-xl: 16px;
  --mt-radius-pill: 9999px;
  
  /* 间距 */
  --mt-space-xs: 4px;
  --mt-space-sm: 8px;
  --mt-space-md: 16px;
  --mt-space-lg: 24px;
  --mt-space-xl: 32px;
  
  /* 字体 */
  --mt-font-display: 'Inter', 'SF Pro Display', -apple-system, sans-serif;
  --mt-font-body: 'Inter', 'SF Pro Text', -apple-system, sans-serif;
  --mt-font-mono: 'JetBrains Mono', 'SF Mono', ui-monospace, monospace;
  
  /* 毛玻璃（可选） */
  --mt-glass-bg: rgba(15, 16, 17, 0.72);
  --mt-glass-blur: 20px;
  --mt-glass-border: rgba(255, 255, 255, 0.08);
}
```

### 10.2 主题切换机制

```rust
// Rust 后端
#[tauri::command]
fn set_theme(theme_id: String, app_handle: AppHandle) -> Result<()> {
    let theme = plugin_manager.get_theme(&theme_id)?;
    app_handle.emit("theme:changed", theme.css_variables)?;
    config_store.set("theme.active", json!(theme_id))?;
    Ok(())
}
```

```typescript
// Vue 前端
listen('theme:changed', (event) => {
  const vars = event.payload
  Object.entries(vars).forEach(([key, val]) => {
    document.documentElement.style.setProperty(key, val)
  })
})
```

---

## 11. 插件 SDK API 清单

### 11.1 前端 SDK (`@monotools/plugin-sdk`)

| API                                           | 说明             |
| --------------------------------------------- | ---------------- |
| `ctx.search.registerTrigger(prefix, options)` | 注册搜索触发前缀 |
| `ctx.search.unregisterTrigger(prefix)`        | 注销触发前缀     |
| `ctx.commands.register(command)`              | 注册命令         |
| `ctx.commands.unregister(commandId)`          | 注销命令         |
| `ctx.ui.showNotification(message, type?)`     | 显示通知         |
| `ctx.ui.showDialog(options)`                  | 显示对话框       |
| `ctx.ui.registerView(route, component)`       | 注册视图路由     |
| `ctx.clipboard.readText()`                    | 读取剪贴板       |
| `ctx.clipboard.writeText(text)`               | 写入剪贴板       |
| `ctx.storage.get(key)`                        | 读取插件私有存储 |
| `ctx.storage.set(key, value)`                 | 写入插件私有存储 |
| `ctx.config.get()`                            | 获取当前配置     |
| `ctx.config.set(key, value)`                  | 设置配置项       |
| `ctx.events.emit(event, data)`                | 发布事件         |
| `ctx.events.on(event, handler)`               | 订阅事件         |
| `ctx.invoke(command, args)`                   | 调用后端命令     |
| `ctx.theme.getCurrent()`                      | 获取当前主题     |
| `ctx.theme.onChange(handler)`                 | 监听主题变化     |

### 11.2 后端 SDK (`monotools-plugin-sdk`)

| API                                          | 说明                   |
| -------------------------------------------- | ---------------------- |
| `ctx.register_command(id, handler)`          | 注册后端命令           |
| `ctx.register_search_provider(id, provider)` | 注册搜索提供者         |
| `ctx.get_config()`                           | 获取插件配置           |
| `ctx.set_config(key, value)`                 | 设置插件配置           |
| `ctx.emit_event(event, data)`                | 向前端推送事件         |
| `ctx.log(level, message)`                    | 写入日志               |
| `ctx.fs.read(path)`                          | 读取文件（受权限限制） |
| `ctx.fs.write(path, data)`                   | 写入文件（受权限限制） |

---

## 12. 插件开发工作流

### 12.1 开发环境

```bash
# 1. 安装 MonoTools CLI 工具
npm install -g @monotools/cli

# 2. 创建插件模板
mtools create-plugin my-plugin --type=command

# 3. 进入开发模式（热重载）
cd my-plugin
mtools dev

# 4. 构建
mtools build

# 5. 打包为 .mtplugin 文件
mtools pack

# 6. 安装到 MonoTools
mtools install ./my-plugin.mtplugin
```

### 12.2 调试

- 前端: 使用浏览器 DevTools（Tauri 内置）
- 后端 WASM: 使用 `wasm-pack` + Chrome DevTools
- 日志: 查看 `%LOCALAPPDATA%/MonoTools/logs/`

---

## 13. 安全与沙箱

### 13.1 前端沙箱

- 插件前端代码运行在 WebView 中，与主页面共享上下文
- 通过 ES Module 隔离，禁止直接访问 `window` 上的其他插件
- 敏感 API（如文件系统）必须通过 IPC 调用后端，受权限控制

### 13.2 后端沙箱

- WASM 插件运行在 WASM 运行时中，内存安全
- Sidecar 插件以独立进程运行，通过 stdio/IPC 通信
- 禁止插件直接调用系统 API，必须通过 SDK 代理

### 13.3 签名与校验（未来）

- 插件市场要求开发者签名
- 可选启用插件签名验证，防止篡改