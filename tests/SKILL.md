# MonoTools 测试框架 SKILL

本文件定义了 MonoTools 测试框架的标准结构和约定，用于指导 AI 生成新的测试模块。

## 概述

测试代码物理上统一存放在项目根 `tests/` 下，按角色分流：

- `tests/rust/`：Rust 集成测试（features / 模块子项目）
- `tests/ui/`：前端 Vitest 测试
- `tests/common/`：**两侧均可复用**的共享设施。当前仅 Rust 用了 helper 模块（`logger`/`paths`/`report`/`reporter`/`table`），以后 UI 也可在此放 helpers（公共 mock、断言工具等）
- `tests/data/`：运行时数据（gitignore）
- `tests/output/`：运行时产物（gitignore；每个模块下放 summary / log / 验证表）

根级 `tests/run.rs` 是 cargo 入口；`vitest.config.ts` 自动发现 `tests/ui/**`。

详见 [README.md](./README.md)。

## 目录结构

```
tests/
├── README.md
├── SKILL.md                        # 本文件
├── run.rs                          # cargo test 入口
├── common/                         # 双向可复用 helper（Rust: logger/paths/report/reporter/table）
├── data/                           # 测试运行时数据 (gitignore)
├── output/                         # 测试输出 (gitignore)
├── rust/
│   └── features/<module>/          # 功能模块测试
└── ui/                             # 前端 (Vitest)
    ├── commands/
    ├── stores/
    ├── utils/
    └── services/
```

## 测试路径约定（核心）

所有测试相关的绝对路径都必须用 helper 计算，**不要硬编码**。位置遵循：

| 类别 | 路径 |
|------|------|
| 仓库根 | `<manifest_dir>.parent()` |
| 测试根 | `<repo>/tests/` |
| 数据根 | `<repo>/tests/data/` |
| 输出根 | `<repo>/tests/output/` |
| 模块输出 | `<repo>/tests/output/<module>/` |

### Rust helper（`tests/common/paths.rs`）

| 调用 | 返回 |
|------|------|
| `tests_root()` | `<repo>/tests/` 目录 |
| `data_dir(module)` / `data_path(module, file)` | `<repo>/tests/data/<module>/...` |
| `output_dir(module)` / `output_path(module, file)` | `<repo>/tests/output/<module>/...` |
| `config_dir(module)` / `config_path(module, file)` | `<repo>/tests/config/<module>/...`（预留） |
| `timestamped_output_path(module, base, ext)` | `<repo>/tests/output/<module>/<base>_<timestamp>.<ext>` |
| `ensure_dir(path)` | mkdir -p |

### UI helper（vitest 已内置）

- `setActivePinia(createPinia())`：在 `beforeEach` 隔离 pinia
- `vi.spyOn(api, 'method')`：mock services
- happy-dom 提供 document / window，Node 测试自身可通过 `vi.spyOn(globalThis, ...)` mock

未来 `tests/common/ts/` 放共享的 UI 工具（路径等）时，vitest 配置的 include 仍然只扫 `tests/ui/**`，避免被误以为是测试。

## Rust 测试

### 添加新模块步骤

1. 创建目录 `tests/rust/features/<module_name>/`
2. 创建 `config.rs`、`test.rs`、`mod.rs`
3. 在 `tests/run.rs` 中加：

```rust
#[path = "rust/features/<module_name>/test.rs"]
mod <module_name>_test;
```

并在 `run_all_tests()` 函数中调用 `<module_name>_test::run_<module_name>_tests().await`。

4. 测试数据落 `tests/data/<module>/`，输出落 `tests/output/<module>/`（由 `common/paths.rs` 计算）

### 模块模板

```rust
use monotools_lib::<相关模块>;

// helper 都在 tests/common/ 下，通过 #[path = "common/xxx.rs"] 引入
#[path = "../../common/logger.rs"] mod logger;
#[path = "../../common/paths.rs"] mod paths;
#[path = "../../common/report.rs"] mod report;
#[path = "../../common/reporter.rs"] mod reporter;
#[path = "../../common/table.rs"] mod table;
#[path = "./config.rs"] mod config;

use config::<Name>TestConfig;
use logger::TestLogger;
use paths::{data_path, output_path, ensure_dir, timestamped_output_path};
use reporter::TestReporter;

const MODULE_NAME: &str = "<module_name>";

#[tokio::test]
async fn run_all_<module>_tests() {
    run_<module>_tests().await;
}

pub async fn run_<module>_tests() {
    let config = <Name>TestConfig::default();
    let mut logger = TestLogger::new(MODULE_NAME, &paths::output_dir(MODULE_NAME));
    let mut reporter = TestReporter::new("<模块中文名>");

    logger.section("测试初始化");
    // ...

    reporter.add_test("测试A");
    reporter.finish_test("测试A", t1.passed, t1.duration_ms, &t1.message);

    let output_dir = paths::output_dir(MODULE_NAME);
    ensure_dir(&output_dir);
    reporter.save(&timestamped_output_path(MODULE_NAME, "summary", "txt"));
}
```

## UI 测试

UI 测试位于 `tests/ui/<category>/<name>.test.ts`。vitest 配置在 `vitest.config.ts`：

- environment: `happy-dom`
- alias: `@/` → `<repo>/src/`
- include: `tests/ui/**/*.test.ts`

### 测试约定

- 调用 `describe()` 组织，单个 `it()` 断言具体行为
- 使用 `beforeEach(() => setActivePinia(createPinia()))` 隔离 Pinia store
- 涉及异步 timer 的场景优先 `vi.useFakeTimers()`，需要实际时间时 `await sleep`
- mock `services/*` 通过 `vi.spyOn(api, 'method')`

### 测试目录分类

| 分类 | 内容 |
|------|------|
| `tests/ui/commands/` | CommandRegistry / 命令构造器 / 快捷键匹配 |
| `tests/ui/stores/` | Pinia stores (settings/search/theme) |
| `tests/ui/utils/` | 纯函数：text / format / sort |
| `tests/ui/services/` | 单例服务：hotkeyManager 等 |

### 写 UI 测试的辅助技巧

- text 宽度依赖 Canvas；在 `tests/ui/utils/text.test.ts` 里如需替换 Canvas mock，用 `beforeEach` 给 `(globalThis as any).document.createElement('canvas')` 打桩返回恒定宽度集。
- 对 Pinia store 要用 `vi.spyOn(api, 'method')` 把 invoke 路径切换掉。

## 执行命令

```bash
# 全部前端测试
pnpm test

# 全部 Rust 集成测试
pnpm test:rust

# 详细输出某个测试
pnpm exec vitest run tests/ui/commands/registry.test.ts --reporter=verbose
cargo test --manifest-path src-tauri/Cargo.toml --test run run_all_search_engine_tests -- --nocapture
```

## 注意事项

1. **路径不要写死**：永远用 `tests/common/paths.rs::X()` 计算
2. **输出 / 数据目录不提交**：已在 `.gitignore` 添加 `tests/output/`、`tests/data/`
3. **新模块**：UI 在 `tests/ui/<category>/`；Rust 在 `tests/rust/features/<module>/`
4. **Pinia store 隔离**：每个 `describe` 用 `setActivePinia(createPinia())` 隔离
5. **shared helper** 用 `tests/common/`：未来 UI 的共享 mock / 工具也放这里（Rust + UI 都能 import）
6. **测试 pure 函数优先**：保持测试简单、确定
