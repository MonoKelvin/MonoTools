<div align="center">

# <img src="public/logo/logo_256x256.png" alt="MonoTools" width="120"/>

**MonoTools** 是一款面向 Windows 平台的轻量级桌面启动器与效率工具，采用 Raycast/Linear 设计语言。采用 **静默驻留** 模式运行，通过全局快捷键唤出 Spotlight 式搜索框，提供应用启动、NTFS USN Journal 文件搜索、自定义命令执行等核心能力。

[![Platform](https://img.shields.io/badge/平台-Windows-blue)](https://github.com/MonoKelvin/MonoTools)
[![Version](https://img.shields.io/badge/版本-0.1.0-green)](https://github.com/MonoKelvin/MonoTools)
[![License](https://img.shields.io/badge/许可证-MIT-orange)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.77%2B-orange)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.11-purple)](https://tauri.app)
[![Vue](https://img.shields.io/badge/Vue-3.5%2B-4FC08D)](https://vuejs.org)

</div>

---

## ✨ 核心特性

- 🚀 **静默驻留**: 开机自启，托盘运行，无主窗口常驻
- ⌨️ **全局唤出**: `Alt + Space` 快捷键捕获，屏幕中央弹出搜索框
- 🔍 **全局搜索**: 应用启动、文件搜索、自定义命令聚合搜索
- 📁 **文件搜索**: 基于 NTFS USN Journal 和 MFT 的高性能文件索引
- 🔧 **自定义命令**: 用户可配置快捷命令，扩展启动能力
- 🎨 **主题系统**: Raycast 风格毛玻璃暗色主题，支持亮/暗切换
- 💻 **CLI 支持**: 命令行接口与 GUI 功能同源
- 🧪 **测试框架**: 完善的后端测试体系，包含测试报告生成和路径验证

---

## 🛠️ 技术栈

### 前端

- **Vue 3** (3.5+) - Composition API + `<script setup>`
- **TypeScript** (5.7+) - 严格模式
- **PrimeVue** (4.x) - UI 组件库
- **Pinia** (3.x) - 状态管理
- **Vite** (6.x) - 构建工具
- **Tailwind CSS** (4.x) - 原子化 CSS
- **SCSS** - 全局样式与主题变量
- **Lucide Vue** - 图标库

### 后端

- **Rust** (1.77+) - 2021 Edition
- **Tauri** (2.11+) - 应用框架
- **Tokio** (1.x) - 异步运行时
- **SQLite** (0.40) - 配置与数据存储，FTS5 全文搜索
- **windows-rs** (0.62) - Win32 API 绑定（USN Journal、注册表、热键等）

---

## 🚀 快速开始

### 前置要求

- **Node.js** >= 18.0.0
- **Rust** >= 1.77.0
- **Windows 10/11** (x86_64)
- **pnpm** >= 8 (推荐)

### 安装

```bash
# 克隆仓库
git clone https://github.com/MonoKelvin/MonoTools.git
cd MTools

# 安装前端依赖
pnpm install
```

### 开发

```bash
# 启动开发模式（前端热重载 + Tauri 应用）
pnpm dev

# 或分别启动
pnpm dev:frontend     # 前端开发服务器 (http://localhost:1420)
pnpm tauri dev        # Tauri 开发窗口
```

### 构建

```shell
# 构建前端 + 打包 Tauri 桌面应用
pnpm tauri build
```

构建产物位于 `src-tauri/target/release/bundle/` 目录。

### CLI

```bash
# 运行 CLI 命令
pnpm cli search "chrome"
pnpm cli launch "C:\Program Files\..."
pnpm cli --help

# 或直接使用 cargo
cargo run --manifest-path src-tauri/Cargo.toml --bin monotools-cli -- search "chrome"
```

---

## 📖 使用指南

### 全局快捷键

| 快捷键 | 功能 |
|--------|------|
| `Alt + Space` | 唤出/隐藏搜索框 |

### 搜索模式

| 模式 | 说明 |
|------|------|
| 全局搜索 | 应用、文件、自定义命令聚合搜索 |
| 应用搜索 | 索引开始菜单、注册表、桌面快捷方式 |
| 文件搜索 | NTFS USN Journal 高速索引 |

### CLI 命令

```bash
# 搜索应用
pnpm cli search "chrome"

# 启动应用或文件
pnpm cli launch "C:\Program Files\..."

# 查看帮助
pnpm cli --help

# 查看版本
pnpm cli version

# 重建文件索引
pnpm cli index rebuild

# 查看统计信息
pnpm cli stats
```

---

## 📁 项目结构

```
MTools/
├── docs/                         # 设计文档
│   ├── DESIGN.md                 # 详细设计文档
│   └── UI_DESIGN-raycast.md       # UI 设计规范
├── src/                          # Vue 3 前端
│   ├── main.ts                   # 入口
│   ├── App.vue                   # 根组件
│   ├── assets/                   # 字体、全局样式
│   ├── components/               # 组件
│   │   ├── common/               # 通用组件
│   │   │   ├── MtButton.vue      # 按钮组件
│   │   │   ├── MtCard.vue        # 卡片组件
│   │   │   ├── MtDivider.vue     # 分割线组件
│   │   │   ├── MtInput.vue       # 输入框组件
│   │   │   ├── MtMenu.vue        # 菜单组件
│   │   │   ├── MtPanel.vue       # 面板组件
│   │   │   ├── ResultItem.vue    # 搜索结果项
│   │   │   ├── SearchInput.vue   # 搜索输入框
│   │   │   └── ThemeToggle.vue   # 主题切换
│   │   ├── panels/               # 面板组件
│   │   │   ├── CommandsPanel.vue # 命令面板
│   │   │   └── SettingsPanel.vue # 设置面板
│   │   └── search/               # 搜索相关
│   │       ├── ActionBar.vue     # 底部操作栏
│   │       ├── CategoryTabs.vue  # 分类标签
│   │       └── SearchResults.vue # 搜索结果列表
│   ├── pages/                    # 页面
│   │   ├── SearchPage.vue        # 搜索页
│   │   ├── CommandsPage.vue      # 命令页
│   │   └── SettingsPage.vue      # 设置页
│   ├── router/                   # 路由配置
│   ├── services/                 # Tauri IPC 封装
│   ├── stores/                   # Pinia Store
│   ├── types/                    # TypeScript 类型定义
│   └── utils/                    # 工具函数
├── src-tauri/                    # Rust 后端
│   ├── Cargo.toml                # Rust 依赖
│   ├── tauri.conf.json           # Tauri 配置
│   ├── tests/                    # 测试框架
│   │   ├── SKILL.md              # 测试框架规范
│   │   ├── all_tests.rs          # 统一测试入口
│   │   └── rust/                 # Rust 测试
│   │       ├── common/           # 公共工具
│   │       │   ├── mod.rs
│   │       │   ├── paths.rs      # 路径解析
│   │       │   ├── report.rs     # 测试报告生成器
│   │       │   └── table.rs      # 表格格式化工具
│   │       └── features/         # 功能模块测试
│   │           ├── search_engine/ # 搜索引擎测试
│   │           └── usn_journal/  # USN Journal 测试
│   └── src/
│       ├── main.rs               # GUI 入口
│       ├── cli_main.rs           # CLI 入口
│       ├── lib.rs                # 库入口
│       ├── app.rs                # Tauri App 构建
│       ├── app_state.rs          # 应用状态容器
│       ├── commands.rs           # Tauri IPC 命令
│       ├── error.rs              # 错误处理
│       ├── command/              # Command 模式（CLI + IPC 统一）
│       │   ├── command_trait.rs
│       │   ├── command_registry.rs
│       │   ├── command_engine.rs
│       │   ├── cmd_search.rs
│       │   ├── cmd_launch.rs
│       │   ├── cmd_open.rs
│       │   ├── cmd_command.rs
│       │   ├── cmd_config.rs
│       │   ├── cmd_help.rs
│       │   ├── cmd_version.rs
│       │   ├── cmd_index.rs
│       │   └── cmd_stats.rs
│       ├── engines/              # 搜索引擎
│       │   ├── mod.rs
│       │   ├── app_search.rs     # 应用搜索
│       │   ├── file_search.rs    # 文件搜索 (USN Journal + MFT)
│       │   └── command_search.rs # 命令搜索
│       ├── models/               # 数据模型
│       │   ├── mod.rs
│       │   ├── app_entry.rs
│       │   ├── custom_command.rs
│       │   ├── file_result.rs
│       │   ├── search_result.rs
│       │   └── settings.rs
│       ├── repositories/         # 数据访问层
│       │   ├── mod.rs
│       │   ├── settings_repo.rs
│       │   ├── command_repo.rs
│       │   └── stats_repo.rs
│       ├── services/             # 业务逻辑
│       │   ├── mod.rs
│       │   ├── app_state.rs      # 应用状态
│       │   ├── hotkey.rs         # 全局热键
│       │   ├── window.rs         # 窗口控制
│       │   ├── search.rs         # 搜索服务
│       │   └── storage.rs        # SQLite 存储
│       └── platform/windows/     # Windows 平台特定
│           ├── mod.rs
│           ├── hotkey.rs         # RegisterHotKey
│           ├── registry.rs       # 注册表读写
│           ├── shell.rs          # Shell 执行
│           └── usn.rs            # USN Journal + MFT 索引
├── public/                       # 静态资源
│   └── logo/                     # 应用图标
├── scripts/                      # 构建辅助脚本
├── package.json
├── pnpm-workspace.yaml
├── tauri.conf.json
├── tailwind.config.ts
├── vite.config.ts
└── tsconfig.json
```

---

## 🗺️ 开发路线图

### ✅ Phase 0: 基础设施 (已完成)

- [x] Tauri 2.x + Vue 3 项目初始化
- [x] pnpm workspace 配置
- [x] 开发环境搭建
- [x] Rust 测试框架建立（report、table、paths 工具）
- [x] 搜索引擎测试模块
- [x] USN Journal 测试模块

### ✅ Phase 1: MVP — 搜索面板 (已完成)

- [x] 无边框窗口 + 居中定位
- [x] 全局快捷键 `Alt+Space`
- [x] Raycast 风格 UI 主题
- [x] 应用搜索（开始菜单、注册表、桌面）
- [x] 自定义 Mt* 组件系统（MtMenu, MtInput, MtButton 等）
- [x] 背景异步索引构建
- [x] 热键注册失败自动重试

### ✅ Phase 2: 文件搜索 (进行中)

- [x] USN Journal + MFT 索引基础
- [x] SQLite FTS5 全文搜索
- [ ] 文件监控增量更新
- [ ] 搜索性能优化
- [ ] 路径重建算法优化

### ⏳ Phase 3: 自定义命令 (待开发)

- [ ] 命令注册与执行
- [ ] 参数解析
- [ ] 命令管理 UI

### ⏳ Phase 4: 性能优化与插件 (规划中)

- [ ] 虚拟滚动优化
- [ ] 插件系统基础
- [ ] 剪贴板历史

详见 [docs/DESIGN.md](docs/DESIGN.md)。

---

## 🤝 贡献

欢迎贡献代码！请遵循以下规范：

### 开发规范

- **Rust**: 遵循 [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- **Vue/TS**: 使用 Composition API + `<script setup>`
- **Commit**: 使用 [Conventional Commits](https://www.conventionalcommits.org/) 规范

```bash
feat: 新增功能
fix: 修复 bug
docs: 更新文档
style: 代码格式化
refactor: 重构
perf: 性能优化
test: 测试相关
chore: 构建/工具链
```

### 测试

- **后端测试**: 使用 `pnpm test:rust` 运行
- **前端测试**: 使用 `pnpm test` 运行 Vitest
- **测试框架**: 位于 `src-tauri/tests/`，包含测试报告生成器和路径验证工具
- **新增测试**: 参考 `src-tauri/tests/SKILL.md` 测试框架规范

---

## 📄 许可证

MIT License © 2026

---

## 🙏 致谢

本项目受到以下优秀项目的启发：

- [Raycast](https://www.raycast.com/) - 产品理念与设计语言
- [Linear](https://linear.app/) - 设计灵感
- [Alfred](https://www.alfredapp.com/) - 交互模式
- [Tauri](https://tauri.app/) - 应用框架
- [Vue.js](https://vuejs.org/) - 前端框架

---

> **注意**: 本项目目前处于早期开发阶段，功能尚未完整，不建议在生产环境使用。