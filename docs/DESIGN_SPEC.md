# MonoTools 总体设计规范

> 版本: v1.0  
> 日期: 2026-07-01  
> 状态: 设计定稿  
> 适用范围: 全栈开发指导

---

## 1. 项目定位

MonoTools 是一款面向 Windows 平台（后续支持 macOS/Linux）的**高性能桌面启动器与效率工具**。软件采用**静默驻留**模式运行，通过全局快捷键唤出 Spotlight 式搜索框，提供应用启动、全局文件搜索、工作区管理、快捷命令执行等核心能力。

**核心特征**:
- **静默驻留**: 开机自启，托盘运行，无主窗口常驻
- **全局唤出**: `Alt + Space` 快捷键捕获，屏幕中央弹出搜索框
- **插件驱动**: 功能通过插件扩展，主题本身也是插件
- **命令化**: 所有功能暴露为命令指令，支持 CLI 和 IPC 双通道调用

---

## 2. 设计原则

### 2.1 架构原则

| 原则 | 说明 |
|------|------|
| **分层解耦** | 前端 / IPC / 核心服务 / 系统 API 四层严格分离 |
| **插件优先** | 非核心功能一律插件化；主题、搜索提供者、快捷指令均为插件 |
| **命令统一** | 所有功能抽象为 `Command` 对象，CLI 与 GUI 共用同一套后端逻辑 |
| **后台优先** | 即使没有 UI，后台 Rust API 也能完成 100% 功能 |

### 2.2 UI 设计原则

基于 `DESIGN.md` 的 Linear 设计语言：

- **深黑画布**: 背景色 `#010102`，近乎纯黑的深蓝黑底色
- **层级表面**: 四级表面递进（surface-1 ~ surface-4）构建卡片层级
- **单一强调色**: 薰衣草蓝 `#5e6ad2` 仅用于品牌标识、主按钮、焦点环
- **负字距标题**: 大标题使用 `-3.0px` 至 `-0.6px` 负字距
- **产品截图主导**: 界面以功能面板为主角，Chrome 最小化
- **无渐变、无阴影**: 深度通过表面层级和 1px 发丝边框表达

### 2.3 插件设计原则

- **热插拔**: 插件可在运行时加载、卸载、重载，无需重启主程序
- **自包含**: 每个插件是一个独立目录，含 `plugin.json` + 入口文件 + 资源
- **权限显式**: 插件需在 `plugin.json` 中声明所需权限（Capability-based）
- **主题即插件**: 默认主题是一个内置插件（`builtin:default-theme`），不可卸载

---

## 3. 技术栈总览

| 层级 | 技术 | 版本 | 说明 |
|------|------|------|------|
| 前端框架 | Vue 3 | 3.5+ | Composition API + `<script setup>` |
| 前端语言 | TypeScript | 5.5+ | 严格模式，全类型覆盖 |
| UI 组件库 | PrimeVue | 4.x | 基础组件库，需按设计规范深度定制 |
| 状态管理 | Pinia | 2.x | 全局状态 + 插件状态隔离 |
| 构建工具 | Vite | 6.x | 前端构建 + HMR |
| 应用框架 | Tauri | 2.x | Rust 后端 + WebView2 前端 |
| 后端语言 | Rust | 1.80+ | 2021 Edition |
| 异步运行时 | Tokio | 1.x | Rust 异步 IO |
| 数据库 | SQLite | 3.x | 文件索引、配置、插件元数据 |
| 系统 API | windows-rs | 0.58+ | Win32 API 类型安全绑定 |
| 序列化 | serde | 1.x | JSON / MessagePack |

---

## 4. 功能范围（MVP）

### 4.1 核心功能（不可插件化）

| 功能 | 说明 | 责任层 |
|------|------|--------|
| 全局快捷键监听 | `Alt + Space` 系统级热键捕获 | Rust / Tauri |
| 搜索框唤出/隐藏 | 窗口定位、动画、失焦自动隐藏 | Rust + Vue |
| 插件系统内核 | 插件加载、生命周期、权限管理、API 注入 | Rust |
| 主题渲染引擎 | 主题变量注入、CSS 变量动态切换 | Vue + Rust |
| 命令总线 | 指令解析、路由、执行、结果返回 | Rust |
| 设置中心 | 基础配置持久化、热重载 | Rust + Vue |

### 4.2 默认插件（内置，可禁用但不可卸载）

| 插件 ID | 功能 | 类型 |
|---------|------|------|
| `builtin:default-theme` | Linear 风格深色主题 | 主题 |
| `builtin:app-launcher` | 应用启动器（开始菜单程序索引） | 搜索提供者 |
| `builtin:file-search` | NTFS USN Journal 全局文件搜索 | 搜索提供者 |
| `builtin:workspace-manager` | 工作区保存/恢复 | 功能 |
| `builtin:command-palette` | 快捷命令面板（重启、关机等） | 快捷指令 |
| `builtin:settings` | 设置面板 UI | 视图 |

### 4.3 用户可扩展（示例）

- 第三方主题插件
- 浏览器书签搜索插件
- 剪贴板历史插件
- 计算器插件
- 翻译插件

---

## 5. 关键术语

| 术语 | 定义 |
|------|------|
| **Command** | 命令指令，后端可执行的最小功能单元，如 `search:files` |
| **Query** | 搜索框输入的字符串，可含触发前缀（如 `>settings`） |
| **Provider** | 搜索提供者插件，向搜索框注册结果源 |
| **Workspace** | 桌面状态快照，包含打开的窗口、位置、进程信息 |
| **Capability** | Tauri 2.0 权限模型，插件需显式声明所需能力 |
| **Hook** | 插件生命周期钩子（activate/deactivate/search/execute） |
| **Theme Token** | 主题变量，如 `--mt-bg-canvas`，运行时注入 CSS |

---

## 6. 文档索引

本文档为总纲，详细设计见以下分册：

1. [ARCHITECTURE.md](./ARCHITECTURE.md) — 系统架构与模块分层
2. [PLUGIN_SYSTEM.md](./PLUGIN_SYSTEM.md) — 插件系统详细设计
3. [CORE_FEATURES.md](./CORE_FEATURES.md) — 搜索、工作区、快捷键核心功能
4. [UI_DESIGN.md](./UI_DESIGN.md) — UI 规范与 PrimeVue 定制
5. [API_REFERENCE.md](./API_REFERENCE.md) — 命令指令、IPC、插件 API
6. [DATA_MODEL.md](./DATA_MODEL.md) — 数据模型与存储设计
7. [DEVELOPMENT_GUIDE.md](./DEVELOPMENT_GUIDE.md) — 开发阶段与工程规范