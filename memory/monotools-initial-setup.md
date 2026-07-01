---
name: monotools-initial-setup
description: MonoTools 项目初始框架搭建完成
metadata:
  type: project
---

# MonoTools 项目初始框架搭建完成

## 已完成的工作

### ✅ Phase 1: 基础框架搭建 (已完成)

1. **项目结构创建** (2026-07-01)
   - 创建完整的目录结构（前端 + 后端）
   - 配置 TypeScript + Vite
   - 配置 Tauri 2.0 + Rust

2. **前端框架** ✓
   - Vue 3.5 + Composition API
   - TypeScript 5.6 严格模式
   - PrimeVue 4.x + 自定义主题（Linear 风格）
   - Pinia 状态管理（待使用）
   - Vite 6 构建工具
   - Lucide Vue Next 图标库

3. **核心组件** ✓
   - SearchBox 组件（搜索框 UI）
   - useWindow composable（窗口管理）
   - useHotkey composable（热键管理，待完善）
   - 全局样式系统（CSS 变量 + PrimeVue 覆盖）

4. **后端框架** ✓
   - Tauri 2.0 应用框架
   - 命令总线（CommandBus + Parser）
   - 配置管理（ConfigStore）
   - 插件管理器（PluginManager 框架）
   - 文件索引器框架（UsnIndexer）
   - 工作区管理框架（SnapshotService）
   - 系统服务框架（热键、托盘、剪贴板等）

5. **Git 仓库** ✓
   - 初始化本地 Git 仓库
   - 推送到 GitHub: https://github.com/MonoKelvin/MonoTools
   - 分支: `dev-rust`
   - 初始提交: `3f3a7fd`
   - 修复提交: `51b2e21`

6. **构建验证** ✓
   - 前端构建成功 (`npm run build`)
   - 输出: `dist/` 目录
   - 包含: index.html + CSS + JS (约 210KB 未压缩)

## 技术栈

### 前端
- Vue 3.5 + Composition API
- TypeScript 5.6
- PrimeVue 4.x
- Pinia 2.x
- Vite 6.x
- Lucide Vue Next

### 后端
- Rust 1.80
- Tauri 2.0
- Tokio 1.x
- SQLite (rusqlite)
- windows-rs 0.58

## 下一步计划

### Phase 2: 命令总线与插件内核 (Week 3-4)
- [ ] 实现完整的命令处理器
- [ ] 实现搜索命令 (`search:files`, `search:apps`)
- [ ] 实现插件热插拔机制
- [ ] 加载内置主题插件
- [ ] 实现设置面板基础 UI

### Phase 3: 文件搜索引擎 (Week 5-7)
- [ ] 实现 USN Journal 读取
- [ ] 实现 SQLite 分表存储
- [ ] 实现多线程索引构建
- [ ] 实现搜索查询（精确/前缀/模糊/拼音）
- [ ] 实现文件监控增量更新

## 关键文件

- **前端入口**: `src/main.ts`
- **搜索组件**: `src/components/search/SearchBox.vue`
- **后端入口**: `src-tauri/src/main.rs`
- **命令总线**: `src-tauri/src/commands/bus.rs`
- **配置管理**: `src-tauri/src/config/store.rs`
- **Tauri 配置**: `src-tauri/tauri.conf.json`

## GitHub 仓库

https://github.com/MonoKelvin/MonoTools/tree/dev-rust

**[[CLAUDE.md 指令回顾]]**
- 所有文档放到 docs/ 目录 ✓
- 没有具体指令不要提交、推送 ✓
- 临时文件使用后删除（package-lock.json 应添加到 .gitignore）
- 编码过程更新文档 ✓
