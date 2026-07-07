<div align="center">

# <img src="public/logo/logo_256x256.png" alt="MonoTools" width="120"/>

**MonoTools** 是一款面向 Windows 平台的轻量级桌面启动器与效率工具，采用 Raycast/Linear 设计语言。采用 **静默驻留** 模式运行，通过全局快捷键唤出 Spotlight 式搜索框，提供应用启动、全局文件搜索、自定义命令执行、开机自启项管理等核心能力。

[![Platform](https://img.shields.io/badge/平台-Windows-blue)](https://github.com/MonoKelvin/MonoTools)
[![Version](https://img.shields.io/badge/版本-0.1.0-green)](https://github.com/MonoKelvin/MonoTools)
[![License](https://img.shields.io/badge/许可证-MIT-orange)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.77%2B-orange)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-purple)](https://tauri.app)
[![Vue](https://img.shields.io/badge/Vue-3.5%2B-4FC08D)](https://vuejs.org)

</div>

---

## ✨ 核心特性

- 🚀 **静默驻留**: 开机自启，托盘运行，无主窗口常驻
- ⌨️ **全局唤出**: `Alt + Space` 快捷键捕获，屏幕中央弹出搜索框
- 🔍 **全局搜索**: 应用启动、文件搜索、自定义命令聚合搜索
- 📁 **文件搜索**: 基于 NTFS USN Journal 的高性能文件索引
- ⚙️ **自启管理**: 统一管理 Windows 所有自启位置（注册表、启动文件夹、计划任务）
- 🔌 **自定义命令**: 用户可配置快捷命令，扩展启动能力
- 🎨 **主题系统**: Raycast 风格毛玻璃暗色主题，支持亮/暗切换
- 💻 **CLI 支持**: 命令行接口与 GUI 功能同源

---

## 🛠️ 技术栈

### 前端

- **Vue 3** (3.5+) - Composition API + `<script setup>`
- **TypeScript** (5.5+) - 严格模式
- **PrimeVue** (4.x) - UI 组件库
- **Pinia** (2.x) - 状态管理
- **Vite** (6.x) - 构建工具
- **Tailwind CSS** (3.x) - 原子化 CSS
- **SCSS** - 全局样式与主题变量

### 后端

- **Rust** (1.77+) - 2021 Edition
- **Tauri** (2.x) - 应用框架
- **Tokio** (1.x) - 异步运行时
- **SQLite** (3.x) - 配置与数据存储
- **windows-rs** - Win32 API 绑定（USN Journal、注册表、热键等）

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
pnpm tauri dev

# 或分别启动
pnpm dev          # 前端开发服务器 (http://localhost:1420)
pnpm tauri dev    # Tauri 开发窗口
```

### 构建

```shell
# 构建前端 + 打包 Tauri 桌面应用
pnpm tauri build
```

构建产物位于 `src-tauri/target/release/bundle/` 目录。

### CLI

```bash
# 编译 CLI 二进制
cd src-tauri && cargo build --bin monotools-cli

# 使用示例
monotools-cli search "chrome"
monotools-cli launch "C:\Program Files\..."
monotools-cli startup list
monotools-cli startup toggle <id>
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
| 自启管理 | 管理 Windows 自启动项 |

### CLI 命令

```bash
# 搜索应用
monotools-cli search "chrome"

# 启动应用
monotools-cli launch "C:\Program Files\..."

# 列出自启项
monotools-cli startup list

# 切换自启项状态
monotools-cli startup toggle <id>
```

---

## 📁 项目结构

```
MTools/
├── docs/                         # 设计文档
│   └── DESIGN.md                 # 详细设计文档
├── src/                          # Vue 3 前端
│   ├── main.ts                   # 入口
│   ├── App.vue                   # 根组件
│   ├── assets/                   # 字体、全局样式
│   ├── components/               # 组件
│   │   ├── common/               # 公共组件
│   │   ├── search/               # 搜索相关
│   │   └── startup/              # 自启管理
│   ├── pages/                    # 页面
│   │   ├── SearchPage.vue        # 搜索页
│   │   ├── CommandsPage.vue      # 自定义命令页
│   │   ├── StartupPage.vue       # 自启管理页
│   │   └── SettingsPage.vue      # 设置页
│   ├── router/                   # 路由配置
│   ├── services/                 # Tauri IPC 封装
│   ├── stores/                   # Pinia Store
│   ├── types/                    # TypeScript 类型定义
│   └── utils/                    # 工具函数
├── src-tauri/                    # Rust 后端
│   ├── Cargo.toml                # Rust 依赖
│   ├── tauri.conf.json           # Tauri 配置
│   └── src/
│       ├── main.rs               # GUI 入口
│       ├── cli_main.rs           # CLI 入口
│       ├── commands.rs           # Tauri IPC 命令
│       ├── command/              # Command 模式（CLI + IPC 统一）
│       ├── engines/              # 搜索引擎
│       │   ├── app_search.rs     # 应用搜索
│       │   ├── file_search.rs    # 文件搜索
│       │   ├── command_search.rs # 命令搜索
│       │   └── startup_search.rs # 自启搜索
│       ├── models/               # 数据模型
│       ├── repositories/         # 数据访问层
│       ├── services/             # 业务逻辑
│       │   ├── hotkey.rs         # 全局热键
│       │   ├── window.rs         # 窗口控制
│       │   ├── search.rs         # 搜索服务
│       │   ├── startup.rs        # 自启管理
│       │   └── storage.rs        # SQLite 存储
│       └── platform/windows/     # Windows 平台特定
│           ├── hotkey.rs         # RegisterHotKey
│           ├── registry.rs       # 注册表读写
│           ├── shell.rs          # Shell 执行
│           ├── startup_folder.rs # 启动文件夹
│           └── usn.rs            # USN Journal
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

### ✅ Phase 1: MVP — 搜索面板 (已完成)

- [x] 无边框窗口 + 居中定位
- [x] 全局快捷键 `Alt+Space`
- [x] Raycast 风格 UI 主题
- [x] 应用搜索（开始菜单、注册表、桌面）

### ✅ Phase 2: 开机自启管理器 (已完成)

- [x] 注册表 Run/RunOnce 读写
- [x] 启动文件夹管理
- [x] 计划任务集成
- [x] 启用/禁用/延迟启动

### ⏳ Phase 3: 文件搜索 (进行中)

- [x] USN Journal 索引基础
- [ ] 文件监控增量更新
- [ ] 搜索性能优化

### ⏳ Phase 4: 自定义命令 (待开发)

- [ ] 命令注册与执行
- [ ] 参数解析
- [ ] 命令管理 UI

### ⏳ Phase 5: 性能优化与插件 (规划中)

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