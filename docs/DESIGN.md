# MonoTools - 详细设计文档

> **项目状态**: 设计阶段  
> **文档版本**: v1.0  
> **最后更新**: 2026-07-06  
> **技术栈**: Tauri 2.x + Rust + Vue 3 + PrimeVue

---

## 目录

1. [项目概述](#1-项目概述)
2. [需求分析](#2-需求分析)
3. [总体架构设计](#3-总体架构设计)
4. [前端设计](#4-前端设计)
5. [后端设计](#5-后端设计)
6. [核心功能模块详细设计](#6-核心功能模块详细设计)
7. [数据存储设计](#7-数据存储设计)
8. [构建与发布](#8-构建与发布)
9. [开发路线图](#9-开发路线图)
10. [附录](#10-附录)

---

## 1. 项目概述

### 1.1 产品定位

MonoTools 是一款轻量级系统效率工具，聚焦于 **全局搜索** 和 **自定义命令** 两大核心能力。产品对标 [Raycast](https://www.raycast.com/)、[Alfred](https://www.alfredapp.com/)、[uTools](https://u.tools/)，但以更轻的架构和更快的搜索速度为目标。

### 1.2 核心特性

| 特性 | 说明 |
|------|------|
| **全局唤起** | 系统级快捷键（默认 `Alt+Space`）唤出搜索界面 |
| **全局搜索** | 毫秒级搜索应用、文件、命令 |
| **自定义命令** | 支持 Shell 脚本、URL Scheme、自定义逻辑 |
| **主题切换** | 支持亮色/暗色/跟随系统，默认暗色 Raycast 风格 |
| **插件系统** | 架构预留插件接口 |
| **CLI 模式** | 后台服务提供完整 CLI 接口，无 UI 也能使用 |

### 1.3 技术选型

| 层级 | 技术 | 选型理由 |
|------|------|----------|
| 桌面框架 | **Tauri 2.x** | Rust 生态、体积小（<5MB）、安全、跨平台 |
| 后端语言 | **Rust** | 性能、内存安全、丰富的系统 API crate |
| 前端框架 | **Vue 3 + TypeScript** | 响应式强、开发效率高、生态丰富 |
| UI 组件库 | **PrimeVue** | 企业级组件丰富、主题可定制、文档完善 |
| 状态管理 | **Pinia** | Vue 官方推荐，轻量直观 |
| 路由 | **Vue Router** | 单页应用路由 |
| 样式 | **Tailwind CSS** | 原子化 CSS，快速构建 UI |
| 构建工具 | **Vite** | 极速开发体验 |
| 数据库 | **SQLite (via rusqlite)** | 轻量、无服务、本地存储 |
| 图标 | **Lucide Vue** | 风格统一、轻量 |

---

## 2. 需求分析

### 2.1 用户场景

#### 场景 1：快速启动应用
用户按下 `Alt+Space` → 输入应用名称 → 回车启动 → 搜索界面消失。

#### 场景 2：搜索文件
用户按下 `Alt+Space` → 输入文件路径/名称 → 实时毫秒级展示结果 → 回车打开文件所在目录。

#### 场景 3：执行自定义命令
用户按下 `Alt+Space` → 输入自定义命令关键字 → 展示命令 → 回车执行。

#### 场景 4：后台 CLI 使用
在终端执行 `monotools search "keyword"` 或 `monotools launch "app"` 直接执行操作。

### 2.2 功能优先级

| 优先级 | 功能 |
|--------|------|
| P0 (MVP) | 全局快捷键唤起、应用搜索、基础 UI |
| P1 | 文件搜索（NTFS）、自定义命令、主题切换、CLI |
| P2 | 插件系统、设置面板 |
| P3 | 跨平台支持（macOS/Linux）、扩展市场 |

---

## 3. 总体架构设计

### 3.1 整体架构图

```
┌─────────────────────────────────────────────────────────────┐
│                    Presentation Layer                       │
│                    (Vue 3 + PrimeVue)                       │
│  ┌─────────────┐ ┌──────────────┐ ┌─────────────────────┐  │
│  │ Search View │ │ Startup View │ │ Settings / Plugins  │  │
│  └──────┬──────┘ └──────┬───────┘ └──────────┬──────────┘  │
│         │               │                     │              │
│  ┌──────▼───────────────▼─────────────────────▼──────────┐  │
│  │              State Manager (Pinia)                      │  │
│  └──────────────────────┬──────────────────────────────────┘  │
│                         │ Tauri Event/IPC                    │
└─────────────────────────┼────────────────────────────────────┘
                          │
┌─────────────────────────┼────────────────────────────────────┐
│                    Application Layer (Rust)                  │
│  ┌──────────────────────▼──────────────────────────────────┐ │
│  │                    App Commander                          │ │
│  │  (命令路由 / CLI 解析 / 权限控制)                           │ │
│  └────────────┬──────────────────────────────┬──────────────┘ │
│               │                              │                │
│  ┌────────────▼──────────┐    ┌─────────────▼──────────────┐ │
│  │   Hotkey Service      │    │   Search Engine            │ │
│  │   (Global Shortcut)   │    │   (App/File/Command)       │ │
│  └───────────────────────┘    └─────────────┬──────────────┘ │
│  ┌───────────────────────┐    ┌─────────────▼──────────────┐ │
│  │  Startup Manager      │    │   Command Registry         │ │
│  │  (Registry/Scheduled) │    │   (Plugin/Extension)       │ │
│  └───────────────────────┘    └─────────────────────────────┘ │
│  ┌───────────────────────┐    ┌─────────────────────────────┐ │
│  │  Storage Service      │    │   Window Manager           │ │
│  │  (SQLite)             │    │   (Show/Hide/Position)     │ │
│  └───────────────────────┘    └─────────────────────────────┘ │
└───────────────────────────────────────────────────────────────┘
                          │
┌─────────────────────────▼────────────────────────────────────┐
│                    OS / System Layer                          │
│    Windows Registry │ NTFS USN Journal │ Win32 API │ Shell   │
└───────────────────────────────────────────────────────────────┘
```

### 3.2 目录结构

```
MonoTools/
├── Cargo.toml                    # Rust workspace 根
├── package.json                  # Node.js 根
├── pnpm-workspace.yaml
├── .env.example
├── .gitignore
├── tauri.conf.json
├── vite.config.ts
├── tsconfig.json
├── tailwind.config.ts
├── src-tauri/                    # ── Rust Backend ──
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs               # 入口
│   │   ├── lib.rs                # 模块声明
│   │   ├── error.rs              # 错误类型定义
│   │   ├── types.rs              # 公共类型定义
│   │   ├── app.rs                # Tauri App 构建
│   │   ├── services/
│   │   │   ├── mod.rs
│   │   │   ├── hotkey.rs         # 全局快捷键服务
│   │   │   ├── window.rs         # 窗口管理服务
│   │   │   ├── search.rs         # 搜索引擎协调
│   │   │   ├── storage.rs        # SQLite 存储服务
│   │   │   └── command.rs        # 命令注册表
│   │   ├── commands/             # Tauri Commands (API)
│   │   │   ├── mod.rs
│   │   │   ├── hotkey_cmd.rs
│   │   │   ├── search_cmd.rs
│   │   │   ├── command_cmd.rs
│   │   │   └── settings_cmd.rs
│   │   ├── engines/              # 搜索引擎
│   │   │   ├── mod.rs
│   │   │   ├── app_search.rs     # 已安装应用搜索
│   │   │   └── file_search.rs    # 文件搜索 (USN)
│   │   ├── models/               # 数据模型
│   │   │   ├── mod.rs
│   │   │   ├── app_entry.rs
│   │   │   ├── search_result.rs
│   │   │   └── settings.rs
│   │   ├── repositories/         # 数据访问层
│   │   │   ├── mod.rs
│   │   │   ├── settings_repo.rs
│   │   │   └── command_repo.rs
│   │   ├── utils/                # 工具函数
│   │   │   ├── mod.rs
│   │   │   ├── path.rs
│   │   │   └── hash.rs
│   │   └── platform/             # 平台相关代码
│   │       ├── mod.rs
│   │       └── windows/
│   │           ├── mod.rs
│   │           ├── hotkey.rs
│   │           ├── registry.rs
│   │           ├── usn.rs
│   │           └── shell.rs
│   └── tests/                    # Rust 单元测试
│       └── search_tests.rs
├── src/                          # ── Vue 3 Frontend ──
│   ├── main.ts                   # 入口
│   ├── App.vue                   # 根组件
│   ├── assets/
│   │   ├── styles/
│   │   │   ├── main.scss         # 全局样式
│   │   │   ├── theme.scss        # 主题变量
│   │   │   └── primevue.scss     # PrimeVue 覆盖
│   │   └── icons/                # SVG 图标
│   ├── components/
│   │   ├── common/               # 通用组件
│   │   │   ├── AppIcon.vue       # 应用图标组件
│   │   │   ├── ThemeToggle.vue   # 主题切换
│   │   │   ├── SearchInput.vue   # 搜索输入框
│   │   │   └── ResultItem.vue    # 搜索结果项
│   │   ├── search/               # 搜索面板
│   │   │   ├── SearchPanel.vue   # 搜索主面板
│   │   │   ├── SearchResults.vue # 搜索结果列表
│   │   │   ├── ActionBar.vue     # 底部操作栏
│   │   │   └── CategoryTabs.vue  # 分类标签
│   ├── layouts/
│   │   ├── MainLayout.vue        # 主布局
│   │   └── CompactLayout.vue     # 紧凑布局（搜索覆盖层）
│   ├── pages/
│   │   ├── SearchPage.vue        # 搜索页面
│   │   └── SettingsPage.vue      # 设置页面
│   ├── stores/                   # Pinia Stores
│   │   ├── search.ts
│   │   ├── settings.ts
│   │   └── theme.ts
│   ├── composables/              # 组合式函数
│   │   ├── useSearch.ts
│   │   ├── useHotkey.ts
│   │   ├── useTheme.ts
│   │   └── useDebounce.ts
│   ├── router/
│   │   └── index.ts
│   ├── services/                 # 前端 API 服务
│   │   ├── tauri.ts              # Tauri API 封装
│   │   ├── searchApi.ts
│   │   └── commandApi.ts
│   ├── types/                    # TypeScript 类型
│   │   ├── search.ts
│   │   ├── command.ts
│   │   └── settings.ts
│   └── utils/
│       ├── format.ts
│       └── sort.ts
├── docs/                         # ── 文档 ──
│   ├── DESIGN.md                 # 本文档
│   ├── API.md                    # API 文档
│   ├── DEPLOY.md                 # 部署文档
│   └── CHANGELOG.md
├── scripts/                      # 工具脚本
│   ├── build.sh
│   ├── dev.sh
│   └── sign.ps1                  # Windows 签名脚本
└── .claude/                      # Claude Code 配置
```

---

## 4. 前端设计

### 4.1 主题系统

#### 4.1.1 设计理念（Raycast 风格）

MonoTools 的 UI 设计以 **Raycast** 为参照，核心设计语言：

- **配色**: 以深色为主基调，使用灰度阶梯而非纯黑，减少视觉疲劳
- **圆角**: 大圆角（12px-16px），营造亲和感
- **间距**: 宽松的间距系统，呼吸感强
- **字体**: 系统默认字体栈，强调清晰可读
- **动效**: 快速、流畅的微交互，弹出/消失 < 150ms
- **透明度**: 毛玻璃效果（backdrop-blur）

#### 4.1.2 主题色板

**暗色主题（默认）**:

```scss
// theme.scss

// ── 基础色板 ──
$bg-primary:      #1a1a2e;       // 主背景
$bg-secondary:    #16213e;       // 次要背景
$bg-tertiary:     #0f3460;       // 三级背景
$bg-overlay:      rgba(15, 15, 25, 0.92);  // 覆盖层（毛玻璃）

// ── 文字色板 ──
$text-primary:    #eaeaea;       // 主文字
$text-secondary:  #a0a0b0;       // 次要文字
$text-tertiary:   #6c6c7e;       // 辅助文字

// ── 强调色 ──
$accent:          #ff6b6b;       // 主强调色（珊瑚红）
$accent-hover:    #ff8787;       // 悬停态
$accent-subtle:   rgba(255, 107, 107, 0.12);  // 微弱背景

// ── 功能色 ──
$success:         #51cf66;
$warning:         #fcc419;
$error:           #ff6b6b;
$info:            #339af0;

// ── 边框 ──
$border:          rgba(255, 255, 255, 0.08);
$border-hover:    rgba(255, 255, 255, 0.15);

// ── 阴影 ──
$shadow-sm:       0 2px 8px rgba(0, 0, 0, 0.3);
$shadow-md:       0 8px 32px rgba(0, 0, 0, 0.4);
$shadow-lg:       0 16px 64px rgba(0, 0, 0, 0.5);

// ── 圆角 ──
$radius-sm:       8px;
$radius-md:       12px;
$radius-lg:       16px;
$radius-xl:       20px;
$radius-full:     9999px;

// ── 字体 ──
$font-family:     -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC',
                  'Hiragino Sans GB', 'Microsoft YaHei', sans-serif;
$font-mono:       'JetBrains Mono', 'Fira Code', 'SF Mono', monospace;
```

**亮色主题**:

```scss
// theme.light.scss

$bg-primary:      #f5f5f7;
$bg-secondary:    #ffffff;
$bg-tertiary:     #e8e8ed;
$bg-overlay:      rgba(245, 245, 247, 0.92);

$text-primary:    #1d1d1f;
$text-secondary:  #6e6e73;
$text-tertiary:   #aeaeb2;

$accent:          #ff3b30;
$accent-hover:    #ff453a;
$accent-subtle:   rgba(255, 59, 48, 0.08);

$border:          rgba(0, 0, 0, 0.08);
$border-hover:    rgba(0, 0, 0, 0.15);

$shadow-sm:       0 2px 8px rgba(0, 0, 0, 0.06);
$shadow-md:       0 8px 32px rgba(0, 0, 0, 0.08);
$shadow-lg:       0 16px 64px rgba(0, 0, 0, 0.12);
```

#### 4.1.3 PrimeVue 主题定制

使用 PrimeVue 的 `@primeuix/themes` 进行深度定制：

```typescript
// src/assets/styles/primevue-theme.ts
import type { Theme } from '@primeuix/themes'

export const darkTheme: Theme = {
  semantic: {
    primary: {
      50:  '{surface.50}',
      100: '{surface.100}',
      // ... 使用自定义 CSS 变量映射
    },
    colorScheme: {
      light: {
        primary: { bg: '#1a1a2e', text: '#eaeaea' },
        // ...
      },
      dark: {
        primary: { bg: '#ff6b6b', text: '#ffffff' },
        // ...
      }
    }
  }
}
```

### 4.2 核心页面设计

#### 4.2.1 搜索面板（核心界面）

```
┌──────────────────────────────────────────────┐
│ [Raycast-style translucent overlay]            │
│                                               │
│   ┌─────────────────────────────────────┐     │
│   │ 🔍 Search apps, files, commands...  │     │  ← 搜索输入区
│   └─────────────────────────────────────┘     │
│                                               │
│   [Apps] [Files] [Commands] [Startup]         │  ← 分类 Tab
│                                               │
│   ┌─────────────────────────────────────┐     │
│   │ 📱  Google Chrome            ⏎ Open │     │
│   │     C:\Program Files\Google\...    │     │
│   ├─────────────────────────────────────┤     │
│   │ 📱  VS Code                  ⏎ Open │     │
│   │     C:\Program Files\Microsoft\... │     │
│   ├─────────────────────────────────────┤     │
│   │ 🔧  git status               ⏎ Run  │     │
│   │     Custom Command               │     │
│   └─────────────────────────────────────┘     │
│                                               │
│   ⚙ Settings    🎨 Theme    ❓ Help           │  ← 底部操作
└──────────────────────────────────────────────┘
```

**组件结构**:

```vue
<!-- SearchPanel.vue -->
<template>
  <Teleport to="body">
    <Transition name="fade">
      <div v-if="visible" class="search-overlay">
        <div class="search-container" @click.stop>
          <!-- 搜索输入 -->
          <div class="search-input-wrapper">
            <i class="search-icon" />
            <InputText
              ref="searchInput"
              v-model="query"
              placeholder="Search apps, files, commands..."
              @keydown="handleKeydown"
            />
            <Kbd>ESC</Kbd>
          </div>

          <!-- 分类 Tab -->
          <CategoryTabs
            :active="activeCategory"
            @select="activeCategory = $event"
          />

          <!-- 搜索结果 -->
          <ScrollPanel>
            <SearchResults
              :results="filteredResults"
              :loading="loading"
              @select="handleSelect"
            />
          </ScrollPanel>

          <!-- 底部操作栏 -->
          <ActionBar />
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
```

#### 4.2.2 启动项管理页面

```
┌──────────────────────────────────────────────┐
│                                               │
│  Startup Manager                    [+ Add]  │
│                                               │
│  ┌─────────────────────────────────────┐      │
│  │ ┌───┐  VS Code                      │      │
│  │ │ 🟢│  Enabled  ·  Delay: 5s       │ ✏ 🗑  │
│  │ └───┘  C:\Users\...\Code.exe       │      │
│  ├─────────────────────────────────────┤      │
│  │ ┌───┐  WeChat                       │      │
│  │ │ 🔴│  Disabled ·  Delay: 0s        │ ✏ 🗑  │
│  │ └───┘  C:\Program Files\Tencent\... │      │
│  ├─────────────────────────────────────┤      │
│  │ ┌───┐  Spotify                      │      │
│  │ │ 🟢│  Enabled  ·  Delay: 10s      │ ✏ 🗑  │
│  │ └───┘  C:\Users\...\Spotify.exe    │      │
│  └─────────────────────────────────────┘      │
│                                               │
└──────────────────────────────────────────────┘
```

### 4.3 状态管理

```typescript
// stores/search.ts
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { searchApi } from '@/services/searchApi'

export const useSearchStore = defineStore('search', () => {
  // State
  const query = ref('')
  const results = ref<SearchResult[]>([])
  const loading = ref(false)
  const activeCategory = ref<'all' | 'apps' | 'files' | 'commands'>('all')
  const selectedIndex = ref(0)
  const visible = ref(false)

  // Derived
  const filteredResults = computed(() => {
    if (activeCategory.value === 'all') return results.value
    return results.value.filter(r => r.category === activeCategory.value)
  })

  const topResults = computed(() => filteredResults.value.slice(0, 8))

  // Actions
  async function executeSearch(term: string) {
    loading.value = true
    try {
      results.value = await searchApi.search(term)
      selectedIndex.value = 0
    } finally {
      loading.value = false
    }
  }

  function selectNext() {
    if (selectedIndex.value < filteredResults.value.length - 1) {
      selectedIndex.value++
    }
  }

  function selectPrev() {
    if (selectedIndex.value > 0) {
      selectedIndex.value--
    }
  }

  function executeSelected() {
    const item = filteredResults.value[selectedIndex.value]
    if (item) handleSelect(item)
  }

  return {
    query, results, loading, activeCategory,
    selectedIndex, visible,
    filteredResults, topResults,
    executeSearch, selectNext, selectPrev, executeSelected
  }
})
```

### 4.4 服务层（前端 ↔ 后端）

```typescript
// services/tauri.ts
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// 类型安全的 Tauri 命令调用
export const searchService = {
  async search(query: string, options?: SearchOptions) {
    return await invoke<SearchResult[]>('search', { query, options })
  },
  async clearCache() {
    return await invoke('clear_search_cache')
  }
}

export const hotkeyService = {
  async register(hotkey: string) {
    return await invoke<string>('register_hotkey', { hotkey })
  },
  async onTrigger() {
    return listen<boolean>('hotkey-triggered', (event) => event.payload)
  }
}

export const startupService = {
  async list() {
    return await invoke<StartupItem[]>('list_startup_items')
  },
  async toggle(id: string, enabled: boolean) {
    return await invoke('toggle_startup_item', { id, enabled })
  },
  async add(item: Omit<StartupItem, 'id'>) {
    return await invoke<string>('add_startup_item', { item })
  },
  async remove(id: string) {
    return await invoke('remove_startup_item', { id })
  }
}
```

---

## 5. 后端设计

### 5.1 错误处理

```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Windows API error: {0}")]
    WindowsApi(#[from] windows::core::Error),

    #[error("Hotkey already registered: {0}")]
    HotkeyAlreadyRegistered(String),

    #[error("Startup item not found: {0}")]
    StartupItemNotFound(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Permission denied")]
    PermissionDenied,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;
```

### 5.2 通用类型

```rust
// src/types.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub icon_path: Option<PathBuf>,
    pub category: String,
    pub last_launched: Option<i64>,
    pub launch_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileResult {
    pub path: PathBuf,
    pub name: String,
    pub extension: Option<String>,
    pub size: u64,
    pub modified_at: i64,
    pub is_directory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupItem {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub enabled: bool,
    pub delay_seconds: u32,
    pub run_as_admin: bool,
    pub source: StartupSource,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StartupSource {
    RegistryRun,
    RegistryRunOnce,
    StartupFolder,
    ScheduledTask,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchOptions {
    pub categories: Vec<SearchCategory>,
    pub max_results: u32,
    pub include_hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchCategory {
    Apps,
    Files,
    Commands,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub icon: Option<String>,
    pub category: SearchCategory,
    pub action: SearchAction,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SearchAction {
    #[serde(rename = "launch")]
    Launch(String),
    #[serde(rename = "open")]
    Open(String),
    #[serde(rename = "run")]
    Run { command: String, args: Vec<String> },
    #[serde(rename = "navigate")]
    Navigate(String),
}
```

### 5.3 命令 Commander 架构

所有功能统一通过 **Command 模式** 暴露接口：

```rust
// src/commands/mod.rs
use async_trait::async_trait;

#[async_trait]
pub trait Command: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn aliases(&self) -> Vec<&'static str> { vec![] }
    async fn execute(&self, args: &[String], ctx: &CommandContext) -> Result<CommandOutput>;
}

pub struct CommandContext {
    pub search: Arc<dyn SearchEngine>,
    pub startup: Arc<dyn StartupManager>,
    pub storage: Arc<dyn Storage>,
    pub config: Arc<RwLock<Settings>>,
}

pub struct CommandOutput {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

// 注册所有命令
pub fn register_all(registry: &mut CommandRegistry) {
    registry.register(Box::new(SearchCommand));
    registry.register(Box::new(LaunchCommand));
    registry.register(Box::new(StartupListCommand));
    registry.register(Box::new(StartupToggleCommand));
    registry.register(Box::new(StartupAddCommand));
    registry.register(Box::new(ThemeCommand));
    registry.register(Box::new(ConfigCommand));
    registry.register(Box::new(HelpCommand));
    registry.register(Box::new(VersionCommand));
}

// CLI 入口
pub async fn dispatch(input: &str, ctx: &CommandContext) -> Result<CommandOutput> {
    let args: Vec<String> = shell_words::split(input)?;
    let registry = CommandRegistry::new();
    register_all(&mut registry);
    registry.execute(&args, ctx).await
}
```

### 5.4 服务层

#### 5.4.1 全局快捷键服务

```rust
// src/services/hotkey.rs
use crate::error::{AppError, Result};

pub struct HotkeyService {
    registered: RwLock<Option<String>>,
}

impl HotkeyService {
    pub fn new() -> Self {
        Self {
            registered: RwLock::new(None),
        }
    }

    /// 注册全局快捷键
    /// 使用 Windows RegisterHotKey API
    pub async fn register(&self, hotkey: &str) -> Result<()> {
        let mut guard = self.registered.write().await;
        if guard.is_some() {
            self.unregister().await?;
        }

        // 解析快捷键字符串 (如 "Alt+Space" → VK_MENU + VK_SPACE)
        let vk = parse_hotkey(hotkey)?;

        // 调用平台代码注册
        platform::hotkey::register_hotkey(vk)?;

        *guard = Some(hotkey.to_string());
        Ok(())
    }

    pub async fn unregister(&self) -> Result<()> {
        let mut guard = self.registered.write().await;
        if let Some(_) = guard.take() {
            platform::hotkey::unregister_hotkey()?;
        }
        Ok(())
    }

    pub async fn current(&self) -> Option<String> {
        self.registered.read().await.clone()
    }
}
```

#### 5.4.2 窗口管理服务

```rust
// src/services/window.rs
use tauri::{AppHandle, Window};

pub struct WindowService {
    app: AppHandle,
}

impl WindowService {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    /// 显示搜索面板（如果隐藏则显示，并聚焦输入）
    pub async fn show_search(&self) -> Result<()> {
        let window = self.get_or_create_window().await?;
        window.show()?;
        window.set_focus()?;
        // 设置窗口到鼠标位置或屏幕中央
        self.center_window(&window).await?;
        Ok(())
    }

    /// 隐藏搜索面板
    pub async fn hide(&self) -> Result<()> {
        if let Some(window) = self.app.get_window("search") {
            window.hide()?;
            window.unminimize()?;
        }
        Ok(())
    }

    /// 切换显示/隐藏
    pub async fn toggle(&self) -> Result<()> {
        if let Some(window) = self.app.get_window("search") {
            if window.is_visible()? {
                self.hide().await?;
            } else {
                self.show_search().await?;
            }
        } else {
            self.show_search().await?;
        }
        Ok(())
    }

    async fn get_or_create_window(&self) -> Result<Window> {
        if let Some(window) = self.app.get_window("search") {
            return Ok(window);
        }

        let window = tauri::WindowBuilder::new(
            &self.app,
            "search",
            tauri::WindowUrl::App("index.html".into())
        )
        .title("MonoTools")
        .inner_size(720.0, 520.0)
        .min_inner_size(480.0, 360.0)
        .decorations(false)           // 无边框窗口
        .always_on_top(true)          // 始终置顶
        .skip_taskbar(true)           // 不显示在任务栏
        .transparent(true)            // 透明背景
        .resizable(true)
        .hidden_title(true)
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .build()?;

        // 去掉窗口边框阴影（可选，Raycast 风格不需要）
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::WindowsAndMessaging::{
                SetWindowLongPtrW, GWL_EXSTYLE,
                WS_EX_TOOLWINDOW, WS_EX_NOACTIVATE,
            };
            let hwnd = window.hwnd()?;
            unsafe {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE,
                    WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0);
            }
        }

        Ok(window)
    }

    async fn center_window(&self, window: &Window) -> Result<()> {
        let monitor = window.current_monitor()?
            .ok_or_else(|| AppError::InvalidInput("No monitor found".into()))?;

        let monitor_size = monitor.size();
        let window_size = window.outer_size()?;

        let x = (monitor_size.width - window_size.width) / 2;
        let y = (monitor_size.height - window_size.height) / 3; // 偏上放置

        window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32))?;
        Ok(())
    }
}
```

#### 5.4.3 应用搜索引擎

```rust
// src/engines/app_search.rs
use crate::error::Result;
use crate::models::AppEntry;
use crate::repositories::settings_repo::SettingsRepo;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct AppSearchEngine {
    cache: RwLock<HashMap<PathBuf, AppEntry>>,
    settings: Arc<dyn SettingsRepo>,
}

impl AppSearchEngine {
    pub async fn new(settings: Arc<dyn SettingsRepo>) -> Result<Self> {
        let engine = Self {
            cache: RwLock::new(HashMap::new()),
            settings,
        };
        engine.refresh_index().await?;
        Ok(engine)
    }

    /// 扫描并索引所有已安装应用
    pub async fn refresh_index(&self) -> Result<()> {
        let mut cache = self.cache.write().await;
        cache.clear();

        // 1. 扫描开始菜单
        self.scan_start_menu(&mut cache).await?;

        // 2. 扫描桌面快捷方式
        self.scan_desktop(&mut cache).await?;

        // 3. 扫描注册表卸载项
        self.scan_registry_uninstall(&mut cache).await?;

        // 4. 扫描常用程序目录
        self.scan_program_files(&mut cache).await?;

        // 5. 加载用户自定义启动项
        self.scan_user_commands(&mut cache).await?;

        info!("Indexed {} applications", cache.len());
        Ok(())
    }

    /// 搜索应用
    pub async fn search(&self, query: &str, limit: u32) -> Vec<AppEntry> {
        if query.is_empty() {
            // 空查询返回最近使用的
            return self.get_recent(limit as usize).await;
        }

        let cache = self.cache.read().await;
        let q = query.to_lowercase();
        let terms: Vec<&str> = q.split_whitespace().collect();

        let mut scored: Vec<(AppEntry, f32)> = cache.values()
            .filter_map(|app| {
                let name_lower = app.name.to_lowercase();
                let path_lower = app.path.to_string_lossy().to_lowercase();

                let mut score = 0.0f32;

                // 精确匹配名称
                if name_lower == q { score += 100.0; }
                // 名称开头匹配
                else if name_lower.starts_with(&q) { score += 80.0; }
                // 名称包含匹配
                else if name_lower.contains(&q) { score += 50.0; }

                // 路径匹配
                for term in &terms {
                    if name_lower.contains(term) { score += 30.0; }
                    if path_lower.contains(term) { score += 10.0; }
                }

                // 使用频率加权
                score += (app.launch_count as f32) * 0.5;

                if score > 0.0 {
                    Some((app.clone(), score))
                } else {
                    None
                }
            })
            .collect();

        // 按分数排序，取前 limit 个
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.truncate(limit as usize);
        scored.into_iter().map(|(app, _)| app).collect()
    }

    async fn scan_start_menu(&self, cache: &mut HashMap<PathBuf, AppEntry>) -> Result<()> {
        // 公共开始菜单
        let common_start = dirs::join_paths(&[
            dirs::get_windows_dir()?,
            "Start Menu".into(),
            "Programs".into(),
        ])?;

        // 用户开始菜单
        let user_start = dirs::join_paths(&[
            dirs::get_local_appdata()?,
            "Microsoft".into(),
            "Windows".into(),
            "Start Menu".into(),
            "Programs".into(),
        ])?;

        for dir in [common_start, user_start] {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    self.index_shortcut(&entry.path(), cache).await?;
                }
            }
        }
        Ok(())
    }

    async fn index_shortcut(&self, path: &PathBuf, cache: &mut HashMap<PathBuf, AppEntry>) -> Result<()> {
        if !path.extension().map_or(false, |e| {
            e == "lnk" || e == "exe" || e == "bat" || e == "cmd" || e == "url"
        }) {
            return Ok(());
        }

        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        if name.is_empty() || cache.contains_key(path) {
            return Ok(());
        }

        let target_path = if path.extension().map_or(false, |e| e == "lnk") {
            platform::shell::resolve_shortcut(path)?
        } else {
            path.clone()
        };

        let entry = AppEntry {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            path: target_path,
            icon_path: None,
            category: self.categorize_app(path),
            last_launched: None,
            launch_count: 0,
        };

        cache.insert(path.clone(), entry);
        Ok(())
    }

    fn categorize_app(&self, path: &PathBuf) -> String {
        let path_str = path.to_string_lossy().to_lowercase();
        match () {
            _ if path_str.contains("microsoft") => "System".into(),
            _ if path_str.contains("visual studio") => "Development".into(),
            _ if path_str.contains("node") || path_str.contains("npm") => "Development".into(),
            _ if path_str.contains("git") => "Development".into(),
            _ if path_str.contains("chrome") || path_str.contains("firefox") || path_str.contains("edge") => "Browser".into(),
            _ if path_str.contains("discord") || path_str.contains("slack") || path_str.contains("teams") => "Communication".into(),
            _ => "Applications".into(),
        }
    }

    // ... 其他扫描方法
}
```

#### 5.4.4 文件搜索引擎（USN 方案）

```rust
// src/engines/file_search.rs
use crate::error::Result;
use crate::models::FileResult;
use std::collections::HashMap;
use std::path::PathBuf;

/// 基于 NTFS USN Journal 的高性能文件索引
pub struct UsnFileSearchEngine {
    /// USN Journal 缓存
    journal_cache: RwLock<Option<UsnJournal>>,
    /// 文件名 → USN 记录映射（内存索引）
    index: RwLock<HashMap<String, Vec<UsnRecord>>>,
    /// 上一次 USN 位置，用于增量更新
    last_usn: RwLock<u64>,
}

impl UsnFileSearchEngine {
    pub fn new() -> Self {
        Self {
            journal_cache: RwLock::new(None),
            index: RwLock::new(HashMap::new()),
            last_usn: RwLock::new(0),
        }
    }

    /// 初始化索引：读取 USN Journal 并构建内存索引
    pub async fn build_index(&self) -> Result<()> {
        info!("Building USN file index...");

        let mut journal = self.read_usn_journal().await?;
        let records = self.parse_usn_records(&journal)?;

        let mut index = self.index.write().await;
        let mut name_index: HashMap<String, Vec<UsnRecord>> = HashMap::new();

        for record in records {
            if record.file_name.is_empty() {
                continue;
            }
            let name_lower = record.file_name.to_lowercase();
            name_index.entry(name_lower).or_default().push(record);
        }

        *index = name_index;

        // 保存当前 USN 位置
        let current_usn = journal.next_usn;
        *self.last_usn.write().await = current_usn;

        info!("USN index built: {} entries", index.len());
        Ok(())
    }

    /// 增量更新：只读取变化的部分
    pub async fn update_index(&self) -> Result<()> {
        let last_usn = *self.last_usn.read().await;
        let new_records = self.read_usn_changes(last_usn).await?;

        if new_records.is_empty() {
            return Ok(());
        }

        let mut index = self.index.write().await;
        for record in new_records {
            let name_lower = record.file_name.to_lowercase();
            index.entry(name_lower).or_default().push(record);
        }

        Ok(())
    }

    /// 搜索文件（毫秒级）
    pub async fn search(&self, query: &str, limit: u32) -> Vec<FileResult> {
        if query.is_empty() {
            return vec![];
        }

        let index = self.index.read().await;
        let q = query.to_lowercase();

        let mut results: Vec<FileResult> = index
            .iter()
            .filter(|(name, _)| name.contains(&q))
            .flat_map(|(_, records)| records.iter().take(5)) // 每个匹配最多5条
            .take(limit as usize)
            .map(|record| FileResult {
                path: record.full_path.clone(),
                name: record.file_name.clone(),
                extension: record.extension.clone(),
                size: record.file_size,
                modified_at: record.last_write_time,
                is_directory: record.is_directory,
            })
            .collect();

        results.truncate(limit as usize);
        results
    }

    /// 读取 USN Journal
    async fn read_usn_journal(&self) -> Result<UsnJournal> {
        platform::windows::usn::read_usn_journal().await
    }

    /// 读取 USN 变更
    async fn read_usn_changes(&self, since_usn: u64) -> Result<Vec<UsnRecord>> {
        platform::windows::usn::read_usn_changes(since_usn).await
    }

    /// 解析 USN 记录
    fn parse_usn_records(&self, journal: &UsnJournal) -> Result<Vec<UsnRecord>> {
        platform::windows::usn::parse_usn_records(journal)
    }
}

// USN 数据结构
#[derive(Debug, Clone)]
pub struct UsnRecord {
    pub file_reference_number: u64,
    pub parent_file_reference: u64,
    pub file_name: String,
    pub full_path: PathBuf,
    pub file_size: u64,
    pub last_write_time: i64,
    pub is_directory: bool,
    pub extension: Option<String>,
}

#[derive(Debug)]
pub struct UsnJournal {
    pub usn: u64,
    pub next_usn: u64,
    pub range_start: u64,
    pub range_length: u32,
}
```

#### 5.4.5 启动项管理服务

```rust
// src/services/startup.rs
use crate::models::{StartupItem, StartupSource};
use crate::platform::windows::{registry, startup_folder};

pub struct StartupManager {
    items: RwLock<Vec<StartupItem>>,
}

impl StartupManager {
    pub async fn new() -> Result<Self> {
        let manager = Self {
            items: RwLock::new(vec![]),
        };
        manager.refresh().await?;
        Ok(manager)
    }

    /// 刷新所有启动项列表
    pub async fn refresh(&self) -> Result<()> {
        let mut items = self.items.write().await;

        items.clear();

        // 1. 读取注册表 HKCU Run
        items.extend(registry::read_run_key(
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            StartupSource::RegistryRun,
        ).await?);

        // 2. 读取注册表 HKLM Run
        items.extend(registry::read_run_key(
            "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            StartupSource::RegistryRun,
        ).await?);

        // 3. 读取启动文件夹
        items.extend(startup_folder::read_items(StartupSource::StartupFolder).await?);

        // 4. 读取计划任务
        items.extend(self.read_scheduled_tasks().await?);

        Ok(())
    }

    /// 获取所有启动项
    pub async fn list(&self) -> Vec<StartupItem> {
        self.items.read().await.clone()
    }

    /// 切换启动项启用状态
    pub async fn toggle(&self, id: &str, enabled: bool) -> Result<()> {
        let items = self.items.read().await;
        let item = items.iter().find(|i| i.id == id)
            .ok_or_else(|| AppError::StartupItemNotFound(id.into()))?;

        match item.source {
            StartupSource::RegistryRun => {
                registry::toggle_run_value(&item.command, enabled).await?;
            }
            StartupSource::StartupFolder => {
                startup_folder::toggle_item(&item.command, enabled).await?;
            }
            _ => return Err(AppError::InvalidInput(
                format!("Cannot toggle source: {:?}", item.source)
            )),
        }

        // 更新内存状态
        let mut items = self.items.write().await;
        if let Some(i) = items.iter_mut().find(|i| i.id == id) {
            i.enabled = enabled;
        }

        Ok(())
    }

    /// 添加自定义启动项
    pub async fn add(&self, item: NewStartupItem) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let startup_item = StartupItem {
            id: id.clone(),
            name: item.name,
            command: item.command,
            args: item.args,
            working_dir: item.working_dir,
            enabled: true,
            delay_seconds: item.delay_seconds,
            run_as_admin: item.run_as_admin,
            source: StartupSource::Custom,
            created_at: chrono::Utc::now().timestamp(),
        };

        registry::add_run_value(&startup_item).await?;
        self.items.write().await.push(startup_item);
        Ok(id)
    }

    /// 删除启动项
    pub async fn remove(&self, id: &str) -> Result<()> {
        let items = self.items.read().await;
        let item = items.iter().find(|i| i.id == id)
            .ok_or_else(|| AppError::StartupItemNotFound(id.into()))?;

        match item.source {
            StartupSource::RegistryRun | StartupSource::RegistryRunOnce => {
                registry::remove_run_value(&item.command).await?;
            }
            StartupSource::StartupFolder => {
                startup_folder::remove_item(&item.command).await?;
            }
            StartupSource::Custom => {
                registry::remove_custom_startup(id).await?;
            }
            StartupSource::ScheduledTask => {
                // 使用 schtasks 删除计划任务
                todo!()
            }
        }

        self.items.write().await.retain(|i| i.id != id);
        Ok(())
    }

    // ... 其他方法
}
```

### 5.5 存储服务

```rust
// src/services/storage.rs
use crate::error::Result;
use crate::models::{Settings, StartupItem, CustomCommand};
use rusqlite::{Connection, params};
use std::sync::Arc;
use tauri::AppHandle;

pub struct StorageService {
    conn: Arc<Mutex<Connection>>,
}

impl StorageService {
    pub fn new(app: &AppHandle) -> Result<Self> {
        let app_data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&app_data_dir)?;
        let db_path = app_data_dir.join("monotools.db");
        let conn = Connection::open(db_path)?;

        let service = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        service.migrate()?;
        Ok(service)
    }

    /// 数据库迁移
    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER DEFAULT (strftime('%s', 'now'))
            );

            CREATE TABLE IF NOT EXISTS custom_commands (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                command TEXT NOT NULL,
                args TEXT,
                working_dir TEXT,
                icon TEXT,
                enabled INTEGER DEFAULT 1,
                created_at INTEGER,
                last_used_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS startup_items (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                command TEXT NOT NULL,
                args TEXT,
                working_dir TEXT,
                enabled INTEGER DEFAULT 1,
                delay_seconds INTEGER DEFAULT 0,
                run_as_admin INTEGER DEFAULT 0,
                source TEXT NOT NULL,
                created_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS app_stats (
                app_path TEXT PRIMARY KEY,
                launch_count INTEGER DEFAULT 0,
                last_launched INTEGER
            );

            CREATE TABLE IF NOT EXISTS search_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query TEXT NOT NULL,
                selected_result_id TEXT,
                timestamp INTEGER
            );

            CREATE INDEX IF NOT EXISTS idx_custom_commands_name ON custom_commands(name);
            CREATE INDEX IF NOT EXISTS idx_search_history_query ON search_history(query);
        ")?;

        Ok(())
    }

    // ── Settings ──

    pub fn get_setting<T: for<'a> Deserialize<'a>>(&self, key: &str, default: T) -> Result<T> {
        let conn = self.conn.lock().unwrap();
        let value: Option<String> = conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0)
        ).optional()?;

        match value {
            Some(v) => Ok(serde_json::from_str(&v)?),
            None => Ok(default),
        }
    }

    pub fn set_setting<T: Serialize>(&self, key: &str, value: T) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let json = serde_json::to_string(&value)?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, json],
        )?;
        Ok(())
    }

    // ── Custom Commands ──

    pub fn list_commands(&self) -> Result<Vec<CustomCommand>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, description, command, args, working_dir, icon, enabled, created_at, last_used_at FROM custom_commands")?;
        let rows = stmt.query_map([], |row| {
            Ok(CustomCommand {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                command: row.get(3)?,
                args: serde_json::from_str(&row.get::<_, String>(4)?)?,
                working_dir: row.get(5)?,
                icon: row.get(6)?,
                enabled: row.get(7)?,
                created_at: row.get(8)?,
                last_used_at: row.get(9)?,
            })
        })?;

        rows.collect::<Result<Vec<_>>>()
            .map(|v| v.into_iter().flatten().collect())
    }

    pub fn add_command(&self, cmd: &CustomCommand) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO custom_commands (id, name, description, command, args, working_dir, icon, enabled, created_at, last_used_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                cmd.id,
                cmd.name,
                cmd.description,
                cmd.command,
                serde_json::to_string(&cmd.args)?,
                cmd.working_dir,
                cmd.icon,
                cmd.enabled,
                cmd.created_at,
                cmd.last_used_at,
            ],
        )?;
        Ok(())
    }

    // ... 更多方法
}
```

### 5.6 Tauri Commands 层

```rust
// src/commands/mod.rs
use crate::services::{HotkeyService, WindowService, SearchEngine, StartupManager, StorageService};
use tauri::State;

pub mod hotkey_cmd;
pub mod search_cmd;
pub mod startup_cmd;
pub mod command_cmd;
pub mod settings_cmd;

// 应用状态
pub struct AppState {
    pub hotkey: Arc<HotkeyService>,
    pub window: Arc<WindowService>,
    pub search: Arc<dyn SearchEngine>,
    pub startup: Arc<StartupManager>,
    pub storage: Arc<StorageService>,
}

// 注册所有 Tauri Commands
pub fn register_commands(app: &mut tauri::App) -> Result<()> {
    let app_handle = app.handle();
    let state = Arc::new(AppState {
        hotkey: Arc::new(HotkeyService::new()),
        window: Arc::new(WindowService::new(app_handle.clone())),
        search: Arc::new(AppSearchEngine::new(/* ... */)),
        startup: Arc::new(StartupManager::new()?),
        storage: Arc::new(StorageService::new(&app_handle)?),
    });

    app.manage(state);

    // 注册全局事件
    register_hotkey_handler(app, state.clone());

    Ok(())
}
```

---

## 6. 核心功能模块详细设计

### 6.1 全局快捷键

**调用链**:
```
System Hotkey Event
       │
       ▼
  Windows Message Loop (platform/hotkey.rs)
       │
       ▼
  Tauri Global Shortcut Event (tauri::GlobalShortcutExt)
       │
       ▼
  emit("hotkey-triggered")
       │
       ▼
  Frontend: listen("hotkey-triggered")
       │
       ▼
  WindowService::toggle()
       │
       ▼
  Show/Hide Search Panel
```

**平台实现差异**:

| 平台 | 实现方式 |
|------|----------|
| **Windows** | `RegisterHotKey` Win32 API + Tauri global shortcut fallback |
| **macOS** | Tauri `GlobalShortcutExt` + Accessibility permissions |
| **Linux** | Tauri `GlobalShortcutExt` + X11/Wayland support |

### 6.2 高性能文件搜索

**核心技术决策**:

```
方案对比:
┌──────────────┬──────────────┬──────────────┬──────────────┐
│              │  Everything  │  Spotlight   │  MonoTools   │
│              │  (USN)       │  (mdsindex)  │  (USN)       │
├──────────────┼──────────────┼──────────────┼──────────────┤
│ 索引速度     │  极快(秒级)  │  中等        │  极快(秒级)  │
│ 搜索延迟     │  <10ms       │  ~50ms       │  <10ms       │
│ 实时性       │  实时        │  有延迟      │  实时        │
│ 内存占用     │  低(~50MB)   │  高(~500MB)  │  中(~100MB)  │
│ 磁盘占用     │  极小        │  极大        │  小(~10MB)   │
└──────────────┴──────────────┴──────────────┴──────────────┘
```

**USN Journal 工作流程**:

```
┌──────────────────────────────────────────────────────────┐
│                    USN Journal 流程                        │
│                                                           │
│  1. 以管理员权限打开 NTFS 卷                              │
│     CreateFileW("\\\\.\\C:", ...)                         │
│                                                           │
│  2. 查询/创建 USN Journal                                  │
│     DeviceIoControl(FSCTL_QUERY_USN_JOURNAL)              │
│     DeviceIoControl(FSCTL_CREATE_USN_JOURNAL)             │
│                                                           │
│  3. 首次读取：读取全部 USN 记录                            │
│     DeviceIoControl(FSCTL_READ_USN_JOURNAL)               │
│     → 解析 MFT 记录 → 构建内存索引                         │
│                                                           │
│  4. 增量更新：监听 USN 变更                                │
│     DeviceIoControl(FSCTL_READ_USN_JOURNAL)               │
│     从上次 USN 位置开始读取 → 增量更新索引                   │
│                                                           │
│  5. 搜索：内存索引直接查询                                 │
│     HashMap<String, Vec<UsnRecord>>                       │
│     → O(k·n) 其中 k=查询长度, n=匹配数                     │
│                                                           │
└──────────────────────────────────────────────────────────┘
```

**关键 Rust 实现**:

```rust
// platform/windows/usn.rs
use std::os::windows::raw::HANDLE;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::Foundation::*;

pub async fn read_usn_journal() -> Result<UsnJournal> {
    // 以管理员权限打开 C: 盘
    let handle = unsafe {
        CreateFileW(
            HSTRING::from("\\\\.\\C:"),
            GENERIC_READ.0 | GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED | FILE_FLAG_NO_BUFFERING,
            None,
        )?
    };

    // 查询当前 USN Journal 状态
    let mut journal_data: [u8; 1024] = [0; 1024];
    let mut bytes_returned: u32 = 0;

    let query_journal = USN_JOURNAL_DATA {
        UsnJournalID: 0,
        FirstUsn: 0,
        NextUsn: 0,
        LowestValidUsn: 0,
        MaxUsn: 0,
        MaximumSize: 0,
        AllocationDelta: 0,
    };

    unsafe {
        DeviceIoControl(
            handle,
            FSCTL_QUERY_USN_JOURNAL,
            Some(&query_journal as *const _ as *const c_void),
            std::mem::size_of::<USN_JOURNAL_DATA>() as u32,
            Some(&mut journal_data as *mut _ as *mut c_void),
            journal_data.len() as u32,
            &mut bytes_returned,
            None,
        )?
    };

    // 解析返回的 USN_JOURNAL_DATA
    let journal = unsafe { *(journal_data.as_ptr() as *const USN_JOURNAL_DATA) };

    // 创建或查询 Journal
    // ... (具体实现)

    Ok(UsnJournal { /* ... */ })
}
```

### 6.3 启动项管理

**数据来源层次**:

```
┌─────────────────────────────────────────────┐
│         启动项数据来源（优先级从高到低）        │
├─────────────────────────────────────────────┤
│  1. 用户自定义（通过 MonoTools 添加）            │
│     → 写入 HKCU\...\Run 或专用子键             │
├─────────────────────────────────────────────┤
│  2. 启动文件夹                                │
│     %APPDATA%\Microsoft\Windows\Start Menu\   │
│          Programs\Startup                     │
├─────────────────────────────────────────────┤
│  3. 注册表当前用户 (HKCU Run)                  │
│  4. 注册表本地机器 (HKLM Run)                  │
├─────────────────────────────────────────────┤
│  5. 计划任务（系统层面，只读）                   │
└─────────────────────────────────────────────┘
```

**启动项模型**:

```rust
// 注册表读取实现
pub async fn read_run_key(key_path: &str, source: StartupSource) -> Result<Vec<StartupItem>> {
    use windows::Win32::System::Registry::*;

    let mut items = vec![];
    let hkey = HKEY_CURRENT_USER;

    unsafe {
        let mut hk: HKEY = HKEY::default();
        RegOpenKeyExW(
            hkey,
            HSTRING::from(key_path),
            0,
            KEY_READ,
            &mut hk,
        )?;

        let mut index = 0u32;
        loop {
            let mut name_buf = [0u16; 260];
            let mut value_buf = [0u16; 4096];
            let mut name_len = name_buf.len() as u32;
            let mut value_len = value_buf.len() as u32;

            let result = RegEnumValueW(
                hk,
                index,
                PWSTR(name_buf.as_mut_ptr()),
                &mut name_len,
                None,
                None,
                Some(value_buf.as_mut_ptr()),
                Some(&mut value_len),
            );

            match result {
                Ok(()) => {
                    let name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
                    let value = String::from_utf16_lossy(&value_buf[..(value_len / 2) as usize]);

                    items.push(StartupItem {
                        id: uuid::Uuid::new_v4().to_string(),
                        name,
                        command: value,
                        args: vec![],
                        working_dir: None,
                        enabled: true,
                        delay_seconds: 0,
                        run_as_admin: false,
                        source: source.clone(),
                        created_at: chrono::Utc::now().timestamp(),
                    });
                    index += 1;
                }
                Err(e) if e.code() == ERROR_NO_MORE_ITEMS.to_hresult() => break,
                Err(e) => return Err(e.into()),
            }
        }
    }

    Ok(items)
}
```

### 6.4 自定义命令

```rust
// src/models/custom_command.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommand {
    pub id: String,
    pub name: String,              // 显示名称
    pub description: Option<String>,
    pub keyword: String,           // 搜索关键字
    pub command: String,           // 要执行的命令/程序路径
    pub args: Vec<String>,         // 参数
    pub working_dir: Option<String>,
    pub icon: Option<String>,      // 图标路径或 emoji
    pub category: String,
    pub enabled: bool,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
}

// 命令执行
pub async fn execute_command(cmd: &CustomCommand) -> Result<()> {
    let mut process = Command::new(&cmd.command);
    process.args(&cmd.args);

    if let Some(ref dir) = cmd.working_dir {
        process.current_dir(dir);
    }

    if cmd.run_as_admin {
        // Windows 提权
        use windows::Win32::UI::Shell::*;
        unsafe {
            ShellExecuteW(
                None,
                HSTRING::from("runas"),
                HSTRING::from(&cmd.command),
                HSTRING::from(&cmd.args.join(" ")),
                HSTRING::from(cmd.working_dir.as_deref().unwrap_or("")),
                SW_SHOWNORMAL,
            )?;
        }
        return Ok(());
    }

    process.spawn()?;
    Ok(())
}
```

### 6.5 延迟启动机制

```rust
// src/services/delay_launcher.rs
pub struct DelayLauncher {
    tasks: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
}

impl DelayLauncher {
    /// 注册一个延迟启动任务
    pub async fn register(&self, item: &StartupItem) -> Result<()> {
        if item.delay_seconds == 0 {
            // 立即启动
            self.launch_now(item).await?;
            return Ok(());
        }

        let item = item.clone();
        let handle = spawn(async move {
            sleep(Duration::from_secs(item.delay_seconds as u64)).await;
            let _ = Self::launch_now(&item).await;
        });

        self.tasks.write().await.insert(item.id.clone(), handle);
        Ok(())
    }

    /// 立即执行启动
    async fn launch_now(item: &StartupItem) -> Result<()> {
        info!("Launching startup item: {}", item.name);
        execute_command(&item.command, &item.args)?;
        Ok(())
    }

    /// 取消所有待执行的延迟启动
    pub async fn cancel_all(&self) {
        let mut tasks = self.tasks.write().await;
        for (_, handle) in tasks.drain() {
            handle.abort();
        }
    }
}
```

---

## 7. 数据存储设计

### 7.1 数据库 Schema

```sql
-- settings: 应用配置
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER DEFAULT (strftime('%s', 'now'))
);

-- custom_commands: 自定义命令
CREATE TABLE custom_commands (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    keyword TEXT NOT NULL,
    command TEXT NOT NULL,
    args TEXT,
    working_dir TEXT,
    icon TEXT,
    category TEXT DEFAULT 'Custom',
    enabled INTEGER DEFAULT 1,
    created_at INTEGER,
    last_used_at INTEGER
);

-- startup_items: 用户管理的启动项（不含系统内置）
CREATE TABLE startup_items (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    command TEXT NOT NULL,
    args TEXT,
    working_dir TEXT,
    enabled INTEGER DEFAULT 1,
    delay_seconds INTEGER DEFAULT 0,
    run_as_admin INTEGER DEFAULT 0,
    source TEXT NOT NULL DEFAULT 'Custom',
    created_at INTEGER
);

-- app_stats: 应用使用统计（用于搜索排序）
CREATE TABLE app_stats (
    app_path TEXT PRIMARY KEY,
    launch_count INTEGER DEFAULT 0,
    last_launched INTEGER
);

-- search_history: 搜索历史
CREATE TABLE search_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    query TEXT NOT NULL,
    result_count INTEGER DEFAULT 0,
    selected_result_id TEXT,
    timestamp INTEGER
);

-- theme_presets: 主题预设
CREATE TABLE theme_presets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    mode TEXT NOT NULL,  -- light / dark / auto
    accent_color TEXT,
    created_at INTEGER
);
```

### 7.2 数据流向

```
┌─────────────────────────────────────────────┐
│             数据流向架构                        │
├─────────────────────────────────────────────┤
│                                             │
│  OS Registry ─────┐                         │
│  USN Journal ─────┤                         │
│  Start Menu ──────┤──→ 内存索引 (Search) ──→│
│  Shell Links ─────┘      ↑                  │
│                    实时搜索 / 缓存查询        │
│                                             │
│  User Input ────────→ SQLite ───────────────→│
│  (Settings,         (配置持久化、启动项、     │
│   Commands,         自定义命令、统计、历史)     │
│   Stats)             ↑                      │
│                   定期写入 / 事件触发         │
└─────────────────────────────────────────────┘
```

---

## 8. 构建与发布

### 8.1 开发环境搭建

```bash
# 1. 克隆仓库
git clone https://github.com/MonoKelvin/MonoTools.git
cd MonoTools

# 2. 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 3. 安装 Node.js (>= 18)
# 使用 pnpm
npm install -g pnpm

# 4. 安装 Windows 依赖（通过 vcpkg）
# vcpkg install windows-sdk-10.0

# 5. 安装项目依赖
pnpm install

# 6. 启动开发模式
pnpm tauri dev
```

### 8.2 构建脚本

```json
// package.json scripts
{
  "scripts": {
    "dev": "tauri dev",
    "build": "tauri build",
    "build:win": "tauri build --target x86_64-pc-windows-msvc",
    "build:win:arm": "tauri build --target aarch64-pc-windows-msvc",
    "build:mac": "tauri build --target x86_64-apple-darwin",
    "build:mac:arm": "tauri build --target aarch64-apple-darwin",
    "build:linux": "tauri build --target x86_64-unknown-linux-gnu",
    "preview": "tauri dev --no-devtools",
    "test": "pnpm test:rust && pnpm test:frontend",
    "test:rust": "cargo test --manifest-path src-tauri/Cargo.toml",
    "test:frontend": "vitest",
    "lint": "eslint src/ && cargo clippy",
    "format": "prettier --write src/ && cargo fmt",
    "icons": "node scripts/generate-icons.js",
    "sign:win": "powershell scripts/sign.ps1"
  }
}
```

### 8.3 发布配置

```json
// tauri.conf.json (关键配置)
{
  "$schema": "https://schema.tauri.app/config/2.0.0",
  "productName": "MonoTools",
  "version": "0.1.0",
  "identifier": "com.monotools.app",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "pnpm build:frontend",
    "frontendDist": "../dist",
    "publish": [
      { "provider": "github", "repo": "MonoTools", "owner": "MonoKelvin" }
    ]
  },
  "app": {
    "withGlobalTauri": true,
    "windows": [
      {
        "label": "search",
        "title": "MonoTools",
        "width": 720,
        "height": 520,
        "minWidth": 480,
        "minHeight": 360,
        "decorations": false,
        "alwaysOnTop": true,
        "skipTaskbar": true,
        "transparent": true,
        "resizable": true,
        "visible": false,
        "center": true,
        "titleBarStyle": "Overlay"
      }
    ],
    "security": {
      "csp": "default-src 'self'; connect-src 'self' http://localhost:* ws://localhost:*;"
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "windows": {
      "wix": {
        "language": ["zh-CN", "en-US"],
        "certificateThumbprint": null,
        "digestAlgorithm": "sha256"
      },
      "nsis": {
        "headerImage": "icons/installer_header.bmp",
        "sidebarImage": "icons/installer_sidebar.bmp",
        "installerIcon": "icons/icon.ico",
        "uninstallerIcon": "icons/icon.ico"
      }
    }
  },
  "plugins": {
    "globalShortcut": {
      "shortcuts": ["Alt+Space"]
    }
  }
}
```

---

## 9. 开发路线图

### Phase 0: 基建（1-2 周）

```
□ 项目脚手架搭建
  □ Tauri + Vue3 + TypeScript 项目初始化
  □ 目录结构建立
  □ CI/CD 流水线配置
  □ 代码规范配置（EditorConfig, Prettier, Clippy）

□ 基础架构
  □ 错误处理系统（AppError）
  □ 日志系统
  □ 配置管理
  □ 数据库迁移系统
```

### Phase 1: 核心 MVP（3-4 周）

```
□ 后端
  □ Hotkey 服务 + 全局快捷键
  □ 窗口管理（无边框、置顶、透明）
  □ 应用搜索（开始菜单扫描 + 注册表）
  □ 启动项管理（注册表读写）
  □ 基础 Tauri Commands

□ 前端
  □ 搜索面板 UI（Raycast 风格）
  □ 搜索输入 + 实时过滤
  □ 搜索结果列表
  □ 启动项列表页面
  □ 基础主题切换
  □ Tauri IPC 通信

□ 集成
  □ 全局快捷键 → 弹出搜索
  □ 搜索 → 显示结果 → 回车启动
  □ 启动项 CRUD
```

### Phase 2: 文件搜索（2-3 周）

```
□ USN Journal 引擎
  □ USN Journal 读取 API
  □ 首次全量索引
  □ 增量更新机制
  □ 内存索引数据结构

□ 搜索优化
  □ 模糊搜索算法
  □ 搜索结果排序（使用频率加权）
  □ 索引后台更新

□ 前端集成
  □ 文件搜索 Tab
  □ 文件类型图标
  □ 搜索中状态
```

### Phase 3: 自定义命令（1-2 周）

```
□ 命令注册表
  □ 命令 CRUD
  □ 命令分类
  □ 命令搜索

□ 执行引擎
  □ 命令执行（前台/后台）
  □ 管理员权限提权
  □ 执行结果反馈

□ CLI 模式
  □ CLI 入口程序
  □ 命令解析
  □ 输出格式化
```

### Phase 4: 体验优化（2-3 周）

```
□ 主题系统
  □ 亮色/暗色/自动
  □ PrimeVue 主题定制
  □ 动态切换

□ 使用统计
  □ 应用使用频率
  □ 搜索历史
  □ 智能推荐

□ 启动项增强
  □ 延迟启动
  □ 定时启动
  □ 条件启动（网络就绪等）

□ 性能优化
  □ 搜索索引性能
  □ 启动速度优化
  □ 内存优化
```

### Phase 5: 完善与发布（2-3 周）

```
□ 设置面板
  □ 快捷键自定义
  □ 外观设置
  □ 高级设置

□ 插件系统
  □ 插件接口定义
  □ 插件加载机制
  □ 示例插件

□ 打包发布
  □ Windows 安装包（NSIS + Wix）
  □ 代码签名
  □ 自动更新机制
  □ 发布到 GitHub Releases
```

---

## 10. 附录

### 10.1 关键依赖清单

```toml
# src-tauri/Cargo.toml [dependencies]
tauri = { version = "2.0", features = ["tray-icon", "global-shortcut"] }
tauri-plugin-shell = "2.0"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.32", features = ["bundled", "chrono"] }
thiserror = "2"
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
async-trait = "0.1"
fuzzy-matcher = "0.3"
walkdir = "2"
dirs = "5"
whoami = "1"
winapi = { version = "0.3", features = ["winreg", "shellapi", "winbase"] }
windows-sys = { version = "0.59", features = [
    "Win32_Foundation",
    "Win32_Storage_FileSystem",
    "Win32_System_IO",
    "Win32_UI_WindowsAndMessaging",
    "Win32_System_Registry",
    "Win32_System_Threading",
    "Win32_UI_Shell",
]}
glob-match = "0.2"
shell-words = "1"
log = "0.4"
env_logger = "0.11"
```

```json
// package.json [dependencies]
{
  "dependencies": {
    "vue": "^3.4.0",
    "vue-router": "^4.3.0",
    "pinia": "^2.2.0",
    "primevue": "^4.0.0",
    "@primeuix/themes": "^4.0.0",
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-shell": "^2.0.0",
    "lucide-vue-next": "^0.460.0",
    "@vueuse/core": "^12.0.0"
  },
  "devDependencies": {
    "@vitejs/plugin-vue": "^5.0.0",
    "typescript": "~5.6.0",
    "vite": "^6.0.0",
    "tailwindcss": "^3.4.0",
    "@tailwindcss/typography": "^0.5.0",
    "postcss": "^8.4.0",
    "autoprefixer": "^10.4.0",
    "prettier": "^3.4.0",
    "eslint": "^9.0.0",
    "vitest": "^2.0.0",
    "@types/node": "^22.0.0",
    "sass": "^1.80.0"
  }
}
```

### 10.2 性能指标目标

| 指标 | 目标值 | 测量方法 |
|------|--------|----------|
| 搜索响应延迟 | < 50ms | 热键 → 显示结果 |
| 应用搜索（内存索引） | < 5ms | 输入 → 过滤结果 |
| 文件搜索（USN 索引） | < 10ms | 输入 → 过滤结果 |
| 首次索引时间 | < 30s | 冷启动全量扫描 |
| 内存占用（空闲） | < 80MB | Task Manager |
| 安装包体积 | < 10MB | 压缩后安装包 |
| 启动时间 | < 500ms | 进程启动 → 可交互 |

### 10.3 安全考虑

| 方面 | 策略 |
|------|------|
| **权限最小化** | 仅请求必要权限，不使用 Tauri 的 `allowlist` 全开模式 |
| **IPC 安全** | 所有 Tauri Commands 添加输入验证，防止注入 |
| **管理员权限** | 仅在必要时（注册表 HKLM 写入）请求提权 |
| **数据安全** | 本地 SQLite 加密（如需要可开启 SQLCipher） |
| **自动更新** | 使用 Tauri Updater，签名验证 |
| **沙箱** | Tauri 2.x 默认沙箱模式 |

### 10.4 参考资料

- [Tauri 2.x 官方文档](https://tauri.app/v2/guides/)
- [USN Journal 技术原理](https://learn.microsoft.com/en-us/windows/win32/fileio/change-journals)
- [Everything SDK (voidtools)](https://www.voidtools.com/support/everything/sdk/)
- [PrimeVue 主题定制](https://primevue.org/theming/)
- [Raycast API 参考](https://developers.raycast.com/)
- [Windows RegisterHotKey API](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey)

---

## 文档变更记录

| 版本 | 日期 | 变更 | 作者 |
|------|------|------|------|
| v1.0 | 2026-07-06 | 初始设计文档 | Claude Code |

---

*本文档为 MonoTools 的完整设计指南，开发过程中应作为首要参考。遇到设计决策时优先参考本文档，若文档有遗漏则补充后继续。*
