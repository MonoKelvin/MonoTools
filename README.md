<div align="center">

# <img src="public/logo/logo_256x256.png" alt="MonoTools" width="120"/>

**MonoTools** 是一款面向 Windows 平台的轻量级桌面启动器与效率工具，采用 Raycast/Linear 设计语言。静默驻留系统托盘，通过 `Alt + Space` 全局快捷键唤出 Spotlight 式搜索框，提供应用启动、文件搜索、自定义命令、智能推荐等核心能力。

[![Platform](https://img.shields.io/badge/平台-Windows-blue)](https://github.com/MonoKelvin/MonoTools)
[![Version](https://img.shields.io/badge/版本-0.1.0-green)](https://github.com/MonoKelvin/MonoTools)
[![License](https://img.shields.io/badge/许可证-MIT-orange)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.77%2B-orange)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.11-purple)](https://tauri.app)
[![Vue](https://img.shields.io/badge/Vue-3.5%2B-4FC08D)](https://vuejs.org)

</div>

---

## ✨ 核心功能

- 🚀 **全局搜索** — 应用、文件、自定义命令聚合搜索，毫秒级响应
- 📁 **文件搜索** — 基于 NTFS USN Journal 和 MFT 的高性能文件索引，支持全文搜索
- 🤖 **智能推荐** — 基于使用频率、最近访问和前台窗口上下文的混合推荐算法
- � **自定义命令** — 可配置快捷命令，扩展启动能力
- 🎨 **精美 UI** — Raycast 风格毛玻璃主题，支持亮/暗模式切换
- ⌨️ **全局快捷键** — `Alt + Space` 一键唤出，开机自启静默驻留
- 💻 **CLI 支持** — 命令行接口与 GUI 功能同源
- 🔒 **单例模式** — 确保只运行一个实例，重复启动自动激活已有窗口

---

## 🛠️ 技术栈

- **前端**：Vue 3 + TypeScript + Vite + Pinia + PrimeVue + Tailwind CSS + SCSS
- **后端**：Rust + Tauri 2 + Tokio + SQLite (FTS5)
- **搜索**：NTFS USN Journal + MFT 索引 + 模糊匹配 + Trie
- **推荐**：规则引擎 + Python 混合推荐 (JSON-RPC)
- **Windows**：windows-rs (Win32 API) + 全局热键 + 注册表

---

## 🚀 快速开始

### 环境要求

- **Node.js** >= 18.0.0
- **Rust** >= 1.77.0
- **pnpm** >= 8
- **Windows 10/11** (x86_64)
- **Python** >= 3.10（可选，用于 AI 推荐）

### 安装

```bash
# 克隆仓库
git clone https://github.com/MonoKelvin/MonoTools.git
cd MTools

# 安装依赖
pnpm install
```

### 开发

```bash
# 启动开发模式（前端热重载 + Tauri 应用）
pnpm dev
```

### 构建

```bash
# 构建桌面应用
pnpm tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`。

### CLI 使用

```bash
pnpm cli search "chrome"    # 搜索应用
pnpm cli launch <路径>       # 启动应用/文件
pnpm cli --help              # 查看帮助
```

---

## 📖 使用说明

### 基础操作

| 操作 | 说明 |
|------|------|
| `Alt + Space` | 唤出/隐藏搜索框 |
| 输入关键词 | 实时搜索应用、文件、命令 |
| `Enter` | 执行选中项 |
| `Esc` | 关闭搜索框 |

> 每个分组独立管理选中状态，切换分组自动清空。

---

## 🤝 参与贡献

欢迎贡献代码！请遵循以下规范：

- **Rust**：遵循 Rust API Guidelines
- **Vue/TS**：Composition API + `<script setup>` + 严格 TypeScript
- **Commit**：Conventional Commits 规范
- **测试**：前端 Vitest + 后端 Cargo 测试，统一在 `tests/` 目录下

更多开发规范详见 [CLAUDE.md](CLAUDE.md)。

---

## 📄 许可证

MIT License © 2026

---

## 🙏 致谢

- [Raycast](https://www.raycast.com/) — 产品理念与设计语言
- [Linear](https://linear.app/) — 设计灵感
- [Tauri](https://tauri.app/) — 应用框架
- [Vue.js](https://vuejs.org/) — 前端框架

---

> ⚠️ 本项目处于早期开发阶段，功能尚不完整。
