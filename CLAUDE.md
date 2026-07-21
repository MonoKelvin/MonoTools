# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**MonoTools** is a lightweight Windows desktop launcher and system productivity tool (Raycast/Linear-inspired). It runs silently in the system tray and is invoked via `Alt + Space` to show a Spotlight-style search overlay. Core capabilities: application launcher, NTFS USN Journal file search, custom commands, startup manager, and AI-powered recommendations.

Two binaries share the same library: `monotools` (GUI via Tauri) and `monotools-cli` (standalone CLI).

## Tech Stack

- **Frontend**: Vue 3.5+ (Composition API + `<script setup>`) + TypeScript 5.7+ + Vite 6+
- **UI**: PrimeVue 4.x (Aura theme) + Tailwind CSS 4.x + SCSS + lucide-vue-next icons
- **State**: Pinia 3.x | **Routing**: Vue Router 4.4+ (hash mode)
- **Backend**: Rust 1.77+ (2021 Edition) + Tauri 2.11+ + Tokio
- **Plugins**: tauri-plugin-single-instance (单例模式), tauri-plugin-global-shortcut, tauri-plugin-shell, tauri-plugin-fs
- **Database**: SQLite (rusqlite 0.40 bundled) | **CLI**: clap with derive
- **Windows**: windows 0.62 + windows-sys 0.61 | **Search**: NTFS USN Journal, MFT indexing, fuzzy-matcher
- **AI Recommendation**: Python (pybridge) + rule-based engine + hybrid recommendation
- **Package Manager**: pnpm 8+ (workspace monorepo)

## Development Commands

```bash
pnpm install              # Install dependencies
pnpm dev                  # Start Tauri dev mode (Vite HMR on :1420 + Tauri window)
pnpm build                # Build frontend + package Tauri desktop app
pnpm build:frontend       # Build frontend only (vite build)
pnpm build:debug          # Build in debug mode
pnpm tauri build          # Create Windows installers (src-tauri/target/release/bundle/)
pnpm test                 # Run Vitest tests (frontend)
pnpm test:rust            # Run Cargo tests (backend)
pnpm lint                 # ESLint on src/
pnpm format               # Prettier format TS/Vue/SCSS
pnpm format:rust          # cargo fmt
pnpm cli                  # Run CLI binary (e.g., "pnpm cli search chrome")
```

## Architecture

### Frontend (`src/`)

**Modular structure (V1.1)**: 模块化目录结构，遵循 `core/` → `ui/` → `modules/` → `pages/` 的单向依赖。

```
src/
├── core/                    # 核心基础设施 (无业务逻辑)
│   ├── command/            # 命令系统框架 (bindings, specs, store, types, registry)
│   ├── router/             # 路由配置
│   ├── stores/             # 通用 store (theme, settings)
│   ├── config/             # 全局配置 (icon, search, sorting, ui)
│   └── types/              # 通用类型定义 (command, search, settings, statusBar)
│
├── ui/                      # UI 组件库 (纯展示，无业务逻辑)
│   ├── components/         # MtButton, MtCard, MtInput, MtMenu, MtModal, MtComboBox, etc.
│   ├── pages/              # OverlayPage (overlay 容器)
│   └── widgets/            # 带业务逻辑的 UI 组件
│       ├── appicon/        # 应用图标系统 (useAppIcon, useIconRenderer, sources/)
│       ├── HotkeyModal.vue
│       └── ThemeToggle.vue
│
├── modules/                 # 业务模块 (内聚，删除模块时一起删)
│   ├── search/             # 搜索模块
│   │   ├── components/     # SearchInput, ResultItem, AppResultItem, GroupSection, ActionBar, etc.
│   │   ├── composables/    # useSearchStatusBar, useStatusMessages, useAdaptiveText
│   │   ├── utils/          # fileKinds, resultTypeMeta, sort
│   │   ├── pages/          # SearchPage.vue
│   │   ├── store.ts
│   │   ├── types.ts
│   │   └── commandSpecs.ts
│   ├── commands/           # 命令模块
│   │   └── components/     # CommandsPanel
│   └── settings/           # 设置模块
│       └── components/     # SettingsPanel
│
├── services/               # Tauri IPC 封装 (api.ts 是唯一入口)
├── utils/                  # 工具函数 (adaptiveText, format, text)
├── assets/                 # 字体、样式 (theme, tooltip, fonts, main)
├── App.vue
└── main.ts
```

- **Entry**: `src/main.ts` — Vue app + PrimeVue (Aura) + Pinia + Router
- **Router**: 3 hash-mode routes: `/` (SearchPage), `/commands`, `/settings` — 均指向 SearchPage.vue，通过 meta.isPanel 区分面板模式
- **API layer**: `src/services/api.ts` — typed wrappers for all Tauri IPC commands, subdivided by domain (searchApi, appIconApi, commandApi, settingsApi, recommendApi, etc.)
- **Mock backend**: `src/services/tauri.ts` — provides mock data when running in browser (not Tauri context)
- **Stores**: 3 Pinia stores — `theme` (core/stores), `settings` (core/stores), `search` (modules/search)
- **Command Bus**: `src/core/command/` — `commandRegistry.execute(id)` is the single dispatch point for all UI behavior (keyboard, context menu, tray menu).
- **Icon System**: `src/ui/widgets/appicon/` — 可插拔图标源 (ipc, known, lobehub, fallback)，通过 registry 注册，useAppIcon 编排。

### Backend (`src-tauri/src/`)

- **Two binaries** sharing `monotools_lib` library:
  - `main.rs` → GUI (calls `monotools_lib::run()`)
  - `cli_main.rs` → CLI (clap-based)
- **App Module** (`app/`):
  - `builder.rs` — Tauri `Builder` chain
  - `state.rs` — `AppState` (central DI container)
  - `ipc.rs` — IPC command registration
  - `modules.rs` — Feature module setup (tray, etc.)
- **Core Module System** (`core/`):
  - `command/` — `Command` trait, `CommandRegistry`, `CommandRepo`, built-in commands (help, version)
  - `config.rs` — 全局配置常量
  - `error.rs` — 错误类型
- **Services** (`services/`):
  - `hotkey.rs` — 全局热键服务
  - `window.rs` — 窗口管理
  - `storage.rs` — SQLite 存储
  - `window_monitor.rs` — 前台窗口监控
  - `tray/` — 系统托盘
- **Search engine** (`search_engine/`):
  - `AppSearchEngine` (Start Menu/desktop/registry + Trie 索引)
  - `FileSearchEngine` (USN Journal / MFT indexing with SQLite FTS5)
  - `CommandSearchEngine` (custom commands)
  - `SearchSource` trait — 可插拔搜索源抽象
  - `SearchService` — 编排器
- **Recommendation Engine** (`recommend/`):
  - `rule_engine.rs` — 基于规则的推荐 (使用频率 + 最近访问 + 上下文)
  - `py_engine.rs` — Python 推荐引擎 (通过 pybridge 调用)
  - `engine.rs` — 混合推荐编排
- **PyBridge** (`pybridge/`):
  - Python 子进程管理
  - JSON-RPC 通信协议
  - 服务注册表
- **Repositories** (`repositories/`): Trait-based data access — `SettingsRepo`, `CommandRepo`, `StatsRepo`, `PinRepo`
- **Models** (`models/`): `Settings`
- **Search models** (`search_engine/models/`): `AppEntry`, `FileResult`, `SearchResult`
- **Platform** (`platform/windows/`): Windows-specific code — `hotkey.rs`, `icon.rs`, `shell.rs`, `usn.rs`, `commands.rs`, `mica.rs`, `special_shortcuts.rs`, `version.rs`

### Python Recommendation Service (`python/`)

```
python/
├── recommend/
│   └── service.py      # 推荐服务 (JSON-RPC)
├── pybridge/
│   └── server.py       # PyBridge 服务端
├── requirements.txt
└── README.md
```

### Data Flow (Search Example)

```
User types → SearchPage → searchStore.setQuery() → debounce → searchApi.search()
  → Tauri invoke → search_cmd() (app/ipc.rs) → AppState → SearchService (3 engines)
  → results sorted by score → back to frontend
  → User presses Enter → searchApi.execute() → execute_result() → shell::launch()
```

### Data Flow (Recommendation Example)

```
App focus change → window_monitor → recommend engine
  → rule_engine + py_engine → hybrid scoring → recommend list
  → searchStore.recommendations → SearchPage → GroupSection (pinned/recent groups)
```

## Key Conventions

- **Rust**: Follows Rust API Guidelines. Workspace in `Cargo.toml`, library in `src-tauri/Cargo.toml`. Binary `monotools` (GUI) and `monotools-cli` (CLI) both depend on `monotools_lib`.
- **Frontend**: Strict TypeScript, `<script setup>` syntax, `@/` path alias resolves to `src/`.
- **Styling**: Custom color palette (bg-primary: `#1a1a2e`, accent: `#ff6b6b`), frosted-glass backdrop-blur effect, CSS custom properties for theming. Custom Mt* component system (MtButton, MtMenu, MtInput, etc.) for consistent UI.
- **Config**: Root `tauri.conf.json` defines two windows (search overlay + main window). CSP security policy. Bilingual installers (zh-CN + en-US).
- **Testing**: All test code (Vitest + cargo) lives under `<repo>/tests/`. See [tests/SKILL.md](./tests/SKILL.md). Sub `tests/rust/` for cargo integration (loaded via `[[test]] path = "../tests/run.rs"` from `src-tauri/Cargo.toml`) and `tests/ui/` for Vitest. Vitest config: `vitest.config.ts` with happy-dom + `@/` alias.
- **Background Tasks**: File indexing runs asynchronously in background via `tauri::async_runtime::spawn`, hotkey registration uses async with retry logic.
- **Single Instance**: Uses `tauri-plugin-single-instance` to ensure only one instance runs at a time. When a new instance is launched, it activates the existing window instead of creating a new one.
- **ActionBar**: Bottom status bar with merged status display. Shows index building status (with fade-in animation) and search results count/status. Status messages auto-hide after timeout (5s for completed, 8s for error — values from `src/core/config/ui.ts::ACTION_BAR_TIMEOUTS`).
- **GroupSection Selection**: 每个分组独立且互斥的选中状态。`activeSelectionGroupId` + `selectedIndexes` (Set<number> 本地索引)，切换分组自动清空旧选中。支持单选、Ctrl+单击切换、Shift 范围选择、Ctrl+Shift 范围反选。
- **Custom Tooltip**: GroupSection 统一使用自定义 tooltip (非 PrimeVue v-tooltip)，支持所有显示模式，hover 项任意位置触发，选中项也显示。玻璃模糊效果 + 三级视觉层次 (标题/副标题/路径)。

## 编码规范 (Coding Standards)

> 旨在防止优化成果 reverse drift. 任何破坏以下规范的 PR 需在描述中说明理由.
> 这部分规则由 V1.1 重构计划沉淀.

### 1. 配置管理 (Configuration)

1.1 **不写魔法数**: 出现 ≥ 2 次的字面量必须抽到 [src/core/config/](file:///e:/work/code/MTools/src/core/config) (前端) 或 [src-tauri/src/core/config.rs](file:///e:/work/code/MTools/src-tauri/src/core/config.rs) (后端). 单一真源. 改一处全工程生效.

1.2 **不写魔法字符串**: 文件路径与事件名必须在 `config::paths::*` / `config::ipc_events::*` 定义. Windows 跳过名单等在 `config::fs::SKIP_*` 集中.

1.3 **跨前后端常量必须双向同步**: 改 `ICON_CONFIG.size` 时同步改 `config::icon::SIZE`. PR description 必须声明 "已同步前后端".

1.4 **SCSS 变量与 TS 变量接受短期双重真源** (`--text-md` ↔ `FONT_SIZES.base`), 长期通过 Vite `additionalData` 把 TS 变量注入 SCSS 消除双重.

### 2. 抽象机制 (Abstraction)

2.1 **重复 ≥ 2 处 → 必须抽象**: AppResultItem / ResultItem 的 icon 加载代码已抽到 [src/ui/widgets/appicon/useIconRenderer.ts](file:///e:/work/code/MTools/src/ui/widgets/appicon/useIconRenderer.ts). 任何 "非常相似" 的 2+ 处在第 3 处出现前必须抽.

2.2 **可扩展机制优先于硬编码**: `resultType` → label / icon 放在 [src/modules/search/utils/resultTypeMeta.ts](file:///e:/work/code/MTools/src/modules/search/utils/resultTypeMeta.ts), 新增 type 只需 1 处改动. 同理 `fileKinds` / `commands` 都应走 "中央表 + 查找函数" 模式.

2.3 **避免过度抽象**: 只在 ≥ 2 个真实调用方出现后再抽象, 不为 "未来可能用到" 提前建抽象层.

2.4 **composable 不接收 emit**: 副作用仍由父组件通过 emit 透传, composable 只管 "状态机的输入与输出".

2.5 **扩展点用 trait / interface + registry**: 后端新增 `SearchSource` / 前端新增 `IconSource` 通过 registry 注册. 编排器 (`SearchService` / `useAppIcon`) 不感知具体 source / impl.

### 3. 模块边界 (Module Boundaries)

3.1 **前端模块单向依赖**: `core/` → `ui/` → `modules/` → `pages/`. `modules/` 之间互不直接依赖, 通过 `core/` 的共享类型和服务通信.

3.2 **UI 组件零业务**: `ui/components/` 下的组件只负责展示, 不直接调用 services/stores, 所有数据通过 props 传入, 所有交互通过 emit 传出. `ui/widgets/` 可包含业务逻辑 (如 appicon 系统).

3.3 **后端 engines 解耦**: App / File / CommandSearchEngine 互不直接 import, 编排由 [src-tauri/src/search_engine/service.rs](file:///e:/work/code/MTools/src-tauri/src/search_engine/service.rs) 通过 `SearchSource` trait 负责.

3.4 **平台代码隔离**: Windows 特定代码必须在 [src-tauri/src/platform/windows/](file:///e:/work/code/MTools/src-tauri/src/platform/windows/), 跨平台 `#[cfg(windows)]` 守卫.

3.5 **状态归属唯一**: 分组折叠 → `useSearchStore().collapsedGroups`, 分组选中 → `useSearchStore().activeSelectionGroupId` + `useSearchStore().selectedIndexes`, 禁止同一状态在 store / props / ref 三处并存. computed 可派生但不能反向同步.

3.6 **API layer 单向引用**: [src/services/api.ts](file:///e:/work/code/MTools/src/services/api.ts) 是 IPC 命令的唯一入口, stores / composables 不直接 `invoke()`.

3.7 **循环依赖禁止**: 禁止 `store.ts` 从 `@/modules/search` 导入类型 (会形成循环依赖). 必须直接从 `./types` 导入.

### 4. 冗余代码审查 (Dead Code Review)

4.1 `#[allow(dead_code)]` 标记 30 天未用 → 删除: 标记前必须 `grep` 全工程确认无引用.

4.2 **重复 if 分支合并**: 相同字面量出现 ≥ 3 次 → 提常量.

4.3 **console.log 提交前清理**: 每条必须标 `[Tag:scope]` 前缀 (例: `[useAppIcon]`), 便于按 tag grep 关闭.

4.4 **死 computed / 死 emit / 死 prop / 死 export 一律删除**: PR 提交前必须 `grep "<符号名>"` 确认无引用.

4.5 **禁用占位代码**: `let _ = cmd` / `void emit` / `// TODO 后续实现` 这类抑制警告的占位在主路径出现需 PR 描述说明; 短期 stub 加 `#[allow(dead_code)]` + TODO + owner 注释.
