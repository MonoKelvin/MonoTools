---
name: monotools-initial-setup
description: MonoTools 项目初始框架搭建完成
metadata:
  type: project
---

# MonoTools 项目开发进度

## 已完成的工作

### ✅ Phase 1: 基础框架搭建 (100% 完成)

1. **项目结构创建** (2026-07-01)
   - 创建完整的目录结构（前端 + 后端）
   - 配置 TypeScript + Vite
   - 配置 Tauri 2.0 + Rust

2. **前端框架** ✓
   - Vue 3.5 + Composition API
   - TypeScript 5.6 严格模式
   - PrimeVue 4.x + 自定义主题（Linear 风格）
   - Pinia 状态管理
   - Vite 6 构建工具
   - Lucide Vue Next 图标库

3. **核心组件** ✓
   - SearchBox 组件（搜索框 UI）
   - useWindow composable（窗口管理）
   - useHotkey composable（热键管理）
   - 全局样式系统

4. **后端框架** ✓
   - Tauri 2.0 应用框架
   - 命令总线（CommandBus + Parser）
   - 配置管理（ConfigStore）
   - 插件管理器（PluginManager）
   - 文件索引器框架（UsnIndexer + IndexStore）
   - 工作区管理框架（SnapshotService + WindowEnumerator）
   - 系统服务框架（热键、托盘、剪贴板等）

5. **Git 仓库** ✓
   - 初始化本地 Git 仓库
   - 推送到 GitHub: https://github.com/MonoKelvin/MonoTools
   - 分支: `dev-rust`
   - 初始提交: `3f3a7fd`

6. **构建验证** ✓
   - 前端构建成功 (`npm run build`)
   - 输出: `dist/` 目录

---

### ✅ Phase 2: 命令总线与插件内核 (进行中 70%)

#### 2.1 命令处理器实现 ✓

已实现的命令：
- ✅ `search:files` - 文件搜索
- ✅ `search:apps` - 应用搜索（框架）
- ✅ `search:all` - 聚合搜索
- ✅ `workspace:save` - 保存工作区
- ✅ `workspace:restore` - 恢复工作区
- ✅ `workspace:list` - 列出工作区
- ✅ `plugin:list` - 列出插件
- ✅ `theme:set` - 设置主题
- ✅ `theme:list` - 列出主题
- ✅ `config:get` - 获取配置
- ✅ `config:set` - 设置配置
- ✅ `system:shutdown` - 关机/重启
- ✅ `system:lock` - 锁定工作站
- ✅ `system:open` - 打开路径

**提交**: `0ba48c1`

#### 2.2 插件系统框架 ✓

- ✅ PluginLoader（扫描、解析、加载插件）
- ✅ PluginRegistry（搜索提供者、命令、视图、主题注册表）
- ✅ PluginSandbox（权限检查和验证）
- ✅ PluginApi（插件上下文、生命周期钩子）
- ✅ PluginManager 更新（集成沙箱和加载器）

**提交**: `d6afd4e`

#### 2.3 MIT 许可证 ✓
- ✅ 添加 LICENSE 文件

---

## 待完成的工作

### Phase 2 剩余任务

- [ ] 实现默认主题插件（`builtin:default-theme`）
- [ ] 实现应用扫描器（`builtin:app-launcher`）
- [ ] 实现设置面板基础 UI
- [ ] 完成文件索引器（USN Journal 读取）

### Phase 3: 文件搜索引擎 (Week 5-7)
- [ ] 实现 USN Journal 读取
- [ ] 实现 SQLite 分表存储
- [ ] 实现多线程索引构建
- [ ] 实现搜索查询（精确/前缀/模糊/拼音）
- [ ] 实现文件监控增量更新

### Phase 4: 工作区管理 (Week 8-9)
- [ ] 窗口枚举完整实现
- [ ] 快照保存/恢复完整实现
- [ ] 工作区 UI 面板

### Phase 5-7: 后续完善
- [ ] 插件系统完善
- [ ] UI 精打磨
- [ ] 测试与发布

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

## GitHub 仓库

https://github.com/MonoKelvin/MonoTools/tree/dev-rust

**[[CLAUDE.md 指令回顾]]**
- 所有文档放到 docs/ 目录 ✓
- 没有具体指令不要提交、推送 ✓
- `package-lock.json` 已添加到 .gitignore ✓
- 编码过程更新文档 ✓
- 添加 MIT 许可证 ✓

