# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**MonoTools** is a lightweight Windows desktop launcher and system productivity tool (Raycast/Linear-inspired). It runs silently in the system tray and is invoked via `Alt + Space` to show a Spotlight-style search overlay. Core capabilities: application launcher, NTFS USN Journal file search, and custom commands.

Two binaries share the same library: `monotools` (GUI via Tauri) and `monotools-cli` (standalone CLI).

## Tech Stack

- **Frontend**: Vue 3.5+ (Composition API + `<script setup>`) + TypeScript 5.6+ + Vite 5.4+
- **UI**: PrimeVue 4.x (Aura theme) + Tailwind CSS 3.x + SCSS + lucide-vue-next icons
- **State**: Pinia 2.x | **Routing**: Vue Router 4.4+ (hash mode)
- **Backend**: Rust 1.77+ (2021 Edition) + Tauri 2.x + Tokio
- **Database**: SQLite (rusqlite bundled) | **CLI**: clap with derive
- **Windows**: windows crate 0.58 + windows-sys 0.59 | **Search**: fuzzy-matcher, walkdir
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
- **Components**: Shared (`SearchInput`, `ResultItem`, `ThemeToggle`) and page-specific (`CategoryTabs`, `SearchResults`, `ActionBar`)

### Backend (`src-tauri/src/`)

- **Two binaries** sharing `monotools_lib` library:
  - `main.rs` → GUI (calls `monotools_lib::run()`)
  - `cli_main.rs` → CLI (clap-based)
- **`app.rs`**: Tauri `Builder` chain — creates `AppState` (central DI container), registers hotkeys, sets up `invoke_handler` with all IPC commands
- **`commands.rs`**: All `#[tauri::command]` functions — the Tauri IPC bridge to frontend
- **Command pattern** (`command/`): Trait-based system serving both CLI and internal dispatch. Each subcommand implements `Command` trait, registered in `CommandRegistry`, dispatched by `CommandEngine`
- **Services** (`services/`): `AppState` (shared state), `HotkeyService`, `WindowService`, `SearchEngine`, `StorageService` (SQLite), `DelayLauncher`
- **Search engines** (`engines/`): 3 parallel engines — `AppSearchEngine` (Start Menu/desktop), `FileSearchService` (USN Journal / fallback walkdir), `CommandSearchEngine`
- **Repositories** (`repositories/`): Trait-based data access with in-memory implementations. Works with both CLI and Tauri
- **Platform** (`platform/windows/`): Windows-specific code — `registry.rs` (reg.exe wrapper), `usn.rs` (file indexing), `shell.rs` (launch), `hotkey.rs`

### Data Flow (Search Example)

```
User types → SearchPage → searchStore.setQuery() → debounce → searchApi.search()
  → Tauri invoke → search_cmd() (commands.rs) → AppState engines (4 engines)
  → results sorted by score → back to frontend
  → User presses Enter → searchApi.execute() → execute_result() → shell::launch()
```

## Key Conventions

- **Rust**: Follows Rust API Guidelines. Workspace in `Cargo.toml`, library in `src-tauri/Cargo.toml`. Binary `monotools` (GUI) and `monotools-cli` (CLI) both depend on `monotools_lib`.
- **Frontend**: Strict TypeScript, `<script setup>` syntax, `@/` path alias resolves to `src/`.
- **Styling**: Custom color palette (bg-primary: `#1a1a2e`, accent: `#ff6b6b`), frosted-glass backdrop-blur effect, CSS custom properties for theming.
- **Config**: Root `tauri.conf.json` defines two windows (search overlay + main window). CSP security policy. Bilingual installers (zh-CN + en-US).
- **No tests exist yet** but tooling is set up (Vitest for frontend, `cargo test` for Rust).
