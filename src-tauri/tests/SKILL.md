# Rust 测试框架 SKILL

本文件定义了 MonoTools 后端测试框架的标准结构和约定，用于指导 AI 生成新的测试模块。

## 目录结构

```
tests/
├── rust/
│   ├── common/               # 公共工具模块
│   │   ├── mod.rs
│   │   ├── logger.rs         # 测试日志输出器（输出到 output/<module>/log_*.txt）
│   │   ├── paths.rs          # 路径解析工具（相对路径）
│   │   ├── report.rs         # 测试报告生成器（旧版）
│   │   ├── reporter.rs       # 测试报告输出器（新版，带表格和详情）
│   │   └── table.rs          # 表格格式化工具（路径验证输出）
│   ├── config/               # 全局配置（可选）
│   ├── data/                 # 测试数据（运行时生成，不提交）
│   ├── output/               # 测试输出（运行时生成，不提交）
│   │   └── search_engine/
│   │       ├── summary_20260710_075820.txt
│   │       └── path_validation_20260710_075820.txt
│   └── features/             # 功能模块测试（分类存放）
│       └── search_engine/    # 文件搜索引擎测试模块（包含 USN Journal）
│           ├── mod.rs
│           ├── config.rs     # 模块配置
│           └── test.rs       # 测试代码
├── run.rs                    # 统一测试入口（单文件，运行所有模块）
├── all_tests.rs              # 测试入口代理（指向 run.rs）
└── SKILL.md                  # 本文件
```

## 新增测试模块步骤

### 1. 创建模块目录

```bash
mkdir -p tests/rust/features/<module_name>
```

### 2. 创建配置文件 (`config.rs`)

```rust
pub struct <ModuleName>TestConfig {
    pub sample_size: usize,
    pub timeout_ms: u64,
}

impl Default for <ModuleName>TestConfig {
    fn default() -> Self {
        <ModuleName>TestConfig {
            sample_size: 100,
            timeout_ms: 30000,
        }
    }
}

impl <ModuleName>TestConfig {
    pub fn from_file(_path: &str) -> Self {
        Self::default()
    }
}
```

### 3. 创建测试文件 (`test.rs`)

**核心结构：**

```rust
use std::path::PathBuf;

use monotools_lib::<相关模块>;

#[path = "../../common/paths.rs"]
mod paths;
#[path = "../../common/report.rs"]
mod report;
#[path = "../../common/table.rs"]
mod table;
#[path = "./config.rs"]
mod config;

use config::<ModuleName>TestConfig;
use paths::{data_path, output_path, ensure_dir};
use report::{TestReport, TestResults};
use table::ValidationReport;

const MODULE_NAME: &str = "<module_name>";

fn setup_test_dir() -> PathBuf {
    let dir = data_path(MODULE_NAME, "test_files");
    ensure_dir(&dir);
    dir
}

#[tokio::test]
async fn run_all_<module_name>_tests() {
    run_<module_name>_tests().await;
}

pub async fn run_<module_name>_tests() {
    let config = <ModuleName>TestConfig::default();
    let mut report = TestReport::new("<模块中文名>");
    let mut results = TestResults::new();

    let t1 = test_case_1(&config);
    results.add_result("测试名称", t1.passed, &t1.message, t1.duration_ms);
    report.add_section_item("分类", "测试名称", &format!("通过"));

    let output_dir = output_path(MODULE_NAME, "");
    ensure_dir(&output_dir);
    report.save(&output_path(MODULE_NAME, &timestamped_filename("summary", "txt")));

    println!("{}", results.generate_summary());
}

fn timestamped_filename(base: &str, ext: &str) -> String {
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    format!("{}_{}.{}", base, ts, ext)
}
```

### 4. 创建模块入口 (`mod.rs`)

```rust
pub mod config;
pub mod test;
```

### 5. 更新统一测试入口

在 `run.rs` 中添加：

```rust
#[path = "rust/features/<module_name>/test.rs"]
mod <module_name>_test;
```

并在 `run_all_tests()` 函数中调用：

```rust
<module_name>_test::run_<module_name>_tests().await;
```

## 路径解析约定

使用 `paths` 模块提供的函数：

| 函数 | 用途 | 示例路径 |
|------|------|----------|
| `config_dir(module)` | 配置文件目录 | `tests/rust/config/<module>/` |
| `data_dir(module)` | 测试数据目录 | `tests/rust/data/<module>/` |
| `output_dir(module)` | 测试输出目录 | `tests/rust/output/<module>/` |
| `config_path(module, filename)` | 配置文件路径 | `tests/rust/config/<module>/config.ini` |
| `data_path(module, filename)` | 数据文件路径 | `tests/rust/data/<module>/test.db` |
| `output_path(module, filename)` | 输出文件路径 | `tests/rust/output/<module>/summary.txt` |
| `ensure_dir(path)` | 确保目录存在 | - |

## 测试报告约定

### 输出文件结构

每个模块输出以下文件（带时间戳）：

```
output/<module_name>/
├── summary_20260710_075820.txt          # 汇总报告（包含所有测试用例结果）
├── log_20260710_075820.txt              # 测试日志（详细运行过程）
└── path_validation_20260710_075820.txt  # 路径验证报告（表格形式，可选）
```

文件名格式：`<base_name>_<timestamp>.txt`，timestamp 格式为 `YYYYMMDD_HHMMSS`

### 报告内容格式

**summary.txt:**

```
<模块名> 测试报告
================================================================================
输出时间: 2026-07-10 07:58:20

## 索引功能
索引构建: 通过, 1500000 个文件, 耗时 112345ms, 抽样验证 95/100

## 搜索功能
搜索验证: 通过, 总验证 98/100, 平均搜索时间 25ms

## USN Journal
监控测试: 通过, 总变化数 15, 创建 5, 修改 3, 删除 2, 验证 15/15

## 测试结果
总测试数: 3
通过: 3
失败: 0
```

**path_validation.txt:**

```
文件路径验证报告
================================================================================

文件名                    完整路径                                          分隔符          状态
--------------------------------------------------------------------------------------------------------------------------------------------------------------
example.txt               C:/work/path/example.txt                         --           √
void.doc                  F:/not/exists/path/void.doc                      --           ×

================================================================================
总样本数: 100
通过: 98 (98%)
未通过: 2 (2.0%)
================================================================================
```

### 使用 `TestReport`

```rust
let mut report = TestReport::new("模块名");

report.add_section_item("分类名", "项目名", "值");
report.add_section_item_with_details("分类名", "项目名", "值", &["详情行1", "详情行2"]);

report.save(&output_path(MODULE_NAME, &timestamped_filename("summary", "txt")));
```

### 使用 `TestResults`

```rust
let mut results = TestResults::new();

results.add_result("测试名", true, "", duration_ms);

let (passed, failed) = results.summary();

println!("{}", results.generate_summary());
```

### 使用 `ValidationReport`（路径验证）

```rust
let mut validation_report = ValidationReport::new("文件路径验证报告");

for result in &samples {
    let filename = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    validation_report.add_entry(&filename, &result.id, exists);
}

validation_report.save(&output_path(MODULE_NAME, &timestamped_filename("path_validation", "txt")));
```

## 测试用例命名规范

| 前缀 | 用途 | 示例 |
|------|------|------|
| `test_` | 单元测试函数 | `test_build_index` |
| `run_all_` | 测试入口（带 `#[tokio::test]`） | `run_all_search_engine_tests` |
| `run_` | 公共执行函数（可被 run.rs 调用） | `run_search_engine_tests` |
| `<name>_Result` | 测试结果结构体 | `BuildIndexResult` |

## 测试执行命令

```bash
# 运行所有测试（推荐方式）
cargo test --test all_tests

# 运行所有测试（直接调用 run.rs）
cargo test --test run

# 运行特定模块测试
cargo test --test all_tests search_engine_test::run_all_search_engine_tests

# 运行单个测试函数
cargo test --test all_tests run_all_tests

# 启用详细输出
cargo test --test all_tests -- --nocapture
```

## 现有测试模块

| 模块 | 路径 | 描述 |
|------|------|------|
| search_engine | `tests/rust/features/search_engine/` | 文件搜索引擎测试（包含索引构建、搜索验证、USN Journal 监控） |

## 注意事项

1. **路径不要写死**：始终使用 `paths` 模块提供的函数
2. **输出目录不提交**：已添加到 `.gitignore`
3. **数据目录不提交**：已添加到 `.gitignore`
4. **使用异步测试**：测试函数使用 `#[tokio::test]` 属性
5. **模块导入方式**：使用 `#[path = "..."]` 宏导入公共模块和配置
6. **测试函数结构**：提供两个入口函数
   - `run_all_xxx_tests()`：带 `#[tokio::test]` 属性，供单独运行
   - `run_xxx_tests()`：`pub async`，供 `run.rs` 调用
7. **清理测试数据**：测试结束后清理临时文件和目录
8. **错误处理**：使用 `Result` 和 `match` 处理可能的错误
9. **时间戳文件名**：使用 `timestamped_filename()` 函数生成带时间戳的输出文件名

## 模块导入模板

```rust
#[path = "../../common/paths.rs"]
mod paths;
#[path = "../../common/report.rs"]
mod report;
#[path = "../../common/table.rs"]
mod table;
#[path = "./config.rs"]
mod config;

use config::<ModuleName>TestConfig;
use paths::{data_path, output_path, ensure_dir};
use report::{TestReport, TestResults};
use table::ValidationReport;
```

## 扩展指南

### 添加新模块示例

假设要添加 `app_search` 模块：

1. 创建目录：`tests/rust/features/app_search/`
2. 创建：`tests/rust/features/app_search/config.rs`
3. 创建：`tests/rust/features/app_search/test.rs`
4. 创建：`tests/rust/features/app_search/mod.rs`
5. 更新：`tests/run.rs`

### 输出文件分类

每个模块的输出文件应包含：
- **summary.txt**：汇总报告（概览、功能测试、性能指标、测试结果）
- **path_validation.txt**：路径验证报告（表格形式，包含文件名、完整路径、状态）

### 模块合并原则

当多个测试模块逻辑上属于同一功能域时，应合并到一个模块中：
- USN Journal 测试合并到 search_engine 模块（USN 是搜索引擎增量更新的基础）
- 保持测试入口统一，避免分散的测试文件
- 合并后的模块包含多个测试子项，但共享同一输出目录和报告格式
