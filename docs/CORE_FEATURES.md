# MonoTools 核心功能设计

> 版本: v1.0  
> 日期: 2026-07-01

---

## 1. 全局搜索系统

### 1.1 搜索框交互设计

```
┌─────────────────────────────────────────────┐
│  ┌───────────────────────────────────────┐   │
│  │  🔍  report.docx              ⌘K     │   │  ← 输入框
│  └───────────────────────────────────────┘   │
│                                              │
│  ┌─────────────────────────────────────────┐  │
│  │ 📄 report.docx    ~/Documents/...   2d │  │  ← 结果项
│  │ 📄 report_2024... ~/Projects/...    5d │  │
│  │ 📁 Reports        ~/Work/...        1w │  │
│  └─────────────────────────────────────────┘  │
│                                              │
│  ─────────────────────────────────────────   │
│  文件搜索 (builtin:file-search)              │  ← 来源标识
└─────────────────────────────────────────────┘
```

**交互规则**:
- 唤出时自动聚焦输入框，光标闪烁
- 输入防抖 80ms，超过 2 字符触发搜索
- `↑/↓` 导航结果，`Enter` 执行默认动作，`Tab` 切换动作
- `Esc` 清空输入，再次 `Esc` 隐藏窗口
- 输入框右侧显示当前模式标识（如 `⌘K` 表示命令模式）

### 1.2 搜索模式与触发前缀

| 前缀 | 模式 | 说明 | 示例 |
|------|------|------|------|
| 无 | 全局搜索 | 聚合所有提供者结果 | `report` |
| `>` | 命令模式 | 执行快捷命令 | `>settings` |
| `=` | 计算器 | 数学表达式计算 | `=1+2*3` |
| `?` | 帮助 | 搜索帮助文档 | `?workspace` |

### 1.3 搜索提供者聚合

```rust
// SearchEngine 核心逻辑
pub async fn search(query: &str, context: &SearchContext) -> Vec<SearchResult> {
    let parsed = QueryParser::parse(query);
    
    // 1. 确定激活的提供者
    let providers = if let Some(prefix) = parsed.prefix {
        plugin_manager.get_providers_by_prefix(prefix)
    } else {
        plugin_manager.get_all_active_providers()
    };
    
    // 2. 并行查询所有提供者（超时 200ms）
    let futures = providers.iter().map(|p| {
        tokio::time::timeout(Duration::from_millis(200), p.search(&parsed))
    });
    
    let results = join_all(futures).await;
    
    // 3. 合并、去重、排序
    let merged = merge_results(results);
    sort_results(merged, &context.history)
}
```

### 1.4 文件搜索技术实现

基于 NTFS USN Journal，详见 REDESIGN.md 技术方案。

**索引构建**:

```rust
pub struct UsnIndexer {
    db_pool: SqlitePool,
    volumes: Vec<Volume>,
}

impl UsnIndexer {
    pub async fn build_index(&self) -> Result<IndexStats> {
        let mut handles = vec![];
        
        for vol in &self.volumes {
            let handle = tokio::task::spawn_blocking(|| {
                Self::index_volume(vol)
            });
            handles.push(handle);
        }
        
        let results = futures::future::join_all(handles).await;
        Ok(merge_stats(results))
    }
    
    fn index_volume(vol: &Volume) -> Result<IndexStats> {
        unsafe {
            // 1. 打开卷句柄
            let handle = CreateFileW(
                vol.path,
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                None,
            )?;
            
            // 2. 创建/查询 USN Journal
            let mut journal_data = USN_JOURNAL_DATA_V0::default();
            let mut bytes_returned = 0;
            DeviceIoControl(
                handle,
                FSCTL_QUERY_USN_JOURNAL,
                None, 0,
                Some(&mut journal_data as *mut _ as *mut c_void),
                size_of::<USN_JOURNAL_DATA_V0>() as u32,
                Some(&mut bytes_returned),
                None,
            )?;
            
            // 3. 枚举 MFT 记录
            let mut enum_buffer = vec![0u8; 1024 * 1024];
            let mut start_file_ref = 0u64;
            
            loop {
                let mut bytes_returned = 0;
                let result = DeviceIoControl(
                    handle,
                    FSCTL_ENUM_USN_DATA,
                    Some(&start_file_ref as *const _ as *const c_void),
                    size_of::<u64>() as u32,
                    Some(enum_buffer.as_mut_ptr() as *mut c_void),
                    enum_buffer.len() as u32,
                    Some(&mut bytes_returned),
                    None,
                );
                
                if result.is_err() { break; }
                
                // 4. 解析 USN 记录，构建路径，写入 SQLite
                Self::process_records(&enum_buffer[..bytes_returned as usize], vol)?;
                
                // 更新起始 FRN 继续枚举
                start_file_ref = *(enum_buffer.as_ptr() as *const u64);
            }
            
            Ok(IndexStats::new())
        }
    }
}
```

**数据库分表策略**:

```sql
-- 按文件名 ASCII 和分 41 个表 (0~40)
-- listN 存储 ASCII 和在 [N*100, (N+1)*100) 的文件
CREATE TABLE list0 (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE,
    path TEXT NOT NULL,
    parent_path TEXT NOT NULL,
    size INTEGER DEFAULT 0,
    modified_at INTEGER DEFAULT 0,
    is_dir INTEGER DEFAULT 0,
    ascii_sum INTEGER DEFAULT 0,
    pinyin TEXT  -- 中文文件名拼音索引
);

CREATE INDEX idx_list0_name ON list0(name);
CREATE INDEX idx_list0_pinyin ON list0(pinyin);

-- SQLite 性能优化
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = -262144;    -- 256MB
PRAGMA page_size = 65536;       -- 64KB
PRAGMA mmap_size = 268435456;   -- 256MB
```

**搜索查询**:

```rust
pub fn search_files(query: &str, limit: usize) -> Result<Vec<FileEntry>> {
    let ascii_sum = calc_ascii_sum(query);
    let table_id = (ascii_sum / 100).min(40);
    let table_name = format!("list{}", table_id);
    
    let conn = db_pool.get()?;
    
    // 1. 精确匹配
    let exact: Vec<FileEntry> = conn.prepare(&format!(
        "SELECT * FROM {} WHERE name = ?1 LIMIT ?2", table_name
    ))?.query_map([query, &limit.to_string()], row_mapper)?.collect();
    
    // 2. 前缀匹配
    let prefix: Vec<FileEntry> = conn.prepare(&format!(
        "SELECT * FROM {} WHERE name LIKE ?1 LIMIT ?2", table_name
    ))?.query_map([format!("{}%", query), limit.to_string()], row_mapper)?.collect();
    
    // 3. 包含匹配（跨所有相关表）
    let contains = search_all_tables(query, limit)?;
    
    // 4. 拼音匹配
    let pinyin = search_pinyin(query, limit)?;
    
    // 5. 合并去重排序
    Ok(merge_and_rank(exact, prefix, contains, pinyin))
}
```

### 1.5 应用搜索

扫描来源：
- `%APPDATA%/Microsoft/Windows/Start Menu/Programs/`
- `C:/ProgramData/Microsoft/Windows/Start Menu/Programs/`
- 注册表 `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths`
- 桌面快捷方式

存储结构：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,           // 唯一ID
    pub name: String,         // 显示名称
    pub exe_path: String,     // 可执行路径
    pub icon_path: Option<String>, // 图标路径
    pub description: Option<String>,
    pub keywords: Vec<String>, // 别名/关键词
    pub pinyin: Vec<String>,  // 拼音索引
    pub launch_count: u32,    // 启动次数（用于排序）
    pub last_accessed: Option<DateTime<Utc>>,
}
```

---

## 2. 工作区管理系统

### 2.1 功能定义

| 功能       | 说明                                     |
| ---------- | ---------------------------------------- |
| 保存工作区 | 捕获当前所有可见窗口的状态，序列化为快照 |
| 恢复工作区 | 按快照启动应用并恢复窗口位置/状态        |
| 工作区列表 | 展示所有保存的工作区，支持搜索           |
| 编辑工作区 | 修改名称、图标、删除单个应用快照         |
| 导入/导出  | JSON 格式的工作区配置迁移                |

### 2.2 数据模型

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,                    // UUID v4
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,          // Emoji 或图标路径
    pub color: Option<String>,         // 主题色标识
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_restored_at: Option<DateTime<Utc>>,
    pub apps: Vec<AppSnapshot>,
    pub auto_start: bool,              // 是否开机自动恢复
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub id: String,
    pub exe_path: String,             // 可执行文件绝对路径
    pub args: Vec<String>,            // 命令行参数
    pub working_dir: Option<String>,
    pub window_title: String,
    pub window_rect: WindowRect,
    pub window_state: WindowState,
    pub launch_order: u32,            // 启动顺序
    pub launch_delay_ms: u64,         // 启动延迟（等待前一个就绪）
    pub require_admin: bool,          // 是否需要管理员权限
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}
```

### 2.3 快照捕获实现

```rust
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Threading::*;
use windows::Win32::Storage::FileSystem::GetModuleFileNameExW;

pub struct WindowEnumerator;

impl WindowEnumerator {
    pub fn capture_workspace() -> Vec<AppSnapshot> {
        let mut snapshots = Vec::new();
        
        unsafe {
            EnumWindows(
                Some(Self::enum_callback),
                LPARAM(&mut snapshots as *mut _ as isize),
            );
        }
        
        snapshots
    }
    
    extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            // 过滤不可见窗口
            if !IsWindowVisible(hwnd).as_bool() {
                return BOOL(1);
            }
            
            // 获取窗口标题
            let mut title = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title);
            if len == 0 { return BOOL(1); }
            let title = String::from_utf16_lossy(&title[..len as usize]);
            
            // 过滤系统窗口
            if Self::is_system_window(&title) {
                return BOOL(1);
            }
            
            // 获取进程 ID
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            
            // 获取窗口位置和状态
            let mut placement = WINDOWPLACEMENT::default();
            placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
            GetWindowPlacement(hwnd, &mut placement);
            
            let rect = placement.rcNormalPosition;
            
            // 获取 EXE 路径
            let exe_path = Self::get_process_path(pid);
            
            // 获取命令行参数（通过 WMI）
            let args = Self::get_command_line(pid);
            
            let snapshots = &mut *(lparam.0 as *mut Vec<AppSnapshot>);
            snapshots.push(AppSnapshot {
                id: uuid::Uuid::new_v4().to_string(),
                exe_path,
                args,
                working_dir: None,
                window_title: title,
                window_rect: WindowRect {
                    x: rect.left,
                    y: rect.top,
                    width: rect.right - rect.left,
                    height: rect.bottom - rect.top,
                },
                window_state: match placement.showCmd {
                    SW_SHOWMINIMIZED => WindowState::Minimized,
                    SW_SHOWMAXIMIZED => WindowState::Maximized,
                    _ => WindowState::Normal,
                },
                launch_order: 0,
                launch_delay_ms: 500,
                require_admin: false,
            });
            
            BOOL(1)
        }
    }
    
    unsafe fn get_process_path(pid: u32) -> String {
        let handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            pid,
        ).unwrap_or_default();
        
        let mut path = [0u16; 512];
        let len = GetModuleFileNameExW(Some(handle), None, &mut path);
        String::from_utf16_lossy(&path[..len as usize])
    }
    
    fn get_command_line(pid: u32) -> Vec<String> {
        // 通过 WMI 查询 Win32_Process
        use wmi::{COMLibrary, WMIConnection};
        let com = COMLibrary::new().ok()?;
        let wmi = WMIConnection::new(com).ok()?;
        
        let query = format!("SELECT CommandLine FROM Win32_Process WHERE ProcessId = {}", pid);
        // 解析 CommandLine 字符串为参数数组
        vec![]
    }
    
    fn is_system_window(title: &str) -> bool {
        let system_titles = ["Program Manager", "Windows Input Experience", "Search"];
        system_titles.contains(&title)
    }
}
```

### 2.4 工作区恢复实现

```rust
pub async fn restore_workspace(workspace: &Workspace) -> RestoreReport {
    let mut report = RestoreReport::new();
    
    // 按 launch_order 排序
    let mut apps = workspace.apps.clone();
    apps.sort_by_key(|a| a.launch_order);
    
    for app in &apps {
        // 1. 检查是否已运行
        if is_process_running(&app.exe_path) {
            // 尝试找到现有窗口并调整位置
            if let Some(hwnd) = find_window_by_title(&app.window_title) {
                set_window_state(hwnd, &app.window_rect, &app.window_state);
                report.adjusted.push(app.id.clone());
            } else {
                report.skipped.push(app.id.clone());
            }
            continue;
        }
        
        // 2. 启动应用
        match start_application(app) {
            Ok(_) => {
                // 3. 等待窗口出现（轮询，超时 10s）
                match wait_for_window(&app.window_title, Duration::from_secs(10)).await {
                    Ok(hwnd) => {
                        // 4. 恢复窗口位置
                        set_window_state(hwnd, &app.window_rect, &app.window_state);
                        report.restored.push(app.id.clone());
                    }
                    Err(e) => {
                        report.failed.push((app.id.clone(), format!("Window not found: {}", e)));
                    }
                }
            }
            Err(e) => {
                report.failed.push((app.id.clone(), format!("Launch failed: {}", e)));
            }
        }
        
        // 5. 等待间隔
        if app.launch_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(app.launch_delay_ms)).await;
        }
    }
    
    report
}

fn start_application(app: &AppSnapshot) -> Result<std::process::Child> {
    let mut cmd = std::process::Command::new(&app.exe_path);
    
    if let Some(dir) = &app.working_dir {
        cmd.current_dir(dir);
    }
    
    for arg in &app.args {
        cmd.arg(arg);
    }
    
    // 如果原窗口是最大化，传递相应启动参数
    if app.window_state == WindowState::Maximized {
        // 某些应用支持 --maximized 参数
    }
    
    Ok(cmd.spawn()?)
}
```

---

## 3. 全局快捷键系统

### 3.1 快捷键注册

```rust
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::Foundation::HWND;

pub struct HotkeyService {
    registered: HashMap<String, u32>, // id -> atom
}

impl HotkeyService {
    pub fn register(&mut self, hwnd: HWND, id: &str, modifiers: u32, vk: u32) -> Result<()> {
        let atom = GlobalAddAtomW(&HSTRING::from(id))?;
        
        unsafe {
            RegisterHotKey(hwnd, atom as i32, modifiers, vk)?;
        }
        
        self.registered.insert(id.to_string(), atom);
        Ok(())
    }
    
    pub fn unregister(&mut self, hwnd: HWND, id: &str) -> Result<()> {
        if let Some(atom) = self.registered.remove(id) {
            unsafe {
                UnregisterHotKey(hwnd, atom as i32)?;
                GlobalDeleteAtom(atom)?;
            }
        }
        Ok(())
    }
    
    pub fn handle_message(&self, wparam: WPARAM, lparam: LPARAM) -> Option<String> {
        let id = wparam.0 as u32;
        self.registered.iter()
            .find(|(_, v)| **v == id)
            .map(|(k, _)| k.clone())
    }
}
```

### 3.2 默认快捷键配置

```json
{
  "hotkeys": {
    "search.toggle": {
      "key": "Space",
      "modifiers": ["Alt"],
      "global": true,
      "description": "唤出/隐藏搜索框"
    },
    "search.files": {
      "key": "F",
      "modifiers": ["Alt", "Shift"],
      "global": true,
      "description": "直接唤出并切换到文件搜索"
    },
    "workspace.quick_save": {
      "key": "S",
      "modifiers": ["Alt", "Shift"],
      "global": true,
      "description": "快速保存当前工作区"
    }
  }
}
```

### 3.3 消息循环集成

```rust
// 在 Tauri 主循环中处理 WM_HOTKEY
fn main() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            let hwnd = window.hwnd()? as HWND;
            
            let mut hotkey = HotkeyService::new();
            hotkey.register(hwnd, "search.toggle", MOD_ALT, VK_SPACE.0 as u32)?;
            
            // 设置 Windows 消息钩子
            window.listen("tauri://focus", |_| {});
            
            Ok(())
        })
        .build(tauri::generate_context!())?;
    
    app.run(|_app_handle, event| {
        if let tauri::RunEvent::WindowEvent { event: WindowEvent::Focused(false), .. } = event {
            // 失焦自动隐藏
            hide_search_window();
        }
    });
}
```

---

## 4. 设置与配置系统

### 4.1 配置分层

| 层级       | 存储位置                                        | 说明               |
| ---------- | ----------------------------------------------- | ------------------ |
| 默认配置   | 代码内嵌                                        | 硬编码默认值       |
| 用户配置   | `%LOCALAPPDATA%/MonoTools/config/settings.json` | 用户修改覆盖       |
| 插件配置   | `%LOCALAPPDATA%/MonoTools/config/plugins.json`  | 各插件独立配置     |
| 运行时配置 | 内存                                            | 临时状态，不持久化 |

### 4.2 核心配置项

```typescript
interface MonoToolsConfig {
  // 通用
  language: 'zh-CN' | 'en-US' | 'system'
  startup: boolean           // 开机自启
  silentStart: boolean        // 静默启动（不显示窗口）
  
  // 搜索框
  search: {
    windowWidth: number       // 默认 800
    windowHeight: number      // 默认 520
    position: 'center' | 'cursor' | { x: number, y: number }
    blurOnLostFocus: boolean  // 失焦自动隐藏
    debounceMs: number        // 输入防抖，默认 80
    maxResults: number        // 最大结果数，默认 50
    showRecentOnEmpty: boolean // 空输入时显示最近使用
  }
  
  // 快捷键
  hotkeys: Record<string, HotkeyConfig>
  
  // 搜索
  searchProviders: {
    order: string[]          // 提供者排序
    disabled: string[]       // 禁用的提供者
  }
  
  // 文件搜索
  fileSearch: {
    indexedVolumes: string[] // 索引的磁盘卷
    excludePaths: string[]   // 排除路径
    maxIndexSizeMB: number   // 索引大小限制
  }
  
  // 工作区
  workspace: {
    autoSaveInterval: number // 自动保存间隔（分钟），0 为关闭
    confirmBeforeRestore: boolean
  }
  
  // 主题
  theme: {
    active: string          // 当前主题插件 ID
    mode: 'dark' | 'light' | 'system'
    accentColor: string     // 强调色
  }
}
```

### 4.3 配置热重载

```rust
#[tauri::command]
fn update_config(path: String, value: Value, app: AppHandle) -> Result<()> {
    let mut config = CONFIG.write().map_err(|_| "Lock poisoned")?;
    config.set_by_path(&path, value.clone())?;
    config.save()?; // 持久化到 JSON
    
    // 广播配置变更事件
    app.emit("config:changed", json!({
        "path": path,
        "value": value
    }))?;
    
    Ok(())
}
```

---

## 5. 系统托盘

### 5.1 托盘菜单

```
┌─────────────────────┐
│  MonoTools          │  ← 标题（不可点击）
├─────────────────────┤
│  显示搜索框     Alt+Space │
│  工作区管理           │
│  插件管理             │
│  设置                 │
├─────────────────────┤
│  主题 ▸               │
│    ├─ 深色 (默认)     │
│    ├─ 浅色            │
│    └─ 跟随系统        │
├─────────────────────┤
│  开机自启      [✓]    │
├─────────────────────┤
│  退出                 │
└─────────────────────┘
```

### 5.2 实现

```rust
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton};

fn setup_tray(app: &mut tauri::App) -> Result<()> {
    let tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("MonoTools")
        .menu(&tauri::menu::Menu::new())
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "show" => show_search_window(app),
                "workspaces" => open_workspace_manager(app),
                "plugins" => open_plugin_manager(app),
                "settings" => open_settings(app),
                "autostart" => toggle_autostart(app),
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                show_search_window(tray.app_handle());
            }
        })
        .build(app)?;
    
    Ok(())
}
```

---

## 6. 静默启动机制

### 6.1 启动模式

| 模式     | 行为                                 |
| -------- | ------------------------------------ |
| 静默启动 | 无窗口弹出，仅托盘图标，后台构建索引 |
| 正常启动 | 显示搜索框（调试用）                 |
| 首次启动 | 显示欢迎向导，引导设置快捷键和索引   |

### 6.2 开机自启

```rust
use tauri_plugin_autostart::ManagerExt;

fn setup_autostart(app: &mut tauri::App) -> Result<()> {
    let autostart_manager = app.autolaunch();
    
    // 根据配置设置
    if config.startup {
        autostart_manager.enable()?;
    } else {
        autostart_manager.disable()?;
    }
    
    Ok(())
}
```

### 6.3 单实例约束

```rust
use tauri::Manager;

fn ensure_single_instance() {
    // 使用命名互斥体确保单实例
    // 若已有实例运行，则向已有实例发送消息并退出
}
```