# MonoTools 重构详细设计文档

> 版本: v1.0  
> 日期: 2026-06-29  
> 作者: MonoKelvin  
> 状态: 设计阶段

---

## 目录

1. [项目概述](#1-项目概述)
2. [Tauri + Rust 技术路线可行性分析](#2-tauri--rust-技术路线可行性分析)
3. [系统架构设计](#3-系统架构设计)
4. [核心功能设计](#4-核心功能设计)
5. [UI 设计方案](#5-ui-设计方案)
6. [技术选型清单](#6-技术选型清单)
7. [项目目录结构](#7-项目目录结构)
8. [开发阶段规划](#8-开发阶段规划)
9. [风险评估与应对](#9-风险评估与应对)
10. [附录](#10-附录)

---

## 1. 项目概述

### 1.1 项目定位

MonoTools 是一款面向 Windows 平台（后续支持 macOS）的**高性能桌面启动器与效率工具**，从 ZTools 项目 fork 并进行全面重构。核心目标是打造一个集**应用启动、全局文件搜索、工作区管理**于一体的生产力工具，以插件化架构支持功能扩展。

### 1.2 目标用户画像

| 用户类型 | 使用场景 | 核心需求 |
|---------|---------|---------|
| 开发者 | 快速启动 IDE、终端、项目文件夹 | 快速搜索、工作区切换 |
| 设计师 | 管理多个设计工具和素材文件 | 文件搜索、窗口布局管理 |
| 知识工作者 | 多任务并行，频繁切换上下文 | 工作区保存/恢复、快捷启动 |
| 效率爱好者 | 追求键盘驱动的工作流 | 全局快捷键、Spotlight 式搜索 |

### 1.3 核心竞争力

1. **极速文件搜索** — 基于 NTFS USN Journal 的原生索引，100万文件 < 15秒
2. **智能工作区管理** — 一键保存/恢复桌面状态，无缝切换工作上下文
3. **插件化架构** — 开放的插件系统，功能可无限扩展
4. **轻量高性能** — Tauri + Rust 技术栈，内存占用仅为 Electron 的 1/4
5. **简约高级 UI** — 毛玻璃效果、流畅动画、Spotlight 式交互

### 1.4 与 ZTools 的关系

MonoTools 保持与 ZTools 的核心设计思想一致（插件系统、搜索驱动的交互），但在技术栈、功能范围和 UI 设计上进行全面升级：

| 维度 | ZTools | MonoTools |
|------|--------|-----------|
| 技术栈 | Electron + Vue 3 | Tauri + Rust + Vue 3 |
| 文件搜索 | Fuse.js 模糊搜索 | NTFS USN Journal 索引 |
| 工作区 | 不支持 | 完整支持 |
| 包体积 | ~150MB | ~5-10MB |
| 内存占用 | 200-400MB | 50-100MB |

---

## 2. Tauri + Rust 技术路线可行性分析

### 2.1 性能对比

| 指标 | Electron (当前) | Tauri + Rust (目标) | 提升幅度 |
|------|----------------|---------------------|---------|
| 安装包体积 | ~150MB | ~5-10MB | **95% ↓** |
| 空闲内存 | 200-400MB | 30-80MB | **75% ↓** |
| 冷启动时间 | 2-5秒 | 0.3-1秒 | **80% ↓** |
| CPU 空闲占用 | 1-3% | < 0.5% | **80% ↓** |
| 渲染引擎 | Chromium (自带) | WebView2 (系统内置) | — |

**分析**: Tauri 使用操作系统自带的 WebView2 (Windows 10 1803+ 内置)，不需要捆绑 Chromium，这是体积和内存大幅降低的根本原因。

### 2.2 系统集成能力

| 能力 | Electron 方案 | Tauri + Rust 方案 | 优势方 |
|------|-------------|-------------------|--------|
| Win32 API 调用 | Node-API C++ addon | `windows-rs` crate 原生绑定 | **Tauri** — 类型安全、无 GC |
| NTFS USN Journal | C++ DLL 通过 Node-API | Rust 直接调用 `DeviceIoControl` | **Tauri** — 无中间层 |
| 进程枚举 | C++ addon 或 PowerShell | `windows-rs` 调用 `EnumProcesses` | **Tauri** — 更简洁 |
| 窗口管理 | C++ addon | `windows-rs` 调用 Win32 API | **Tauri** — 原生安全 |
| 注册表访问 | `winreg` npm 包 | `windows-rs` 或 `winreg` crate | 相当 |
| 全局快捷键 | `uiohook-napi` (原生) | Tauri 内置支持 | **Tauri** — 无需额外依赖 |
| 系统托盘 | Electron 内置 | Tauri 内置支持 | 相当 |
| 文件监控 | `chokidar` | `notify` crate | 相当 |
| 剪贴板 | C++ addon | `arboard` crate 或 Tauri 插件 | 相当 |

**结论**: Tauri + Rust 在系统集成方面**全面优于或持平** Electron 方案，特别是在 Win32 API 调用方面，`windows-rs` crate 提供了完整的类型安全绑定，无需维护 C++ addon。

### 2.3 插件系统对比

| 维度 | MonoTools 插件系统 | Tauri 插件系统 |
|------|---------------|---------------|
| 后端插件 | Node.js 模块 | Rust 原生插件 (`.so`/`.dll`) |
| 前端插件 | Vue 组件 + WebContentsView | Vue 组件 + WebView |
| IPC 机制 | Electron IPC (无限制) | Tauri invoke (权限控制) |
| 安全模型 | 无沙箱 | Capability-based 权限模型 |
| 插件市场 | npm 生态 | 自建市场 + crates.io |

**策略**: 采用双层插件架构：
- **Rust 插件**: 高性能系统级插件（文件搜索提供者、系统集成等）
- **前端插件**: UI 插件保持与 MonoTools 兼容的 API 设计

### 2.4 跨平台能力

| 平台 | 支持状态 | WebView 引擎 |
|------|---------|-------------|
| Windows 10+ | ✅ 主要目标 | WebView2 (Edge Chromium) |
| macOS | ✅ 支持 (Phase 2) | WebKit (原生) |
| Linux | ⚠️ 实验性 | WebKitGTK |
| iOS/Android | 🔮 未来考虑 | Tauri 2.0 移动端支持 |

### 2.5 开发体验

| 维度 | 评估 |
|------|------|
| 前端开发 | **无变化** — 仍然使用 Vue 3 + TypeScript + Vite |
| 后端开发 | **学习曲线** — Rust 语法较复杂，但编译器反馈优秀 |
| 构建工具 | **成熟** — Cargo (Rust) + Vite (前端) + Tauri CLI |
| 调试体验 | **良好** — Tauri DevTools + Rust debugger (lldb) |
| 热更新 | **支持** — 前端 HMR + Rust 代码重编译 |
| 测试框架 | **成熟** — Rust: `#[test]` + `cargo test`，前端: Vitest |

### 2.6 生态和社区

- **Tauri**: GitHub 80k+ stars，活跃开发，2.0 已正式发布
- **Rust crates**: crates.io 拥有 14 万+ 包，覆盖 Windows API、数据库、网络等
- **关键 crate 可用性**:
  - `windows` (Microsoft 官方) — Win32 API 完整绑定 ✅
  - `rusqlite` — SQLite 绑定，成熟稳定 ✅
  - `tokio` — 异步运行时，生态完善 ✅
  - `serde` — 序列化框架，事实标准 ✅
  - `notify` — 跨平台文件监控 ✅

### 2.7 风险和挑战

| 风险 | 严重程度 | 应对措施 |
|------|---------|---------|
| Rust 学习曲线 | 中 | 团队学习计划，从简单模块开始 |
| WebView2 兼容性 | 低 | Win10 1803+ 内置，覆盖率 > 95% |
| 原生模块迁移 | 中 | 分阶段迁移，优先核心功能 |
| 插件生态重建 | 中 | 保持前端 API 兼容性 |
| macOS 文件搜索 | 高 | Windows 优先，macOS 延后 |

### 2.8 结论

**✅ 推荐采用 Tauri 2.0 + Rust 技术路线**

理由：
1. 性能优势巨大（体积、内存、启动速度）
2. Rust 对 Win32 API 的原生支持完美匹配文件搜索和系统集成需求
3. 前端技术栈（Vue 3 + Vite）完全保留，迁移成本低
4. Tauri 2.0 已经成熟，生态快速发展
5. 安全模型（Capability-based）优于 Electron 的无限制 IPC

---

## 3. 系统架构设计

### 3.1 整体架构

```
┌──────────────────────────────────────────────────────────────┐
│                    Frontend Layer (WebView2)                  │
│                  Vue 3 + TypeScript + Vite                    │
│                                                              │
│  ┌────────────┐  ┌────────────┐  ┌────────────────────────┐  │
│  │ SearchBar  │  │ Workspace  │  │   Plugin / Settings    │  │
│  │ (Spotlight │  │  Manager   │  │      Panel             │  │
│  │  Style)    │  │            │  │                        │  │
│  └────────────┘  └────────────┘  └────────────────────────┘  │
├──────────────────────────┬───────────────────────────────────┤
│       Tauri IPC Bridge   │     invoke() / emit() / listen() │
│       (JSON/MessagePack) │                                   │
├──────────────────────────┴───────────────────────────────────┤
│                    Rust Backend (Tauri Core)                  │
│                                                              │
│  ┌────────────┐  ┌────────────┐  ┌────────────────────────┐  │
│  │  Search    │  │  Workspace │  │    Plugin System       │  │
│  │  Engine    │  │  Manager   │  │    Manager             │  │
│  └─────┬──────┘  └─────┬──────┘  └──────────┬─────────────┘  │
│        │               │                    │                │
│  ┌─────┴───────────────┴────────────────────┴─────────────┐  │
│  │                 Core Services Layer                     │  │
│  │                                                        │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │  │
│  │  │  USN     │ │ Process  │ │ Window   │ │ Clipboard│  │  │
│  │  │  Indexer │ │ Enum     │ │ Manager  │ │ Service  │  │  │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘  │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │  │
│  │  │  File    │ │ Hotkey   │ │ App      │ │ Tray     │  │  │
│  │  │  Watcher │ │ Service  │ │ Scanner  │ │ Manager  │  │  │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘  │  │
│  └────────────────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────────────────┤
│                  Windows API Layer (windows-rs)               │
│                                                              │
│  NTFS USN Journal │ Win32 API │ COM/DCOM │ WMI │ Registry   │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 数据流设计

#### 3.2.1 搜索请求流

```
用户输入 "report.docx"
    │
    ▼
[前端 SearchBar.vue]
    │ 防抖 100ms
    ▼
invoke("search_files", { query: "report.docx", limit: 50 })
    │
    ▼
[后端 commands/search.rs]
    │ 解析查询
    ▼
[SearchEngine.search()]
    │ 1. 计算文件名 ASCII 和 → 确定查询表 (list0~list40)
    │ 2. 前缀匹配: SELECT * FROM listN WHERE name LIKE 'report%'
    │ 3. 模糊匹配: 拼音转换 + 模糊查询
    │ 4. 合并结果 + 排序
    ▼
[返回 Vec<FileEntry>]
    │
    ▼
[前端渲染搜索结果]
    │ 动画: 逐项淡入
    ▼
用户看到结果列表
```

#### 3.2.2 工作区保存流

```
用户点击 "保存工作区"
    │
    ▼
invoke("save_workspace", { name: "开发环境" })
    │
    ▼
[WorkspaceManager.save_snapshot()]
    │
    ├── 1. EnumWindows() → 枚举所有可见窗口
    │
    ├── 2. 对每个窗口:
    │      ├── GetWindowText() → 窗口标题
    │      ├── GetWindowThreadProcessId() → PID
    │      ├── GetWindowPlacement() → 位置/状态
    │      ├── OpenProcess() + GetModuleFileNameExExe() → EXE 路径
    │      └── WMI 查询 → 命令行参数
    │
    ├── 3. 过滤系统窗口、不可见窗口
    │
    ├── 4. 序列化为 Workspace 结构体
    │
    └── 5. 写入 JSON 文件 (~/.monotools/workspaces/{id}.json)
    │
    ▼
[返回保存结果]
    │
    ▼
前端显示成功提示 + 动画
```

#### 3.2.3 工作区恢复流

```
用户选择工作区 → 点击 "恢复"
    │
    ▼
invoke("restore_workspace", { id: "xxx" })
    │
    ▼
[WorkspaceManager.restore()]
    │
    ├── 1. 读取 Workspace JSON
    │
    ├── 2. 对每个 AppSnapshot:
    │      ├── 检查进程是否已运行
    │      ├── Command::new(exe).args(args).spawn() → 启动应用
    │      ├── 等待窗口出现 (轮询 FindWindow，超时 10s)
    │      └── SetWindowPlacement() → 恢复位置和状态
    │
    └── 3. 返回恢复结果 (成功/失败的应用列表)
    │
    ▼
前端显示恢复进度和结果
```

### 3.3 模块间通信

| 通信方式 | 使用场景 |
|---------|---------|
| Tauri IPC (invoke) | 前端 → 后端的请求/响应 |
| Tauri Events (emit/listen) | 后端 → 前端的事件推送 |
| Rust Channels (mpsc) | Rust 模块间异步通信 |
| Arc<Mutex<T>> | Rust 共享状态 |

---

## 4. 核心功能设计

### 4.1 全局文件搜索引擎

#### 4.1.1 技术原理

基于 NTFS 文件系统的 **USN Journal** (Update Sequence Number Journal) 机制。USN Journal 是 NTFS 维护的一个变更日志，记录了文件系统上所有文件的创建、修改、删除操作。通过直接读取 MFT (Master File Table)，可以高效枚举磁盘上所有文件，而无需遍历目录树。

**参考实现**: File-Engine 项目 (Java + C++)，MonoTools 用 Rust 重写核心逻辑。

#### 4.1.2 索引构建流程

```
[启动 / 定时刷新]
    │
    ▼
遍历所有 NTFS 卷 (C:, D:, ...)
    │ 每个卷一个线程并行处理
    ▼
CreateFile("\\.\C:") → 打开卷句柄
    │
    ▼
DeviceIoControl(FSCTL_CREATE_USN_JOURNAL) → 创建/打开 USN Journal
    │
    ▼
DeviceIoControl(FSCTL_ENUM_USN_DATA) → 枚举 MFT 中所有记录
    │
    ▼
对每条 USN 记录:
    ├── 读取文件名 (FileName)
    ├── 读取父目录 FRN (ParentFileReferenceNumber)
    ├── 通过 FRN 链构建完整路径
    └── 计算文件名 ASCII 和 → 确定分表编号
    │
    ▼
批量写入 SQLite 数据库 (list0 ~ list40)
    │
    ▼
启动文件变更监控 (持续监听 USN Journal 变更)
```

#### 4.1.3 数据库设计

**存储引擎**: SQLite (via `rusqlite`)

**分表策略**: 参考 File-Engine，按文件名 ASCII 字符和分为 41 个表：

```sql
-- 表命名: list0 ~ list40
-- listN 存储 ASCII 和在 [N*100, (N+1)*100) 范围的文件
CREATE TABLE list0 (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL,           -- 完整路径
    name TEXT NOT NULL,           -- 文件名
    parent_path TEXT NOT NULL,    -- 父目录路径
    size INTEGER DEFAULT 0,       -- 文件大小
    modified_at INTEGER DEFAULT 0,-- 修改时间戳
    is_dir INTEGER DEFAULT 0,     -- 是否目录
    ascii_sum INTEGER DEFAULT 0   -- 文件名 ASCII 和
);

-- 索引
CREATE INDEX idx_list0_name ON list0(name);
CREATE INDEX idx_list0_path ON list0(parent_path);
```

**SQLite 优化参数**:
```sql
PRAGMA journal_mode = WAL;          -- WAL 模式提升并发性能
PRAGMA synchronous = NORMAL;        -- 平衡性能和安全
PRAGMA cache_size = -262144;        -- 256MB 缓存
PRAGMA page_size = 65536;           -- 64KB 页大小
PRAGMA mmap_size = 268435456;       -- 256MB 内存映射
PRAGMA temp_store = MEMORY;         -- 临时表存储在内存
```

#### 4.1.4 搜索算法

```
输入: query = "report"

1. 精确匹配:
   SELECT * FROM listN WHERE name = 'report' LIMIT 50

2. 前缀匹配:
   SELECT * FROM listN WHERE name LIKE 'report%' LIMIT 50

3. 包含匹配:
   SELECT * FROM listN WHERE name LIKE '%report%' LIMIT 50

4. 拼音匹配 (中文文件名):
   - 将中文文件名转换为拼音索引
   - SELECT * FROM listN WHERE pinyin LIKE 'report%' LIMIT 50

5. 结果合并与排序:
   - 精确匹配 > 前缀匹配 > 包含匹配 > 拼音匹配
   - 同级别内按最近使用时间排序
   - 去重后返回 Top 50
```

#### 4.1.5 文件变更监控

使用 Windows `ReadDirectoryChangesW` API 或 USN Journal 变更通知：

```rust
// 方案 A: USN Journal 轮询
loop {
    let changes = read_usn_journal_changes(volume_handle, last_usn);
    for change in changes {
        match change.reason {
            USN_REASON_FILE_CREATE => index_add(change.path),
            USN_REASON_FILE_DELETE => index_remove(change.path),
            USN_REASON_RENAME_NEW_NAME => index_update(change.path),
            _ => {}
        }
    }
    sleep(Duration::from_secs(1));
}

// 方案 B: ReadDirectoryChangesW (推荐)
// 实时性更好，资源消耗更低
```

#### 4.1.6 性能目标

| 指标 | 目标值 | 参考值 (File-Engine) |
|------|--------|---------------------|
| 索引构建速度 | 100万文件 < 15秒 | 100万文件 ~10秒 |
| 搜索延迟 | < 50ms | ~30ms |
| 索引文件大小 | < 500MB / 100万文件 | ~300MB |
| 内存占用 | < 100MB (运行时) | ~80MB |
| 增量更新 | < 1秒 / 1000个变更 | ~0.5秒 |

### 4.2 工作区管理

#### 4.2.1 功能定义

工作区管理允许用户：
1. **保存工作区** — 记录当前桌面所有打开应用的状态（EXE路径、命令行参数、窗口位置、窗口状态）
2. **恢复工作区** — 一键启动所有应用并恢复到保存时的窗口布局
3. **管理工作区** — 编辑、删除、导入导出工作区配置
4. **自动保存** — 可选的定时自动保存当前工作状态

#### 4.2.2 数据模型

```rust
use serde::{Deserialize, Serialize};

/// 工作区配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// 唯一标识
    pub id: String,
    /// 工作区名称
    pub name: String,
    /// 描述
    pub description: Option<String>,
    /// 图标 (emoji 或图标路径)
    pub icon: Option<String>,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 最后修改时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// 最后恢复时间
    pub last_restored_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 应用快照列表
    pub apps: Vec<AppSnapshot>,
}

/// 单个应用的窗口快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSnapshot {
    /// 可执行文件路径
    pub exe_path: String,
    /// 命令行参数
    pub args: Vec<String>,
    /// 工作目录
    pub working_dir: Option<String>,
    /// 窗口标题 (用于匹配)
    pub window_title: String,
    /// 窗口位置和大小
    pub window_rect: WindowRect,
    /// 窗口状态
    pub window_state: WindowState,
    /// 启动顺序 (用于串行启动时的排序)
    pub launch_order: Option<u32>,
    /// 启动延迟 (毫秒，用于等待前一个应用就绪)
    pub launch_delay_ms: Option<u64>,
}

/// 窗口矩形
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// 窗口状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
}
```

#### 4.2.3 进程枚举实现

```rust
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;

/// 枚举所有可见窗口
pub fn enumerate_windows() -> Vec<WindowInfo> {
    let mut windows = Vec::new();
    
    unsafe {
        EnumWindows(
            Some(enum_window_callback),
            LPARAM(&mut windows as *mut _ as _),
        );
    }
    
    windows
}

extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        // 过滤不可见窗口
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1); // 继续枚举
        }
        
        // 过滤无标题窗口
        let title = get_window_text(hwnd);
        if title.is_empty() {
            return BOOL(1);
        }
        
        // 获取进程 ID
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        
        // 获取窗口位置
        let mut placement = WINDOWPLACEMENT::default();
        placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
        GetWindowPlacement(hwnd, &mut placement);
        
        // 获取进程信息
        let exe_path = get_process_exe_path(pid);
        let args = get_process_command_line(pid);
        
        let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);
        windows.push(WindowInfo {
            hwnd: hwnd.0,
            title,
            pid,
            exe_path,
            args,
            rect: placement.rcNormalPosition.into(),
            state: placement.showCmd.into(),
        });
        
        BOOL(1) // 继续枚举
    }
}
```

#### 4.2.4 命令行参数获取

通过 Windows WMI (Windows Management Instrumentation) 获取进程命令行：

```rust
/// 通过 WMI 获取进程命令行参数
pub fn get_process_command_line(pid: u32) -> Vec<String> {
    // 使用 windows-rs 的 WMI 接口
    // 或者使用 wmi crate
    let wmi_con = wmi::WMIConnection::new().unwrap();
    let results: Vec<wmi::Win32_Process> = wmi_con
        .filtered_query(&format!("WHERE ProcessId = {}", pid))
        .unwrap();
    
    if let Some(process) = results.first() {
        parse_command_line(&process.CommandLine)
    } else {
        Vec::new()
    }
}
```

#### 4.2.5 窗口恢复实现

```rust
/// 恢复工作区
pub async fn restore_workspace(workspace: &Workspace) -> RestoreResult {
    let mut result = RestoreResult::new();
    
    for app in &workspace.apps {
        // 1. 检查是否已运行
        if is_process_running(&app.exe_path) {
            result.skipped.push(app.clone());
            continue;
        }
        
        // 2. 启动应用
        match start_application(app) {
            Ok(child) => {
                // 3. 等待窗口出现
                match wait_for_window(&app.window_title, Duration::from_secs(10)) {
                    Ok(hwnd) => {
                        // 4. 恢复窗口位置
                        set_window_placement(hwnd, &app.window_rect, &app.window_state);
                        result.restored.push(app.clone());
                    }
                    Err(e) => {
                        result.failed.push((app.clone(), e.to_string()));
                    }
                }
            }
            Err(e) => {
                result.failed.push((app.clone(), e.to_string()));
            }
        }
        
        // 启动间隔
        if let Some(delay) = app.launch_delay_ms {
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }
    }
    
    result
}
```

### 4.3 插件系统

#### 4.3.1 插件架构

采用**双层插件架构**：

```
┌─────────────────────────────────────────────┐
│              Frontend Plugin Layer           │
│  Vue 3 组件 + JavaScript API                │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐     │
│  │ UI 插件 │  │ UI 插件 │  │ UI 插件 │     │
│  └────┬────┘  └────┬────┘  └────┬────┘     │
│       └────────────┼────────────┘           │
│              Plugin SDK (JS)                │
├─────────────────────────────────────────────┤
│              Backend Plugin Layer            │
│  Rust 动态库 (.dll/.so) 或 WASM 模块        │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐     │
│  │ 系统插件│  │ 搜索插件│  │ 集成插件│     │
│  └────┬────┘  └────┬────┘  └────┬────┘     │
│       └────────────┼────────────┘           │
│              Plugin API (Rust)              │
└─────────────────────────────────────────────┘
```

#### 4.3.2 前端插件 API

保持与 ZTools 兼容的 API 设计：

```typescript
// 插件入口
interface MonoToolsPlugin {
  // 插件元信息
  name: string;
  version: string;
  description: string;
  icon: string;
  
  // 生命周期
  onActivate(): void;
  onDeactivate(): void;
  
  // 搜索集成 (可选)
  onSearch?(query: string): Promise<SearchResult[]>;
  
  // UI 渲染 (可选)
  render?(): Vue.Component;
}

// 插件 SDK
interface PluginSDK {
  // 存储
  storage: {
    get(key: string): Promise<any>;
    set(key: string, value: any): Promise<void>;
    delete(key: string): Promise<void>;
  };
  
  // 通知
  notify(message: string, type: 'info' | 'success' | 'warning' | 'error'): void;
  
  // 调用后端
  invoke(cmd: string, args?: Record<string, any>): Promise<any>;
  
  // 事件
  on(event: string, handler: Function): void;
  off(event: string, handler: Function): void;
  emit(event: string, data: any): void;
}
```

#### 4.3.3 Rust 插件接口

```rust
/// Rust 插件 trait
pub trait MonoToolsPlugin: Send + Sync {
    /// 插件名称
    fn name(&self) -> &str;
    
    /// 插件版本
    fn version(&self) -> &str;
    
    /// 初始化
    fn initialize(&mut self, api: &PluginApi) -> Result<()>;
    
    /// 处理搜索请求
    fn on_search(&self, query: &str) -> Result<Vec<SearchResult>> {
        Ok(Vec::new()) // 默认不处理
    }
    
    /// 自定义命令处理
    fn handle_command(&self, cmd: &str, args: &serde_json::Value) -> Result<serde_json::Value> {
        Err(anyhow::anyhow!("Command not supported"))
    }
    
    /// 清理
    fn cleanup(&mut self);
}
```

### 4.4 剪贴板管理

保留 ZTools 的剪贴板管理功能，使用 Rust 重写：

```rust
use arboard::Clipboard;

/// 剪贴板监控服务
pub struct ClipboardService {
    clipboard: Clipboard,
    history: Vec<ClipboardEntry>,
    max_history: usize,
}

impl ClipboardService {
    /// 监控剪贴板变化
    pub fn start_monitoring(&mut self) {
        // 使用定时器轮询剪贴板内容变化
        // 或使用 Windows ClipboardUpdateNotification
    }
    
    /// 获取剪贴板历史
    pub fn get_history(&self) -> &[ClipboardEntry] {
        &self.history
    }
    
    /// 搜索剪贴板历史
    pub fn search(&self, query: &str) -> Vec<&ClipboardEntry> {
        self.history.iter()
            .filter(|entry| entry.content.contains(query))
            .collect()
    }
}
```

---

## 5. UI 设计方案

### 5.1 设计语言

**主题**: Premium Minimal (简约高级)

**设计原则**:
1. **留白** — 充足的间距，让内容呼吸
2. **层次** — 通过阴影和模糊效果创造深度感
3. **克制** — 减少视觉元素，聚焦核心功能
4. **流畅** — 所有交互都有动画过渡
5. **一致** — 统一的圆角、间距、颜色系统

### 5.2 色彩系统

```css
:root {
  /* 基础色 */
  --bg-primary: rgba(255, 255, 255, 0.85);
  --bg-secondary: rgba(255, 255, 255, 0.6);
  --bg-tertiary: rgba(255, 255, 255, 0.4);
  
  /* 文字色 */
  --text-primary: #1a1a2e;
  --text-secondary: #6b7280;
  --text-tertiary: #9ca3af;
  
  /* 强调色 (可配置) */
  --accent-blue: #3b82f6;
  --accent-purple: #8b5cf6;
  --accent-pink: #ec4899;
  --accent-green: #10b981;
  --accent-orange: #f59e0b;
  --accent-red: #ef4444;
  
  /* 毛玻璃效果 */
  --glass-bg: rgba(255, 255, 255, 0.72);
  --glass-border: rgba(255, 255, 255, 0.18);
  --glass-blur: 20px;
  --glass-shadow: 0 8px 32px rgba(0, 0, 0, 0.08);
  
  /* 圆角 */
  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 16px;
  --radius-xl: 24px;
  
  /* 间距 */
  --space-xs: 4px;
  --space-sm: 8px;
  --space-md: 16px;
  --space-lg: 24px;
  --space-xl: 32px;
  --space-2xl: 48px;
}

/* 暗色主题 */
[data-theme="dark"] {
  --bg-primary: rgba(30, 30, 46, 0.85);
  --bg-secondary: rgba(30, 30, 46, 0.6);
  --text-primary: #e2e8f0;
  --text-secondary: #94a3b8;
  --glass-bg: rgba(30, 30, 46, 0.72);
  --glass-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
}
```

### 5.3 毛玻璃效果

```css
.glass-card {
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow);
}
```

### 5.4 搜索界面设计

**交互流程**:
1. 按下全局快捷键 (默认 `Alt + Space`) → 搜索框从屏幕中央弹出
2. 输入关键词 → 实时显示搜索结果
3. 使用 `↑↓` 键导航 → `Enter` 打开选中项 → `Esc` 关闭

**动画设计**:
```css
/* 搜索框弹出动画 */
@keyframes search-enter {
  0% {
    opacity: 0;
    transform: scale(0.9) translateY(-10px);
  }
  100% {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

/* 搜索结果淡入动画 */
@keyframes result-enter {
  0% {
    opacity: 0;
    transform: translateY(8px);
  }
  100% {
    opacity: 1;
    transform: translateY(0);
  }
}

/* 交错动画 — 每个结果项延迟 30ms */
.result-item {
  animation: result-enter 0.2s ease-out forwards;
  animation-delay: calc(var(--index) * 30ms);
  opacity: 0;
}
```

### 5.5 工作区界面设计

**布局**: 卡片网格 (2-3 列)

**每个工作区卡片包含**:
- 工作区名称和图标
- 应用图标预览 (最多显示 6 个)
- 最后使用时间
- 操作按钮 (恢复、编辑、删除)

**动画**:
```css
/* 卡片悬停效果 */
.workspace-card {
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.workspace-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.12);
}

/* 卡片列表交错动画 */
.workspace-card {
  animation: card-enter 0.3s ease-out forwards;
  animation-delay: calc(var(--index) * 50ms);
}

@keyframes card-enter {
  0% {
    opacity: 0;
    transform: scale(0.95) translateY(20px);
  }
  100% {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}
```

### 5.6 主题系统

支持三种模式：
- **亮色模式** — 白色背景，适合日间使用
- **暗色模式** — 深色背景，适合夜间使用
- **跟随系统** — 自动匹配系统主题

6 种主题色可选：蓝、紫、粉、绿、橙、红

---

## 6. 技术选型清单

### 6.1 Rust 后端

| Crate | 版本 | 用途 | 选择理由 |
|-------|------|------|---------|
| `tauri` | 2.x | 应用框架 | 官方框架，生态完善 |
| `windows` | 0.58+ | Win32 API 绑定 | Microsoft 官方维护 |
| `rusqlite` | 0.31+ | SQLite 数据库 | 成熟稳定，性能优秀 |
| `tokio` | 1.x | 异步运行时 | Rust 生态标准 |
| `serde` / `serde_json` | 1.x | 序列化 | 事实标准 |
| `notify` | 6.x | 文件监控 | 跨平台支持 |
| `glob` | 0.3 | 路径匹配 | 标准库级别 |
| `tracing` | 0.1 | 日志 | 结构化日志 |
| `anyhow` | 1.x | 错误处理 | 简化错误处理 |
| `chrono` | 0.4 | 时间处理 | 功能全面 |
| `uuid` | 1.x | UUID 生成 | 标准实现 |
| `arboard` | 3.x | 剪贴板 | 跨平台支持 |
| `wmi` | 0.13 | WMI 查询 | 获取进程信息 |
| `open` | 5.x | 打开文件/URL | 跨平台支持 |
| `dirs` | 5.x | 系统目录 | 标准路径获取 |
| `walkdir` | 2.x | 目录遍历 | 高效遍历 |

### 6.2 前端

| 包名 | 版本 | 用途 | 说明 |
|------|------|------|------|
| `vue` | 3.5+ | UI 框架 | 保持不变 |
| `pinia` | 2.x | 状态管理 | 保持不变 |
| `@tauri-apps/api` | 2.x | Tauri 前端 API | IPC 通信 |
| `@vueuse/core` | 11.x | 组合式工具 | 实用 hooks |
| `@vueuse/motion` | 2.x | 动画库 | 声明式动画 |
| `fuse.js` | 7.x | 前端模糊搜索 | 轻量搜索 |
| `pinyin-pro` | 3.x | 拼音转换 | 中文搜索支持 |
| `lucide-vue-next` | 0.x | 图标库 | 现代图标 |
| `date-fns` | 3.x | 日期处理 | 轻量级 |

### 6.3 构建工具

| 工具 | 用途 |
|------|------|
| `Tauri CLI` | Tauri 项目管理、构建、打包 |
| `Cargo` | Rust 包管理和构建 |
| `Vite` | 前端构建 |
| `pnpm` | 前端包管理 |
| `TypeScript` | 类型检查 |

---

## 7. 项目目录结构

```
MonoTools/
├── src-tauri/                     # Rust 后端
│   ├── Cargo.toml                 # Rust 依赖配置
│   ├── tauri.conf.json            # Tauri 配置
│   ├── build.rs                   # 构建脚本
│   ├── capabilities/              # 权限配置
│   ├── icons/                     # 应用图标
│   └── src/
│       ├── main.rs                # 入口
│       ├── lib.rs                 # 库入口
│       ├── commands/              # Tauri IPC 命令
│       │   ├── mod.rs
│       │   ├── search.rs
│       │   ├── workspace.rs
│       │   ├── clipboard.rs
│       │   ├── plugin.rs
│       │   └── settings.rs
│       ├── services/              # 核心服务
│       │   ├── mod.rs
│       │   ├── file_indexer/      # 文件索引引擎
│       │   │   ├── mod.rs
│       │   │   ├── usn_journal.rs
│       │   │   ├── index_store.rs
│       │   │   ├── search_engine.rs
│       │   │   └── watcher.rs
│       │   ├── workspace/         # 工作区管理
│       │   │   ├── mod.rs
│       │   │   ├── process_enum.rs
│       │   │   ├── window_info.rs
│       │   │   ├── snapshot.rs
│       │   │   └── restore.rs
│       │   ├── clipboard.rs
│       │   ├── hotkey.rs
│       │   └── app_scanner.rs
│       ├── plugins/               # 插件系统
│       │   ├── mod.rs
│       │   ├── loader.rs
│       │   └── api.rs
│       ├── models/                # 数据模型
│       │   ├── mod.rs
│       │   ├── file_entry.rs
│       │   ├── workspace.rs
│       │   └── app_entry.rs
│       ├── utils/                 # 工具函数
│       │   ├── mod.rs
│       │   ├── windows_api.rs
│       │   ├── path_utils.rs
│       │   └── pinyin.rs
│       └── config/
│           ├── mod.rs
│           └── settings.rs
│
├── src/                           # 前端源码
│   ├── main.ts                    # 入口
│   ├── App.vue                    # 根组件
│   ├── assets/                    # 静态资源
│   ├── components/                # 组件
│   │   ├── search/                # 搜索组件
│   │   │   ├── SearchBar.vue
│   │   │   ├── SearchResult.vue
│   │   │   └── ResultItem.vue
│   │   ├── workspace/             # 工作区组件
│   │   │   ├── WorkspacePanel.vue
│   │   │   ├── WorkspaceCard.vue
│   │   │   └── SnapshotDialog.vue
│   │   ├── plugin/                # 插件组件
│   │   │   ├── PluginStore.vue
│   │   │   └── PluginRunner.vue
│   │   ├── settings/              # 设置组件
│   │   │   ├── SettingsPanel.vue
│   │   │   └── ThemeSettings.vue
│   │   └── common/                # 通用组件
│   │       ├── GlassCard.vue
│   │       └── AnimatedList.vue
│   ├── composables/               # 组合式函数
│   │   ├── useSearch.ts
│   │   ├── useWorkspace.ts
│   │   ├── useTheme.ts
│   │   └── useAnimation.ts
│   ├── stores/                    # 状态管理
│   │   ├── searchStore.ts
│   │   ├── workspaceStore.ts
│   │   ├── pluginStore.ts
│   │   └── appStore.ts
│   ├── styles/                    # 样式
│   │   ├── variables.css
│   │   ├── glassmorphism.css
│   │   ├── animations.css
│   │   └── themes/
│   └── utils/
│       ├── tauri.ts
│       └── format.ts
│
├── internal-plugins/              # 内置插件
│   ├── setting/                   # 设置面板
│   └── system/                    # 系统插件
│
├── docs/                          # 文档
│   ├── REDESIGN.md                # 重构设计文档 (本文档)
│   ├── API.md                     # API 文档
│   └── PLUGIN_DEV.md              # 插件开发指南
│
├── tests/                         # 测试
│   ├── src-tauri/                 # Rust 测试
│   └── src/                       # 前端测试
│
├── scripts/                       # 构建脚本
├── resources/                     # 资源文件
├── package.json                   # 前端依赖
├── Cargo.toml                     # Rust workspace 配置 (可选)
├── README.md
├── README_EN.md
├── LICENSE
└── changelog.md
```

---

## 8. 开发阶段规划

### Phase 1: 基础框架搭建 (2-3 周)

**目标**: 搭建 Tauri + Vue 3 项目框架，实现基本窗口和 IPC 通信

- [ ] 初始化 Tauri 2.0 项目
- [ ] 配置 Vue 3 + TypeScript + Vite 前端
- [ ] 实现基本窗口（搜索框弹出/隐藏）
- [ ] 实现 Tauri IPC 通信示例
- [ ] 迁移 MonoTools 的基础 UI 组件
- [ ] 配置全局快捷键 (Alt + Space)
- [ ] 配置系统托盘
- [ ] 重命名 ZTools → MonoTools

**产出**: 可运行的空壳应用，能通过快捷键唤起搜索框

### Phase 2: 文件搜索引擎 (3-4 周)

**目标**: 实现基于 NTFS USN Journal 的文件索引和搜索

- [ ] 实现 NTFS USN Journal 读取模块
- [ ] 实现 MFT 遍历和文件路径构建
- [ ] 实现 SQLite 索引存储（分表策略）
- [ ] 实现搜索引擎（前缀匹配 + 模糊匹配）
- [ ] 实现拼音搜索支持
- [ ] 实现文件变更实时监控
- [ ] 开发搜索 UI 组件
- [ ] 搜索结果排序和展示
- [ ] 性能优化和测试

**产出**: 可用的文件搜索功能，100万文件索引 < 15秒

### Phase 3: 工作区管理 (2-3 周)

**目标**: 实现工作区保存/恢复功能

- [ ] 实现 Windows 进程枚举
- [ ] 实现窗口信息采集（位置、状态、命令行）
- [ ] 实现工作区快照保存
- [ ] 实现工作区恢复（应用启动 + 窗口定位）
- [ ] 开发工作区 UI 组件（卡片列表、编辑对话框）
- [ ] 实现工作区导入/导出
- [ ] 可选：自动保存功能

**产出**: 可用的工作区管理功能

### Phase 4: 插件系统 (2-3 周)

**目标**: 实现插件加载和管理

- [ ] 设计插件 API 规范
- [ ] 实现前端插件加载器
- [ ] 实现 Rust 插件接口（可选）
- [ ] 迁移内置插件（设置面板、系统插件）
- [ ] 开发插件市场 UI（基础版）
- [ ] 编写插件开发文档

**产出**: 可用的插件系统，支持自定义插件

### Phase 5: UI 精打磨 (2-3 周)

**目标**: 完善视觉效果和交互体验

- [ ] 毛玻璃效果实现
- [ ] 动画系统开发（搜索框、结果列表、工作区卡片）
- [ ] 主题系统完善（亮色/暗色/主题色）
- [ ] 响应式适配
- [ ] 图标和品牌设计
- [ ] 性能优化（动画流畅度、内存占用）

**产出**: 精致的 UI 体验

### Phase 6: 测试与发布 (1-2 周)

**目标**: 确保质量，准备发布

- [ ] 单元测试（Rust + 前端）
- [ ] 集成测试
- [ ] 性能测试（搜索、内存、启动速度）
- [ ] Bug 修复
- [ ] 打包配置（Windows 安装包）
- [ ] 文档完善
- [ ] GitHub Release 发布

**产出**: v1.0 正式版

### 时间线总览

```
Week 1-3:   ████████ Phase 1 - 基础框架
Week 4-7:   ████████████ Phase 2 - 文件搜索
Week 8-10:  ████████ Phase 3 - 工作区管理
Week 11-13: ████████ Phase 4 - 插件系统
Week 14-16: ████████ Phase 5 - UI 精打磨
Week 17-18: ████ Phase 6 - 测试发布
```

**总计**: 18 周 (约 4.5 个月)

---

## 9. 风险评估与应对

### 9.1 技术风险

| 风险 | 严重程度 | 发生概率 | 应对策略 |
|------|---------|---------|---------|
| Rust 学习曲线影响开发速度 | 高 | 高 | 预留学习时间，从简单模块开始，使用 AI 辅助编码 |
| NTFS USN Journal 实现复杂 | 高 | 中 | 参考 File-Engine C++ 代码，逐功能翻译为 Rust |
| WebView2 兼容性问题 | 中 | 低 | Win10 1803+ 内置，设置最低版本要求 |
| Tauri API 限制 | 中 | 低 | 必要时使用 sidecar 进程扩展 |
| 文件搜索性能不达标 | 高 | 低 | 参考 File-Engine 优化策略，必要时使用 C++ sidecar |

### 9.2 项目风险

| 风险 | 严重程度 | 发生概率 | 应对策略 |
|------|---------|---------|---------|
| 开发周期超期 | 中 | 中 | 分阶段交付，核心功能优先 |
| 功能范围蔓延 | 中 | 高 | 严格按 Phase 计划执行，新功能放入后续版本 |
| MonoTools 上游更新 | 低 | 中 | 关注上游动态，选择性合并 |
| 插件生态断裂 | 中 | 中 | 保持前端 API 兼容，提供迁移指南 |

### 9.3 应对优先级

1. **Rust 学习** — 在 Phase 1 预留专门学习时间
2. **文件搜索** — 这是核心功能，需要最多的研发投入
3. **工作区管理** — 依赖 Windows API，需要充分测试
4. **UI 打磨** — 可以在功能稳定后再投入

---

## 10. 附录

### 10.1 参考资料

| 资源 | 链接 | 用途 |
|------|------|------|
| Tauri 2.0 官方文档 | https://v2.tauri.app | 框架文档 |
| windows-rs crate | https://crates.io/crates/windows | Win32 API 绑定 |
| File-Engine 项目 | https://github.com/XUANXUQAQ/File-Engine | 文件搜索参考 |
| Everything SDK | https://www.voidtools.com/support/everything/sdk/ | IPC 集成参考 |
| Tauri 插件开发 | https://v2.tauri.app/develop/plugins/ | 插件系统参考 |
| rusqlite 文档 | https://docs.rs/rusqlite | SQLite 绑定 |
| NTFS USN Journal | https://learn.microsoft.com/en-us/windows/win32/fileio/change-journals | 技术原理 |

### 10.2 术语表

| 术语 | 说明 |
|------|------|
| USN Journal | Update Sequence Number Journal，NTFS 文件系统变更日志 |
| MFT | Master File Table，NTFS 文件系统主文件表 |
| FRN | File Reference Number，文件引用号 |
| WebView2 | Windows 内置的 Edge Chromium WebView 组件 |
| IPC | Inter-Process Communication，进程间通信 |
| Tauri | 基于 Rust 的跨平台桌面应用框架 |
| Glassmorphism | 毛玻璃设计风格 |
| Spotlight | macOS 内置的搜索功能，MonoTools 搜索框参考其交互 |
| Capability | Tauri 2.0 的权限控制模型 |

### 10.3 版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| v1.0 | 2026-06-29 | 初始设计文档 |

---

*本文档将随项目开发持续更新。*
