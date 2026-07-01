# MonoTools API 接口与命令指令设计

> 版本: v1.0  
> 日期: 2026-07-01

---

## 1. 设计哲学

**一切功能皆命令**。MonoTools 的所有能力——无论是搜索文件、保存工作区、切换主题——都抽象为统一的 `Command` 对象。这一设计确保：

1. **CLI 与 GUI 同源**: 命令行和图形界面调用完全相同的后端逻辑
2. **插件可扩展**: 插件通过注册新命令扩展功能
3. **可测试**: 每个命令是独立的单元，易于单元测试
4. **可审计**: 所有操作通过命令日志记录

---

## 2. 命令指令规范

### 2.1 命令格式

```
{namespace}:{action} [--flag] [key=value] [...]
```

| 部分 | 说明 | 示例 |
|------|------|------|
| `namespace` | 功能命名空间 | `search`, `workspace`, `plugin`, `theme` |
| `action` | 具体操作 | `files`, `save`, `list`, `set` |
| `--flag` | 布尔开关 | `--force`, `--json`, `--silent` |
| `key=value` | 键值参数 | `query=report.docx`, `limit=50` |

### 2.2 命令示例

```bash
# 搜索文件
monotools search:files query="report.docx" limit=50

# 保存工作区
monotools workspace:save name="开发环境" --auto-start

# 列出所有插件
monotools plugin:list --enabled-only

# 切换主题
monotools theme:set id="builtin:default-theme" mode=dark

# 打开设置面板
monotools ui:open panel=settings

# 重新加载插件
monotools plugin:reload id="com.example.my-plugin"
```

### 2.3 命令响应格式

```json
{
  "success": true,
  "command": "search:files",
  "elapsed_ms": 45,
  "data": { ... },
  "error": null,
  "meta": {
    "page": 1,
    "total": 128,
    "limit": 50
  }
}
```

---

## 3. 命令总线实现

### 3.1 Rust 核心结构

```rust
// src/commands/bus.rs

use std::collections::HashMap;
use anyhow::Result;
use serde_json::Value;

/// 命令定义
pub struct Command {
    pub namespace: String,
    pub action: String,
    pub args: HashMap<String, Value>,
    pub flags: Vec<String>,
}

/// 命令处理器 trait
#[async_trait::async_trait]
pub trait CommandHandler: Send + Sync {
    async fn execute(&self, cmd: &Command, ctx: &CommandContext) -> Result<CommandResponse>;
    fn validate(&self, cmd: &Command) -> Result<()>;
}

/// 命令响应
#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub success: bool,
    pub data: Value,
    pub error: Option<String>,
    pub elapsed_ms: u64,
}

/// 命令上下文
pub struct CommandContext {
    pub app_handle: tauri::AppHandle,
    pub window: Option<tauri::WebviewWindow>,
    pub plugin_manager: Arc<RwLock<PluginManager>>,
    pub config: Arc<RwLock<ConfigStore>>,
    pub caller: CallerType, // CLI | IPC | Plugin
}

pub enum CallerType {
    Cli,
    Ipc,
    Plugin(String), // plugin_id
}

/// 命令总线
pub struct CommandBus {
    handlers: HashMap<String, Box<dyn CommandHandler>>,
}

impl CommandBus {
    pub fn new() -> Self {
        Self { handlers: HashMap::new() }
    }
    
    pub fn register(&mut self, namespace: &str, action: &str, handler: Box<dyn CommandHandler>) {
        let key = format!("{}:{}", namespace, action);
        self.handlers.insert(key, handler);
    }
    
    pub async fn execute(&self, cmd: Command, ctx: CommandContext) -> Result<CommandResponse> {
        let key = format!("{}:{}", cmd.namespace, cmd.action);
        let handler = self.handlers.get(&key)
            .ok_or_else(|| anyhow::anyhow!("Unknown command: {}", key))?;
        
        let start = Instant::now();
        handler.validate(&cmd)?;
        let result = handler.execute(&cmd, &ctx).await;
        let elapsed = start.elapsed().as_millis() as u64;
        
        match result {
            Ok(data) => Ok(CommandResponse {
                success: true,
                data: serde_json::to_value(data)?,
                error: None,
                elapsed_ms: elapsed,
            }),
            Err(e) => Ok(CommandResponse {
                success: false,
                data: Value::Null,
                error: Some(e.to_string()),
                elapsed_ms: elapsed,
            }),
        }
    }
}
```

### 3.2 命令解析器

```rust
// src/commands/parser.rs

pub struct CommandParser;

impl CommandParser {
    pub fn parse(input: &str) -> Result<Command> {
        let mut parts = input.split_whitespace();
        let head = parts.next().ok_or_else(|| anyhow::anyhow!("Empty command"))?;
        
        // 解析 namespace:action
        let (namespace, action) = if head.contains(':') {
            let mut split = head.split(':');
            (split.next().unwrap().to_string(), split.next().unwrap().to_string())
        } else {
            // 简写形式，默认 namespace 为 core
            ("core".to_string(), head.to_string())
        };
        
        let mut args = HashMap::new();
        let mut flags = Vec::new();
        
        for part in parts {
            if part.starts_with("--") {
                flags.push(part.trim_start_matches("--").to_string());
            } else if part.contains('=') {
                let mut split = part.splitn(2, '=');
                let key = split.next().unwrap().to_string();
                let value = Self::parse_value(split.next().unwrap_or(""));
                args.insert(key, value);
            } else {
                // 位置参数，默认放入 "arg" 或根据上下文推断
                args.insert("arg".to_string(), Value::String(part.to_string()));
            }
        }
        
        Ok(Command { namespace, action, args, flags })
    }
    
    fn parse_value(s: &str) -> Value {
        if let Ok(n) = s.parse::<i64>() {
            Value::Number(n.into())
        } else if let Ok(b) = s.parse::<bool>() {
            Value::Bool(b)
        } else {
            Value::String(s.to_string())
        }
    }
}
```

---

## 4. 核心命令清单

### 4.1 搜索命令 (`search:*`)

| 命令                | 参数                               | 说明                   |
| ------------------- | ---------------------------------- | ---------------------- |
| `search:files`      | `query`, `limit`, `offset`, `sort` | 全局文件搜索           |
| `search:apps`       | `query`, `limit`                   | 应用搜索               |
| `search:workspaces` | `query`                            | 工作区搜索             |
| `search:all`        | `query`, `limit`                   | 聚合搜索（所有提供者） |
| `search:providers`  | —                                  | 列出所有搜索提供者     |
| `search:history`    | `limit`                            | 获取搜索历史           |

### 4.2 工作区命令 (`workspace:*`)

| 命令                | 参数                                          | 说明              |
| ------------------- | --------------------------------------------- | ----------------- |
| `workspace:save`    | `name`, `description`, `icon`, `--auto-start` | 保存当前工作区    |
| `workspace:restore` | `id`, `--force`                               | 恢复工作区        |
| `workspace:list`    | —                                             | 列出所有工作区    |
| `workspace:get`     | `id`                                          | 获取工作区详情    |
| `workspace:delete`  | `id`, `--force`                               | 删除工作区        |
| `workspace:export`  | `id`, `path`                                  | 导出工作区为 JSON |
| `workspace:import`  | `path`                                        | 导入工作区        |
| `workspace:edit`    | `id`, `name?`, `description?`                 | 编辑工作区元信息  |

### 4.3 插件命令 (`plugin:*`)

| 命令               | 参数                               | 说明              |
| ------------------ | ---------------------------------- | ----------------- |
| `plugin:list`      | `--enabled-only`, `--builtin-only` | 列出插件          |
| `plugin:install`   | `path` 或 `url`                    | 安装插件          |
| `plugin:uninstall` | `id`, `--force`                    | 卸载插件          |
| `plugin:enable`    | `id`                               | 启用插件          |
| `plugin:disable`   | `id`                               | 禁用插件          |
| `plugin:reload`    | `id`                               | 热重载插件        |
| `plugin:config`    | `id`, `key?`, `value?`             | 获取/设置插件配置 |
| `plugin:logs`      | `id`, `--tail`                     | 查看插件日志      |

### 4.4 主题命令 (`theme:*`)

| 命令            | 参数          | 说明               |
| --------------- | ------------- | ------------------ |
| `theme:list`    | —             | 列出可用主题       |
| `theme:set`     | `id`, `mode?` | 设置当前主题       |
| `theme:get`     | —             | 获取当前主题信息   |
| `theme:preview` | `id`          | 预览主题（不保存） |

### 4.5 系统命令 (`system:*`)

| 命令                 | 参数                                  | 说明               |
| -------------------- | ------------------------------------- | ------------------ |
| `system:shutdown`    | `--restart`, `--sleep`, `--hibernate` | 电源操作           |
| `system:lock`        | —                                     | 锁定工作站         |
| `system:empty-trash` | —                                     | 清空回收站         |
| `system:open`        | `path`                                | 用默认程序打开路径 |

### 4.6 UI 命令 (`ui:*`)

| 命令        | 参数               | 说明           |
| ----------- | ------------------ | -------------- |
| `ui:show`   | —                  | 显示搜索框     |
| `ui:hide`   | —                  | 隐藏搜索框     |
| `ui:toggle` | —                  | 切换搜索框显隐 |
| `ui:open`   | `panel`            | 打开指定面板   |
| `ui:notify` | `message`, `type?` | 显示通知       |

### 4.7 配置命令 (`config:*`)

| 命令           | 参数            | 说明             |
| -------------- | --------------- | ---------------- |
| `config:get`   | `key?`          | 获取配置项       |
| `config:set`   | `key`, `value`  | 设置配置项       |
| `config:reset` | `key?`, `--all` | 重置配置         |
| `config:path`  | —               | 显示配置文件路径 |

---

## 5. Tauri IPC 接口

### 5.1 前端调用后端 (`invoke`)

```typescript
// 通用调用封装
async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<CommandResponse<T>> {
  return await invoke('execute_command', {
    input: `${command} ${serializeArgs(args)}`
  })
}

// 使用示例
const result = await invokeCommand('search:files', {
  query: 'report.docx',
  limit: 50
})
```

### 5.2 Rust 命令处理器

```rust
#[tauri::command]
async fn execute_command(
    input: String,
    app_handle: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<CommandResponse, String> {
    let cmd = CommandParser::parse(&input).map_err(|e| e.to_string())?;
    
    let ctx = CommandContext {
        app_handle,
        window: Some(window),
        plugin_manager: state.plugin_manager.clone(),
        config: state.config.clone(),
        caller: CallerType::Ipc,
    };
    
    state.command_bus.execute(cmd, ctx).await.map_err(|e| e.to_string())
}
```

### 5.3 后端推送前端 (`emit`)

```rust
// 索引更新事件
app_handle.emit("search:index_updated", json!({
    "indexed_files": 1000000,
    "elapsed_seconds": 12
}))?;

// 插件加载事件
app_handle.emit("plugin:loaded", json!({
    "id": "com.example.plugin",
    "name": "Example Plugin"
}))?;

// 主题切换事件
app_handle.emit("theme:changed", json!({
    "theme_id": "builtin:default-theme",
    "mode": "dark"
}))?;
```

### 5.4 前端事件监听

```typescript
import { listen } from '@tauri-apps/api/event'

// 监听索引更新
listen('search:index_updated', (event) => {
  console.log(`索引完成: ${event.payload.indexed_files} 个文件`)
})

// 监听主题切换
listen('theme:changed', (event) => {
  applyTheme(event.payload.css_variables)
})
```

---

## 6. 插件 API 接口

### 6.1 前端插件 SDK

```typescript
// @monotools/plugin-sdk

interface PluginContext {
  // 标识
  id: string
  version: string
  
  // 搜索
  search: {
    registerTrigger(prefix: string, options: TriggerOptions): void
    unregisterTrigger(prefix: string): void
    registerProvider(provider: SearchProvider): void
  }
  
  // 命令
  commands: {
    register(command: CommandRegistration): void
    unregister(commandId: string): void
  }
  
  // UI
  ui: {
    showNotification(message: string, type?: NotificationType): void
    showDialog(options: DialogOptions): Promise<DialogResult>
    registerView(route: string, component: Component): void
    updateResultList(items: SearchResult[]): void
  }
  
  // 存储
  storage: {
    get<T>(key: string): Promise<T | undefined>
    set<T>(key: string, value: T): Promise<void>
    delete(key: string): Promise<void>
  }
  
  // 配置
  config: {
    get(): Promise<Record<string, unknown>>
    getByKey<T>(key: string): Promise<T | undefined>
    set(key: string, value: unknown): Promise<void>
    onChange(handler: (config: Record<string, unknown>) => void): void
  }
  
  // 剪贴板
  clipboard: {
    readText(): Promise<string>
    writeText(text: string): Promise<void>
    readImage(): Promise<Uint8Array | null>
  }
  
  // 事件
  events: {
    emit(event: string, data: unknown): void
    on(event: string, handler: (data: unknown) => void): void
    off(event: string, handler: (data: unknown) => void): void
  }
  
  // 后端调用
  invoke(command: string, args?: Record<string, unknown>): Promise<unknown>
  
  // 日志
  log(level: 'debug' | 'info' | 'warn' | 'error', message: string): void
}

// 类型定义
interface SearchResult {
  id: string
  title: string
  subtitle?: string
  icon?: string
  iconComponent?: string
  source: string
  pluginId: string
  score: number
  actions: ResultAction[]
}

interface ResultAction {
  id: string
  title: string
  shortcut?: string
  handler?: () => void | Promise<void>
}

interface CommandRegistration {
  id: string
  title: string
  description?: string
  shortcut?: string
  handler: (args: Record<string, unknown>) => Promise<unknown>
}
```

### 6.2 后端插件 SDK (Rust)

```rust
// monotools-plugin-sdk

pub struct PluginContext {
    pub id: String,
    pub app_handle: tauri::AppHandle,
}

pub trait Plugin: Send + Sync {
    fn activate(&mut self, ctx: &PluginContext) -> Result<()>;
    fn deactivate(&mut self, ctx: &PluginContext) -> Result<()>;
    
    fn on_search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        Ok(vec![])
    }
    
    fn on_execute(&self, item: &SearchResult) -> Result<ExecutionResult> {
        Ok(ExecutionResult::default())
    }
    
    fn handle_command(&self, cmd: &str, args: &Value) -> Result<Value> {
        Err(anyhow::anyhow!("Command not supported"))
    }
}

pub struct SearchQuery {
    pub raw: String,
    pub prefix: Option<String>,
    pub tokens: Vec<String>,
    pub is_empty: bool,
}

pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub source: String,
    pub score: f32,
    pub action: String,
}
```

---

## 7. CLI 设计

### 7.1 CLI 入口

```rust
// src/cli.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "monotools")]
#[command(about = "MonoTools - 高效桌面启动器")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// 静默模式（不输出日志）
    #[arg(short, long)]
    silent: bool,
    
    /// 输出 JSON 格式
    #[arg(long)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// 搜索相关
    Search {
        #[command(subcommand)]
        action: SearchCommands,
    },
    /// 工作区管理
    Workspace {
        #[command(subcommand)]
        action: WorkspaceCommands,
    },
    /// 插件管理
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
    },
    /// 启动守护进程（正常启动）
    Daemon,
    /// 直接执行命令字符串
    Exec {
        command: String,
    },
}

#[derive(Subcommand)]
enum SearchCommands {
    Files {
        query: String,
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    Apps {
        query: String,
    },
}
```

### 7.2 CLI 使用示例

```bash
# 启动守护进程（正常方式）
monotools daemon

# 搜索文件并输出 JSON
monotools search files "report.docx" --limit 20 --json

# 保存工作区
monotools workspace save "开发环境" --auto-start

# 列出插件
monotools plugin list --enabled-only

# 直接执行任意命令
monotools exec "theme:set id=builtin:default-theme mode=dark"

# 隐藏搜索框
monotools exec "ui:hide"
```

### 7.3 CLI 与守护进程通信

当守护进程已运行时，CLI 通过命名管道/IPC 发送命令：

```rust
// CLI 检测到守护进程运行中
if is_daemon_running() {
    // 通过 IPC 发送命令到守护进程
    let client = IpcClient::connect("monotools")?;
    let response = client.send_command(&cli.command)?;
    println!("{}", format_response(response, cli.json));
} else {
    // 启动守护进程并执行命令
    start_daemon()?;
}
```

---

## 8. 错误码规范

| 错误码              | 说明         | HTTP 类比 |
| ------------------- | ------------ | --------- |
| `OK`                | 成功         | 200       |
| `INVALID_COMMAND`   | 未知命令     | 400       |
| `INVALID_ARGUMENT`  | 参数错误     | 400       |
| `NOT_FOUND`         | 资源不存在   | 404       |
| `PERMISSION_DENIED` | 权限不足     | 403       |
| `PLUGIN_ERROR`      | 插件执行错误 | 500       |
| `SYSTEM_ERROR`      | 系统调用失败 | 500       |
| `TIMEOUT`           | 操作超时     | 504       |
| `BUSY`              | 服务忙       | 503       |

```json
{
  "success": false,
  "error": {
    "code": "PERMISSION_DENIED",
    "message": "Plugin 'com.example.x' requires 'system:exec' permission",
    "details": { "required": "system:exec", "granted": ["ui:show"] }
  }
}
```