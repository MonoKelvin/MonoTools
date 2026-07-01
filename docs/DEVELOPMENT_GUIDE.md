# MonoTools 开发阶段与工程规范

> 版本: v1.0  
> 日期: 2026-07-01

---

## 1. 项目目录结构

```
MonoTools/
├── .github/                      # GitHub Actions 工作流
│   └── workflows/
│       ├── ci.yml
│       └── release.yml
│
├── src-tauri/                    # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/             # Tauri 权限配置
│   │   └── default.json
│   ├── icons/                    # 应用图标
│   └── src/
│       ├── main.rs               # 入口
│       ├── lib.rs
│       ├── cli.rs                # 命令行解析
│       ├── commands/             # Tauri IPC 命令注册
│       │   ├── mod.rs
│       │   ├── bus.rs            # 命令总线
│       │   ├── parser.rs         # 命令解析器
│       │   ├── search.rs
│       │   ├── workspace.rs
│       │   ├── plugin.rs
│       │   ├── theme.rs
│       │   ├── config.rs
│       │   └── system.rs
│       ├── services/             # 核心服务
│       │   ├── mod.rs
│       │   ├── file_indexer/
│       │   │   ├── mod.rs
│       │   │   ├── usn_journal.rs
│       │   │   ├── index_store.rs
│       │   │   ├── search_engine.rs
│       │   │   └── watcher.rs
│       │   ├── workspace/
│       │   │   ├── mod.rs
│       │   │   ├── process_enum.rs
│       │   │   ├── window_info.rs
│       │   │   ├── snapshot.rs
│       │   │   └── restore.rs
│       │   ├── hotkey.rs
│       │   ├── app_scanner.rs
│       │   ├── clipboard.rs
│       │   └── tray.rs
│       ├── plugins/              # 插件系统
│       │   ├── mod.rs
│       │   ├── manager.rs
│       │   ├── loader.rs
│       │   ├── registry.rs
│       │   ├── sandbox.rs
│       │   └── api.rs
│       ├── models/               # 数据模型
│       │   ├── mod.rs
│       │   ├── file_entry.rs
│       │   ├── app_entry.rs
│       │   ├── workspace.rs
│       │   ├── search.rs
│       │   └── plugin.rs
│       ├── utils/                # 工具函数
│       │   ├── mod.rs
│       │   ├── windows_api.rs
│       │   ├── path_utils.rs
│       │   ├── pinyin.rs
│       │   └── db.rs
│       └── config/
│           ├── mod.rs
│           ├── store.rs
│           └── migration.rs
│
├── src/                          # Vue 3 前端
│   ├── main.ts
│   ├── App.vue
│   ├── assets/
│   │   ├── fonts/
│   │   └── images/
│   ├── components/               # 组件
│   │   ├── search/
│   │   │   ├── SearchBox.vue
│   │   │   ├── SearchInput.vue
│   │   │   ├── ResultList.vue
│   │   │   ├── ResultItem.vue
│   │   │   └── ResultDetail.vue
│   │   ├── workspace/
│   │   │   ├── WorkspacePanel.vue
│   │   │   ├── WorkspaceCard.vue
│   │   │   ├── WorkspaceGrid.vue
│   │   │   └── SnapshotDialog.vue
│   │   ├── plugin/
│   │   │   ├── PluginList.vue
│   │   │   ├── PluginCard.vue
│   │   │   └── PluginConfig.vue
│   │   ├── settings/
│   │   │   ├── SettingsPanel.vue
│   │   │   ├── SettingsNav.vue
│   │   │   ├── SettingsForm.vue
│   │   │   ├── HotkeySettings.vue
│   │   │   └── ThemeSettings.vue
│   │   ├── common/               # 通用组件
│   │   │   ├── MtCard.vue
│   │   │   ├── MtButton.vue
│   │   │   ├── MtInput.vue
│   │   │   ├── MtDialog.vue
│   │   │   ├── MtList.vue
│   │   │   ├── MtBadge.vue
│   │   │   ├── MtSkeleton.vue
│   │   │   └── MtEmpty.vue
│   │   └── typography/
│   │       ├── MtDisplay.vue
│   │       ├── MtHeadline.vue
│   │       ├── MtBody.vue
│   │       └── MtCaption.vue
│   ├── composables/              # 组合式函数
│   │   ├── useSearch.ts
│   │   ├── useWorkspace.ts
│   │   ├── useTheme.ts
│   │   ├── usePlugin.ts
│   │   ├── useCommand.ts
│   │   ├── useConfig.ts
│   │   ├── useHotkey.ts
│   │   └── useAnimation.ts
│   ├── stores/                   # Pinia 状态
│   │   ├── searchStore.ts
│   │   ├── workspaceStore.ts
│   │   ├── pluginStore.ts
│   │   ├── themeStore.ts
│   │   └── appStore.ts
│   ├── styles/                   # 全局样式
│   │   ├── variables.css         # CSS 变量
│   │   ├── primevue-override.css  # PrimeVue 覆盖
│   │   ├── animations.css        # 动画定义
│   │   ├── utilities.css         # 工具类
│   │   └── themes/
│   │       └── monotools.ts      # PrimeVue 预设
│   ├── utils/
│   │   ├── tauri.ts              # IPC 封装
│   │   ├── command.ts            # 命令构建
│   │   ├── format.ts             # 格式化
│   │   └── validate.ts           # 校验
│   └── types/                    # 类型定义
│       ├── search.ts
│       ├── workspace.ts
│       ├── plugin.ts
│       └── command.ts
│
├── builtin-plugins/              # 内置插件
│   └── default-theme/
│       ├── plugin.json
│       ├── theme.css
│       ├── dark.css
│       └── light.css
│
├── docs/                         # 文档
│   ├── DESIGN_SPEC.md
│   ├── ARCHITECTURE.md
│   ├── PLUGIN_SYSTEM.md
│   ├── CORE_FEATURES.md
│   ├── UI_DESIGN.md
│   ├── API_REFERENCE.md
│   ├── DATA_MODEL.md
│   └── DEVELOPMENT_GUIDE.md
│
├── scripts/                      # 构建脚本
│   ├── build.rs
│   └── setup.ps1
│
├── tests/
│   ├── unit/                     # 单元测试
│   ├── integration/              # 集成测试
│   └── e2e/                      # 端到端测试
│
├── package.json
├── vite.config.ts
├── tsconfig.json
├── tailwind.config.js
├── Cargo.toml                    # Workspace 配置
├── tauri.conf.json
└── README.md
```

---

## 2. 开发阶段规划

### Phase 1: 基础框架搭建 (Week 1-2)

**目标**: 可运行的空壳应用，支持唤出/隐藏

| 任务 | 负责人 | 验收标准 |
|------|--------|---------|
| 初始化 Tauri 2.0 + Vue 3 项目 | 全栈 | `cargo tauri dev` 成功运行 |
| 配置 PrimeVue + 主题变量 | 前端 | 搜索框显示，颜色正确 |
| 实现无边框窗口 + 居中定位 | 全栈 | `Alt+Space` 唤出窗口 |
| 实现失焦自动隐藏 | 前端 | 点击外部窗口隐藏 |
| 配置系统托盘 | 后端 | 托盘图标显示，菜单可用 |
| 实现 CLI 入口 (`daemon`) | 后端 | `monotools daemon` 启动 |
| 配置单实例 + 开机自启 | 后端 | 重复启动不新建实例 |

**产出**: 可唤出的空搜索框

### Phase 2: 命令总线与插件内核 (Week 3-4)

**目标**: 命令系统跑通，插件可加载

| 任务 | 负责人 | 验收标准 |
|------|--------|---------|
| 实现 CommandBus + Parser | 后端 | `invoke("execute_command", "ui:show")` 成功 |
| 实现 PluginManager 加载逻辑 | 后端 | 能扫描并加载内置插件 |
| 实现 plugin.json 解析 | 后端 | 校验通过，错误友好提示 |
| 实现前端 Plugin SDK 原型 | 前端 | 插件能注册搜索触发器 |
| 实现默认主题插件 | 前端 | 主题变量正确注入 |
| 实现设置面板（基础） | 前端 | 可修改配置并持久化 |

**产出**: 可加载主题插件，命令系统可用

### Phase 3: 文件搜索引擎 (Week 5-7)

**目标**: USN 索引构建，搜索可用

| 任务 | 负责人 | 验收标准 |
|------|--------|---------|
| 实现 USN Journal 读取 | 后端 | 能枚举 C: 卷所有文件 |
| 实现 SQLite 分表存储 | 后端 | 41 个 list 表创建成功 |
| 实现索引构建（多线程） | 后端 | 100万文件 < 15秒 |
| 实现前缀/模糊/拼音搜索 | 后端 | 三种查询返回正确结果 |
| 实现文件监控增量更新 | 后端 | 新建文件 1 秒内可搜到 |
| 集成到搜索框 UI | 前端 | 输入文件名实时显示结果 |
| 实现应用扫描器 | 后端 | 开始菜单应用可搜索 |

**产出**: 全局文件搜索可用

### Phase 4: 工作区管理 (Week 8-9)

**目标**: 工作区保存/恢复可用

| 任务 | 负责人 | 验收标准 |
|------|--------|---------|
| 实现窗口枚举 | 后端 | 获取所有可见窗口信息 |
| 实现进程命令行获取 | 后端 | WMI 查询成功 |
| 实现快照保存 | 后端 | JSON 文件正确生成 |
| 实现应用启动 + 窗口定位 | 后端 | 恢复后窗口位置正确 |
| 实现工作区 UI 面板 | 前端 | 卡片列表，可保存/恢复 |
| 实现工作区导入/导出 | 后端 | JSON 文件可迁移 |

**产出**: 工作区管理可用

### Phase 5: 插件系统完善 (Week 10-11)

**目标**: 第三方插件可开发、安装、热重载

| 任务 | 负责人 | 验收标准 |
|------|--------|---------|
| 实现插件权限校验 | 后端 | 越权插件被拒绝加载 |
| 实现插件热重载 | 后端 | 修改插件文件后自动重载 |
| 实现插件配置表单生成 | 前端 | 根据 JSON Schema 生成表单 |
| 实现插件市场 UI（基础） | 前端 | 可查看已安装插件 |
| 编写插件开发模板 | 全栈 | `mtools create-plugin` 可用 |
| 实现剪贴板插件（示例） | 全栈 | 展示第三方插件开发流程 |

**产出**: 插件系统完整可用

### Phase 6: UI 精打磨 (Week 12-13)

**目标**: 视觉效果、动画、交互达到设计稿标准

| 任务 | 负责人 | 验收标准 |
|------|--------|---------|
| 实现所有动画（唤出、结果、卡片） | 前端 | 与设计稿一致 |
| 实现键盘导航完整支持 | 前端 | 全程无需鼠标 |
| 实现响应式适配 | 前端 | 768px 以下正常显示 |
| 图标提取与缓存 | 后端 | 应用显示正确图标 |
| 空状态、错误状态、加载状态 | 前端 | 各状态有对应 UI |
| 性能优化（虚拟滚动、防抖） | 前端 | 60fps 流畅 |

**产出**: UI 精致可用

### Phase 7: 测试与发布 (Week 14)

**目标**: 稳定版本发布

| 任务 | 负责人 | 验收标准 |
|------|--------|---------|
| Rust 单元测试覆盖 >60% | 后端 | `cargo test` 通过 |
| 前端组件测试 | 前端 | 关键组件有测试 |
| 集成测试（搜索、工作区） | 全栈 | 端到端场景通过 |
| 性能测试（索引、搜索） | 后端 | 达到性能目标 |
| Windows 安装包打包 | 全栈 | `.msi` 安装成功 |
| 编写用户文档 | 全栈 | README + 使用指南 |

**产出**: v1.0 正式版发布

---

## 3. 工程规范

### 3.1 Rust 规范

```rust
// 命名规范
mod file_indexer;      // 模块: snake_case
struct FileEntry;        // 结构体: PascalCase
struct USNJournal;       // 缩写: 全大写
fn build_index() {}     // 函数: snake_case
const MAX_RESULTS: usize = 50; // 常量: SCREAMING_SNAKE_CASE

// 错误处理: 使用 anyhow + thiserror
use anyhow::{Result, Context};

fn read_journal() -> Result<Vec<UsnRecord>> {
    let handle = open_volume().context("Failed to open volume")?;
    // ...
}

// 异步: 使用 tokio
use tokio::time::{sleep, Duration};

// 日志: 使用 tracing
use tracing::{info, warn, error};

info!(target: "file_indexer", "Building index for volume: {}", volume);
```

### 3.2 Vue/TS 规范

```typescript
// 组件命名: PascalCase, 前缀 Mt
// SearchBox.vue -> MtSearchBox (注册时)

// 组合式函数: useXxx
export function useSearch() { }

// Store: useXxxStore
export const useSearchStore = defineStore('search', () => { })

// 类型: PascalCase, 接口前缀 I (可选)
interface SearchResult {
  id: string
  title: string
}

// Props: 显式定义，使用 withDefaults
interface Props {
  item: SearchResult
  selected?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  selected: false
})
```

### 3.3 Git 规范

```
feat: 新增文件搜索提供者
fix: 修复窗口恢复时位置偏移
docs: 更新插件开发文档
style: 调整搜索框圆角
refactor: 重构命令总线
perf: 优化索引构建速度
test: 添加工作区管理测试
chore: 更新依赖版本
```

### 3.4 提交前检查

```json
// package.json
{
  "scripts": {
    "lint": "eslint . --ext .vue,.ts",
    "lint:fix": "eslint . --ext .vue,.ts --fix",
    "format": "prettier --write \"src/**/*.{ts,vue,css}\"",
    "typecheck": "vue-tsc --noEmit",
    "test": "vitest",
    "test:rust": "cd src-tauri && cargo test"
  }
}
```

---

## 4. 构建配置

### 4.1 Vite 配置

```typescript
// vite.config.ts
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      '@components': resolve(__dirname, 'src/components'),
      '@composables': resolve(__dirname, 'src/composables'),
      '@stores': resolve(__dirname, 'src/stores'),
      '@utils': resolve(__dirname, 'src/utils'),
      '@types': resolve(__dirname, 'src/types'),
    },
  },
  build: {
    target: 'esnext',
    minify: 'terser',
    cssCodeSplit: true,
    rollupOptions: {
      output: {
        manualChunks: {
          primevue: ['primevue'],
          vendor: ['vue', 'pinia', 'vue-router'],
        },
      },
    },
  },
  // Tauri 要求
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
})
```

### 4.2 Tauri 配置

```json
{
  "productName": "MonoTools",
  "version": "1.0.0",
  "identifier": "dev.monotools.app",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build"
  },
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "MonoTools",
        "width": 800,
        "height": 520,
        "resizable": false,
        "maximizable": false,
        "minimizable": false,
        "fullscreen": false,
        "transparent": true,
        "decorations": false,
        "alwaysOnTop": true,
        "skipTaskbar": true,
        "visible": false,
        "center": true
      }
    ],
    "withGlobalTauri": true
  },
  "bundle": {
    "active": true,
    "targets": ["msi", "nsis"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/icon.ico"
    ],
    "windows": {
      "wix": {
        "language": "zh-CN"
      }
    }
  }
}
```

---

## 5. 调试指南

### 5.1 前端调试

```bash
# 启动开发服务器
pnpm tauri dev

# 打开 DevTools
# Windows: Ctrl + Shift + I
# 或在代码中: window.__TAURI_INTERNALS__.debug()
```

### 5.2 Rust 调试

```bash
# 附加调试器到 Tauri 进程
# VS Code launch.json 配置:
{
  "type": "lldb",
  "request": "attach",
  "name": "Attach to MonoTools",
  "program": "${workspaceFolder}/src-tauri/target/debug/monotools.exe"
}
```

### 5.3 日志查看

```bash
# 实时查看日志
tail -f "%LOCALAPPDATA%/MonoTools/logs/monotools-$(date +%Y-%m-%d).log"

# 设置日志级别
$env:RUST_LOG = "debug"
monotools daemon
```

---

## 6. 发布检查清单

- [ ] 版本号更新（`package.json`, `Cargo.toml`, `tauri.conf.json`）
- [ ] CHANGELOG 更新
- [ ] 所有测试通过
- [ ] 图标资源完整
- [ ] 内置插件打包到 resources
- [ ] 代码签名证书配置
- [ ] MSI 安装测试（干净环境）
- [ ] 自动更新配置检查
- [ ] 文档链接可用