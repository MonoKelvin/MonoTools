# MonoTools

**MonoTools** 是一款面向 Windows 平台的高性能桌面启动器与效率工具。采用 **静默驻留** 模式运行，通过全局快捷键唤出 Spotlight 式搜索框，提供应用启动、全局文件搜索、工作区管理、快捷命令执行等核心能力。

![Platform](https://img.shields.io/badge/平台-Windows-blue)
![Version](https://img.shields.io/badge/版本-1.0.0-green)
![License](https://img.shields.io/badge/许可证-MIT-orange)

---

## ✨ 核心特性

- 🚀 **静默驻留**: 开机自启，托盘运行，无主窗口常驻
- ⌨️ **全局唤出**: `Alt + Space` 快捷键捕获，屏幕中央弹出搜索框
- 🔍 **全局搜索**: 应用启动、文件搜索、工作区、命令聚合搜索
- 📁 **文件搜索**: 基于 NTFS USN Journal 的高性能文件索引
- 💼 **工作区管理**: 保存/恢复桌面状态快照
- 🔌 **插件驱动**: 功能通过插件扩展，支持热插拔
- 🎨 **主题系统**: 主题即插件，支持深度定制
- 💻 **CLI 支持**: 命令行接口与 GUI 功能同源

---

## 🛠️ 技术栈

### 前端
- **Vue 3** (3.5+) - Composition API + `<script setup>`
- **TypeScript** (5.5+) - 严格模式
- **PrimeVue** (4.x) - UI 组件库
- **Pinia** (2.x) - 状态管理
- **Vite** (6.x) - 构建工具

### 后端
- **Rust** (1.80+) - 2021 Edition
- **Tauri** (2.x) - 应用框架
- **Tokio** (1.x) - 异步运行时
- **SQLite** (3.x) - 文件索引、配置存储
- **windows-rs** (0.58+) - Win32 API 绑定

---

## 🚀 快速开始

### 前置要求

- **Node.js** >= 18.0.0
- **Rust** >= 1.80.0
- **Windows 10/11** (当前仅支持 Windows)

### 安装

```bash
# 克隆仓库
git clone https://github.com/MonoKelvin/MonoTools.git
cd MonoTools

# 安装依赖
npm install

# 安装 Tauri CLI
npm install -g @tauri-apps/cli
```

### 开发

```bash
# 启动开发模式
npm run tauri:dev

# 或分别启动
npm run dev         # 前端开发服务器 (http://localhost:1420)
npm run tauri dev   # Tauri 开发窗口
```

### 构建

```bash
npm run tauri:build
```

构建产物位于 `src-tauri/target/release/bundle/` 目录。

---

## 📖 使用指南

### 全局快捷键

| 快捷键 | 功能 |
|--------|------|
| `Alt + Space` | 唤出/隐藏搜索框 |

### 搜索模式

| 前缀 | 模式 | 示例 |
|------|------|------|
| (无) | 全局搜索 | `report.docx` |
| `>` | 命令模式 | `>settings` |
| `=` | 计算器 | `=1+2*3` |
| `?` | 帮助 | `?workspace` |

### CLI 命令

```bash
# 启动守护进程
monotools daemon

# 搜索文件
monotools search files "report.docx" --limit 20

# 保存工作区
monotools workspace save "开发环境" --auto-start

# 列出插件
monotools plugin list --enabled-only
```

---

## 📁 项目结构

```
MonoTools/
├── docs/                         # 设计文档
│   ├── DESIGN_SPEC.md           # 总体设计规范
│   ├── ARCHITECTURE.md          # 系统架构
│   ├── PLUGIN_SYSTEM.md         # 插件系统设计
│   ├── CORE_FEATURES.md         # 核心功能设计
│   ├── UI_DESIGN.md             # UI 设计规范
│   ├── API_REFERENCE.md         # API 参考
│   ├── DATA_MODEL.md            # 数据模型
│   └── DEVELOPMENT_GUIDE.md     # 开发指南
├── src/                          # Vue 3 前端
│   ├── main.ts                  # 入口
│   ├── App.vue                  # 根组件
│   ├── components/              # 组件
│   │   ├── search/              # 搜索组件
│   │   ├── workspace/           # 工作区组件
│   │   ├── settings/            # 设置面板
│   │   └── common/              # 通用组件
│   ├── composables/             # 组合式函数
│   ├── stores/                  # Pinia Store
│   ├── styles/                  # 全局样式
│   └── utils/                   # 工具函数
├── src-tauri/                    # Rust 后端
│   ├── Cargo.toml               # Rust 依赖
│   ├── tauri.conf.json          # Tauri 配置
│   ├── capabilities/            # Tauri 权限配置
│   └── src/
│       ├── main.rs              # 入口
│       ├── cli.rs               # CLI 解析
│       ├── commands/            # Tauri IPC 命令
│       │   ├── bus.rs           # 命令总线
│       │   └── parser.rs        # 命令解析器
│       ├── services/            # 核心服务
│       │   ├── file_indexer/    # 文件索引
│       │   ├── workspace/       # 工作区管理
│       │   ├── hotkey.rs        # 全局热键
│       │   └── tray.rs          # 系统托盘
│       ├── plugins/             # 插件系统
│       │   └── manager.rs       # 插件管理器
│       ├── models/              # 数据模型
│       ├── utils/               # 工具函数
│       └── config/              # 配置管理
├── builtin-plugins/              # 内置插件
│   └── default-theme/
├── package.json
├── vite.config.ts
├── tsconfig.json
└── README.md
```

---

## 🗺️ 开发路线图

### ✅ Phase 1: 基础框架 (Week 1-2)
- [x] Tauri 2.0 + Vue 3 项目初始化
- [x] PrimeVue 4 + 主题系统
- [x] 无边框窗口 + 居中定位
- [x] 全局快捷键 Alt+Space
- [x] 系统托盘
- [x] CLI 入口

### ⏳ Phase 2: 命令总线与插件内核 (Week 3-4)
- [ ] 命令系统跑通
- [ ] 插件热插拔机制
- [ ] 默认主题插件

### ⏳ Phase 3: 文件搜索引擎 (Week 5-7)
- [ ] USN Journal 索引构建
- [ ] SQLite 分表存储
- [ ] 前缀/模糊/拼音搜索
- [ ] 文件监控增量更新

### ⏳ Phase 4: 工作区管理 (Week 8-9)
- [ ] 窗口枚举
- [ ] 快照保存/恢复
- [ ] UI 面板

### ⏳ Phase 5: 插件系统完善 (Week 10-11)
- [ ] 权限系统
- [ ] 插件配置
- [ ] 插件市场基础

### ⏳ Phase 6: UI 精打磨 (Week 12-13)
- [ ] 动画效果
- [ ] 键盘导航
- [ ] 响应式适配

### ⏳ Phase 7: 测试与发布 (Week 14)
- [ ] 单元/集成测试
- [ ] 性能优化
- [ ] 打包发布

---

## 🤝 贡献

欢迎贡献代码！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详情。

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

MIT License © 2026 MonoKelvin

---

## 📞 联系方式

- **作者**: MonoKelvin
- **邮箱**: monokelvin@example.com
- **GitHub**: https://github.com/MonoKelvin/MonoTools

---

## 🙏 致谢

本项目受到以下优秀项目的启发：

- [Linear](https://linear.app/) - 设计灵感
- [Raycast](https://www.raycast.com/) - 产品理念
- [Alfred](https://www.alfredapp.com/) - 交互模式
- [Tauri](https://tauri.app/) - 应用框架
- [Vue.js](https://vuejs.org/) - 前端框架

---

> **注意**: 本项目目前处于早期开发阶段，功能尚未完整，不建议在生产环境使用。
