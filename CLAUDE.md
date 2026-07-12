# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**MonoTools** is a lightweight Windows desktop launcher and system productivity tool (Raycast/Linear-inspired). It runs silently in the system tray and is invoked via `Alt + Space` to show a Spotlight-style search overlay. Core capabilities: application launcher, NTFS USN Journal file search, custom commands, and startup manager.

Two binaries share the same library: `monotools` (GUI via Tauri) and `monotools-cli` (standalone CLI).

## Tech Stack

- **Frontend**: Vue 3.5+ (Composition API + `<script setup>`) + TypeScript 5.7+ + Vite 6+
- **UI**: PrimeVue 4.x (Aura theme) + Tailwind CSS 4.x + SCSS + lucide-vue-next icons
- **State**: Pinia 3.x | **Routing**: Vue Router 4.4+ (hash mode)
- **Backend**: Rust 1.77+ (2021 Edition) + Tauri 2.11+ + Tokio
- **Plugins**: tauri-plugin-single-instance (单例模式), tauri-plugin-global-shortcut, tauri-plugin-shell, tauri-plugin-fs
- **Database**: SQLite (rusqlite 0.40 bundled) | **CLI**: clap with derive
- **Windows**: windows 0.62 + windows-sys 0.61 | **Search**: NTFS USN Journal, MFT indexing, fuzzy-matcher
- **Package Manager**: pnpm 8+ (workspace monorepo)

## Development Commands

```bash
pnpm install              # Install dependencies
pnpm dev                  # Start Tauri dev mode (Vite HMR on :1420 + Tauri window)
pnpm build                # Build frontend + package Tauri desktop app
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

- **Entry**: `src/main.ts` — Vue app + PrimeVue (Aura) + Pinia + Router
- **Router**: 3 hash-mode routes: `/` (SearchPage), `/commands` (CommandsPage), `/settings` (SettingsPage)
- **API layer**: `src/services/api.ts` — typed wrappers for all Tauri IPC commands, subdivided by domain (searchApi, commandApi, settingsApi, etc.)
- **Mock backend**: `src/services/tauri.ts` — provides mock data when running in browser (not Tauri context)
- **Stores**: 3 Pinia stores — `theme`, `settings`, `search`
- **Components**:
  - **Common**: `MtButton`, `MtCard`, `MtDivider`, `MtInput`, `MtMenu`, `MtPanel`, `SearchInput`, `ResultItem`, `ThemeToggle`
  - **Panels**: `CommandsPanel`, `SettingsPanel`
  - **Search**: `ActionBar`, `CategoryTabs`, `VirtualGroupedResults`

### Backend (`src-tauri/src/`)

- **Two binaries** sharing `monotools_lib` library:
  - `main.rs` → GUI (calls `monotools_lib::run()`)
  - `cli_main.rs` → CLI (clap-based)
- **`app.rs`**: Tauri `Builder` chain — creates `AppState` (central DI container), registers hotkeys, sets up `invoke_handler` with all IPC commands
- **`app_state.rs`**: Central application state container managing all services and engines
- **`commands.rs`**: All `#[tauri::command]` functions — the Tauri IPC bridge to frontend
- **Command pattern** (`command/`): Trait-based system serving both CLI and internal dispatch. Each subcommand implements `Command` trait, registered in `CommandRegistry`, dispatched by `CommandEngine`
  - 8 commands: `search`, `launch`, `open`, `command`, `config`, `help`, `version`, `index`, `stats`
- **Services** (`services/`): `AppState` (shared state), `HotkeyService`, `WindowService`, `SearchEngine`, `StorageService` (SQLite)
- **Search engines** (`engines/`):
  - `AppSearchEngine` (Start Menu/desktop/registry)
  - `FileSearchEngine` (USN Journal / MFT indexing with SQLite FTS5)
  - `CommandSearchEngine` (custom commands)
- **Repositories** (`repositories/`): Trait-based data access — `SettingsRepo`, `CommandRepo`, `StatsRepo`
- **Models** (`models/`): `AppEntry`, `FileResult`, `SearchResult`, `CustomCommand`, `Settings`
- **Platform** (`platform/windows/`): Windows-specific code — `hotkey.rs`, `registry.rs`, `usn.rs`, `shell.rs`

### Data Flow (Search Example)

```
User types → SearchPage → searchStore.setQuery() → debounce → searchApi.search()
  → Tauri invoke → search_cmd() (commands.rs) → AppState engines (3 engines)
  → results sorted by score → back to frontend
  → User presses Enter → searchApi.execute() → execute_result() → shell::launch()
```

## Key Conventions

- **Rust**: Follows Rust API Guidelines. Workspace in `Cargo.toml`, library in `src-tauri/Cargo.toml`. Binary `monotools` (GUI) and `monotools-cli` (CLI) both depend on `monotools_lib`.
- **Frontend**: Strict TypeScript, `<script setup>` syntax, `@/` path alias resolves to `src/`.
- **Styling**: Custom color palette (bg-primary: `#1a1a2e`, accent: `#ff6b6b`), frosted-glass backdrop-blur effect, CSS custom properties for theming. Custom Mt* component system (MtButton, MtMenu, MtInput, etc.) for consistent UI.
- **Config**: Root `tauri.conf.json` defines two windows (search overlay + main window). CSP security policy. Bilingual installers (zh-CN + en-US).
- **Testing**: All test code (Vitest + cargo) lives under `<repo>/tests/`. See [tests/SKILL.md](./tests/SKILL.md). Sub `tests/rust/` for cargo integration (loaded via `[[test]] path = "../tests/run.rs"` from `src-tauri/Cargo.toml`) and `tests/ui/` for Vitest. Vitest config: `vitest.config.ts` with happy-dom + `@/` alias.
- **Background Tasks**: File indexing runs asynchronously in background via `tauri::async_runtime::spawn`, hotkey registration uses async with retry logic.
- **Single Instance**: Uses `tauri-plugin-single-instance` to ensure only one instance runs at a time. When a new instance is launched, it activates the existing window instead of creating a new one.
- **ActionBar**: Bottom status bar with merged status display. Shows index building status (with fade-in animation) and search results count/status. Status messages auto-hide after timeout (5s for completed, 8s for error — values from `src/config/ui.ts::ACTION_BAR_TIMEOUTS`).
- **Command Bus**: `src/commands/` — `commandRegistry.execute(id)` is the single dispatch point for all UI behavior (keyboard, context menu, tray menu). Built-in command builders in `src/commands/builtins/` (search/system). All registered commands can be enumerated via `commandSpecsApi.list()`.

## 编码规范 (Coding Standards)

> 旨在防止优化成果 reverse drift. 任何破坏以下规范的 PR 需在描述中说明理由.
> 这部分规则由 [`monotools-overall-optimization-plan`](./.trae/documents/monotools-optimization-v3.md) 沉淀.

### 1. 配置管理 (Configuration)

1.1 **不写魔法数**: 出现 ≥ 2 次的字面量必须抽到 [src/config/](file:///d:/Work/Code/MonoStudio/MonoTools/src/config) (前端) 或 [src-tauri/src/config.rs](file:///d:/Work/Code/MonoStudio/MonoTools/src-tauri/src/config.rs) (后端). 单一真源. 改一处全工程生效.

1.2 **不写魔法字符串**: 文件路径 (`explorer.exe` / Start Menu) 与事件名 (`index_progress`) 必须在 `config::paths::*` / `config::ipc_events::*` 定义. Windows 跳过名单 (`thumbs.db` / `winsxs`) 等在 `config::fs::SKIP_*` 集中.

1.3 **跨前后端常量必须双向同步**: 改 `ICON_CONFIG.size` 时同步改 `config::icon::SIZE`. PR description 必须声明 "已同步前后端". 例: `icon.rs::ICON_PX` 必须用 `icon_cfg::SIZE`, 不得留硬编码.

1.4 **SCSS 变量与 TS 变量接受短期双重真源** (`--text-md` ↔ `FONT_SIZES.base`), 长期通过 Vite `additionalData` 把 TS 变量注入 SCSS 消除双重.

### 2. 抽象机制 (Abstraction)

2.1 **重复 ≥ 2 处 → 必须抽象**: AppResultItem / ResultItem 80% 重复的 icon 加载代码已抽到 [src/composables/useIconRenderer.ts](file:///d:/Work/Code/MonoStudio/MonoTools/src/composables/useIconRenderer.ts). 任何 "非常相似" 的 2+ 处在第 3 处出现前必须抽.

2.2 **可扩展机制优先于硬编码**: `resultType` → label / icon 放在 [src/utils/resultTypeMeta.ts](file:///d:/Work/Code/MonoStudio/MonoTools/src/utils/resultTypeMeta.ts), 新增 type 只需 1 处改动. 同理 `fileKinds` / `commands` 都应走 "中央表 + 查找函数" 模式. 评分函数 (`score_app_match`) 与常量 (`APP_SCORE_*`) 必须解耦, 单测只调函数, 不调私有方法.

2.3 **避免过度抽象**: 只在 ≥ 2 个真实调用方出现后再抽象, 不为 "未来可能用到" 提前建抽象层.

2.4 **composable 不接收 emit**: 副作用仍由父组件通过 emit 透传, composable 只管 "状态机的输入与输出".

2.5 **扩展点用 trait / interface + registry**: 后端新增 `SearchSource` (engines/search_source.rs) / `IconExtractor` (platform/windows/icon.rs) / 前端新增 `IconSource` (composables/iconSources/registry.ts) 通过 registry 注册. 编排器 (`SearchEngine` / `get_extractor` / `useAppIcon`) 不感知具体 source / impl.

### 3. 模块边界 (Module Boundaries)

3.1 **跨模块调用单向**: `components/` 只依赖 `composables/` `utils/` `services/`. `composables/useX.ts` **不允许** `import @/components/*`.

3.2 **后端 engines 解耦**: App / File / CommandSearchEngine 互不直接 import, 编排由 [src-tauri/src/services/search.rs](file:///d:/Work/Code/MonoStudio/MonoTools/src-tauri/src/services/search.rs) 通过 `SearchSource` trait 负责.

3.3 **平台代码隔离**: Windows 特定代码 (icon / usn / shell / registry) 必须在 [src-tauri/src/platform/windows/](file:///d:/Work/Code/MonoStudio/MonoTools/src-tauri/src/platform/windows/), 跨平台 `#[cfg(windows)]` 守卫.

3.4 **状态归属唯一**: 折叠 → `useSearchStore().collapsedGroups`, 选中 → `useSearchStore().selectedGlobalId`, 禁止同一状态在 store / props / ref 三处并存. computed 可派生但不能反向同步.

3.5 **API layer 单向引用**: [src/services/api.ts](file:///d:/Work/Code/MonoStudio/MonoTools/src/services/api.ts) 是 IPC 命令的唯一入口, stores / composables 不直接 `invoke()`.

### 4. 冗余代码审查 (Dead Code Review)

4.1 `#[allow(dead_code)]` 标记 30 天未用 → 删除: 标记前必须 `grep` 全工程确认无引用.

4.2 **重复 if 分支合并**: `app_type_of` 中 `starts_with` 与 `contains` 重复时, 保留前者即可. 相同字面量出现 ≥ 3 次 → 提常量.

4.3 **console.log 提交前清理**: 每条必须标 `[Tag:scope]` 前缀 (例: `[useAppIcon]`), 便于按 tag grep 关闭.

4.4 **死 computed / 死 emit / 死 prop / 死 export 一律删除**: PR 提交前必须 `grep "<符号名>"` 确认无引用. 已知历史: `topResults` (stores/search.ts) / `displayResults` (SearchPage.vue) / `monogram` kind (ResultItem.vue).

4.5 **禁用占位代码**: `let _ = cmd` / `void emit` / `// TODO 后续实现` 这类抑制警告的占位在主路径出现需 PR 描述说明; 短期 stub 加 `#[allow(dead_code)]` + TODO + owner 注释.
