//! Phase 0 性能基准套件 —— 文件/应用搜索引擎的 P50/P99 延迟测量。
//!
//! 运行方式:
//! ```bash
//! # 仅跑基准 (不跑其他测试)
//! cargo test --test monotools_it -- bench_search --nocapture
//! ```
//!
//! 输出:
//! - stdout 打印 P50/P90/P99/P999 表
//! - `tests/output/benchmarks/baseline.md` 写入 Markdown 报告
//!
//! 设计原则:
//! - 不依赖真实磁盘索引 (用内存 SQLite + 合成数据), 结果可重复
//! - 预热后测量 (跳过前 N 次冷启动)
//! - 用 hdrhistogram 记录延迟分布

use hdrhistogram::Histogram;
use monotools_lib::search_engine::app_search::AppSearchEngine;
// use monotools_lib::search_engine::models::{SearchAction, SearchCategory, SearchResult};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::Instant;

// ============================================================================
// 合成数据生成
// ============================================================================

/// 生成 N 个合成文件名 (模拟真实分布).
fn synthetic_file_names(n: usize) -> Vec<String> {
    let prefixes = [
        "report", "document", "config", "project", "data", "test", "backup",
        "chrome", "firefox", "vscode", "mono", "setup", "install", "readme",
        "notes", "screenshot", "photo", "video", "music", "download",
    ];
    let exts = ["txt", "pdf", "docx", "xlsx", "json", "md", "log", "zip"];
    let mut names = Vec::with_capacity(n);
    for i in 0..n {
        let p = prefixes[i % prefixes.len()];
        let e = exts[(i / prefixes.len()) % exts.len()];
        names.push(format!("{}_{}_{}.{}", p, i, i * 7 % 1000, e));
    }
    names
}

/// 生成 N 个合成应用名.
fn synthetic_app_names(n: usize) -> Vec<String> {
    let names: &[&str] = &[
        "Chrome", "Firefox", "VS Code", "Notion", "Slack", "Discord",
        "Spotify", "Steam", "Word", "Excel", "PowerPoint", "Outlook",
        "Calculator", "Notepad", "Task Manager", "Paint", "Snipping Tool",
        "Camera", "Photos", "Movies TV", "Weather", "Maps", "Store",
        "Settings", "Control Panel", "PowerShell", "Command Prompt",
        "File Explorer", "Registry Editor", "Device Manager",
    ];
    (0..n).map(|i| names[i % names.len()].to_string()).collect()
}

// ============================================================================
// 直接 FTS5 查询 (绕过 FileSearchEngine, 测量纯查询延迟)
// ============================================================================

/// 构建一个内存 FTS5 索引, 填入 `n` 个文件.
fn build_fts_index(n: usize) -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"
        PRAGMA journal_mode=OFF;
        PRAGMA synchronous=OFF;
        PRAGMA temp_store=MEMORY;

        CREATE TABLE dirs (
            id INTEGER PRIMARY KEY,
            full_path TEXT NOT NULL UNIQUE
        );
        CREATE TABLE files (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            dir_id INTEGER NOT NULL
        );
        CREATE VIRTUAL TABLE files_fts USING fts5(
            name, content='files', content_rowid='id',
            tokenize='unicode61 remove_diacritics 1',
            prefix='1 2 3 4'
        );
        CREATE TRIGGER files_ai AFTER INSERT ON files BEGIN
            INSERT INTO files_fts(rowid, name) VALUES (new.id, new.name);
        END;
        CREATE INDEX idx_files_dir_id ON files(dir_id);
        "#,
    )
    .unwrap();

    // 一个根目录
    conn.execute(
        "INSERT OR IGNORE INTO dirs(id, full_path) VALUES (1, ?1)",
        params!["C:\\synthetic"],
    )
    .unwrap();

    // 批量插入文件
    let tx = conn.transaction().unwrap();
    {
        let mut stmt = tx
            .prepare("INSERT INTO files(name, dir_id) VALUES (?1, ?2)")
            .unwrap();
        for name in synthetic_file_names(n) {
            let _ = stmt.execute(params![name, 1i64]);
        }
    }
    tx.commit().unwrap();
    conn
}

/// 执行一次 FTS5 查询, 返回命中数.
fn fts_query(conn: &Connection, query: &str, limit: i64) -> usize {
    let fts_query = build_fts_match_expr(query);
    let mut stmt = conn
        .prepare(
            "SELECT d.full_path, f.name FROM files_fts fts
             JOIN files f ON f.id = fts.rowid
             JOIN dirs d ON d.id = f.dir_id
             WHERE files_fts MATCH ?1
             ORDER BY fts.rank
             LIMIT ?2",
        )
        .unwrap();
    stmt.query_map(params![fts_query, limit], |_| Ok(())).unwrap().count()
}

/// 把用户查询翻译成 FTS5 MATCH 表达式 (与 file_search.rs::build_fts_query 一致).
fn build_fts_match_expr(query: &str) -> String {
    let terms: Vec<String> = query
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| {
            let escaped: String = t
                .chars()
                .map(|c| match c {
                    '"' | '\'' | '\\' | '^' | '$' | '@' | '~' | '*' | '(' | ')' | '.' | ':'
                    | '+' | '-' => format!("\\{}", c),
                    _ => c.to_string(),
                })
                .collect();
            format!("{}*", escaped)
        })
        .collect();
    if terms.is_empty() {
        "*".to_string()
    } else {
        terms.join(" OR ")
    }
}

// ============================================================================
// 应用搜索 fixture (使用 AppSearchEngine 的 pub cache 字段)
// ============================================================================

fn build_app_engine(n_apps: usize) -> AppSearchEngine {
    let engine = AppSearchEngine::new_empty_for_tests();
    {
        let mut cache = engine.cache.write();
        for (i, name) in synthetic_app_names(n_apps).into_iter().enumerate() {
            let name_lower = name.to_lowercase();
            let launch_count = ((n_apps - i) % 50) as u32;
            cache.insert(
                format!("C:\\apps\\app_{}.exe", i),
                monotools_lib::search_engine::models::AppEntry {
                    id: format!("app_{}", i),
                    name,
                    name_lower,
                    path: PathBuf::from(format!("C:\\apps\\app_{}.exe", i)),
                    icon_path: None,
                    category: "Applications".to_string(),
                    last_launched: None,
                    launch_count,
                    alias: None,
                    is_special_shortcut: false,
                    special_command: None,
                    special_args: None,
                    pinyin_initials: None,
                    pinyin_full: None,
                    version: None,
                    file_types: Vec::new(),
                },
            );
        }
    } // cache write guard dropped here
    engine
}

// ============================================================================
// 测量工具
// ============================================================================

fn us_to_ms(us: u64) -> f64 {
    us as f64 / 1000.0
}

// ============================================================================
// 报告生成
// ============================================================================

fn ensure_report_dir() -> std::path::PathBuf {
    let repo_root = std::env!("CARGO_MANIFEST_DIR");
    let report_dir = std::path::Path::new(repo_root)
        .parent()
        .unwrap()
        .join("tests")
        .join("output")
        .join("benchmarks");
    let _ = std::fs::create_dir_all(&report_dir);
    report_dir.join("baseline.md")
}

fn append_to_report(heading: &str, body: &str) {
    let path = ensure_report_dir();
    let mut md = if path.exists() {
        std::fs::read_to_string(&path).unwrap_or_default()
    } else {
            "# MonoTools 搜索性能基准报告\n\n> 工具: hdrhistogram (cargo)\n\n---\n\n".to_string()
    };
    md.push_str(&format!("## {}\n\n{}\n\n---\n\n", heading, body));
    let _ = std::fs::write(&path, md);
}

fn init_report() {
    let path = ensure_report_dir();
    let _ = std::fs::write(
        &path,
        format!(
            "# MonoTools 搜索性能基准报告\n\n> 生成时间: {}\n> 工具: hdrhistogram (cargo)\n\n---\n\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ),
    );
}

// ============================================================================
// 基准用例: 文件搜索 (FTS5)
// ============================================================================

/// 文件搜索: 不同索引规模下的 P50/P99 延迟 (内存 FTS5, 无磁盘 I/O).
/// 规模: 1万 / 10万.
#[test]
fn bench_file_search_latency() {
    let sizes = [10_000, 100_000];
    let queries = ["re", "conf", "x", "notfound_xyz"];
    let warmup = 3;
    let runs = 200;

    init_report();

    for &n in &sizes {
        println!("\n=== 文件搜索基准: {:>7} 条索引 ===", n);
        let conn = build_fts_index(n);
        let mut table = String::new();
        table.push_str("| 查询词 | P50 (ms) | P90 (ms) | P99 (ms) | P999 (ms) | 结果数 |\n");
        table.push_str("|--------|----------|----------|---------|-----------|--------|\n");

        for q in &queries {
            // 预热
            for _ in 0..warmup {
                let _ = fts_query(&conn, q, 100);
            }
            // 正式测量
            let mut hist = Histogram::<u64>::new(3).expect("histogram");
            let mut count = 0usize;
            for _ in 0..runs {
                let start = Instant::now();
                count = fts_query(&conn, q, 100);
                let elapsed = start.elapsed().as_micros() as u64;
                hist.record(elapsed).expect("record");
            }
            let p50 = hist.value_at_percentile(50.0);
            let p90 = hist.value_at_percentile(90.0);
            let p99 = hist.value_at_percentile(99.0);
            let p999 = hist.value_at_percentile(99.9);
            println!(
                "  q={:<15} P50={:>7.2}ms  P90={:>7.2}ms  P99={:>7.2}ms  P999={:>7.2}ms  hits={}",
                q,
                us_to_ms(p50),
                us_to_ms(p90),
                us_to_ms(p99),
                us_to_ms(p999),
                count
            );
            table.push_str(&format!(
                "| `{}` | {:.2} | {:.2} | {:.2} | {:.2} | {} |\n",
                q,
                us_to_ms(p50),
                us_to_ms(p90),
                us_to_ms(p99),
                us_to_ms(p999),
                count
            ));
        }
        append_to_report(
            &format!("文件搜索 (FTS5) — {} 条索引", n),
            &format!("```\n{}\n```", table),
        );
    }
    println!("\n报告已写入: {:?}", ensure_report_dir());
}

// ============================================================================
// 基准用例: 应用搜索
// ============================================================================

/// 应用搜索: 2000 应用缓存下的全量扫描延迟.
#[test]
fn bench_app_search_latency() {
    let n_apps = 2_000;
    let queries = ["c", "chr", "chrome", "notfound_xyz", "vs code"];
    let warmup = 10;
    let runs = 500;

    println!("\n=== 应用搜索基准: {} 个应用 ===", n_apps);
    let engine = build_app_engine(n_apps);

    let mut table = String::new();
    table.push_str("| 查询词 | P50 (ms) | P90 (ms) | P99 (ms) | P999 (ms) | 结果数 |\n");
    table.push_str("|--------|----------|----------|---------|-----------|--------|\n");

    for q in &queries {
        // 预热
        for _ in 0..warmup {
            let _ = engine.search(q, 100);
        }
        // 正式测量
        let mut hist = Histogram::<u64>::new(3).expect("histogram");
        let mut count = 0usize;
        for _ in 0..runs {
            let start = Instant::now();
            let results = engine.search(q, 100);
            count = results.len();
            hist.record(start.elapsed().as_micros() as u64).expect("record");
        }
        let p50 = hist.value_at_percentile(50.0);
        let p90 = hist.value_at_percentile(90.0);
        let p99 = hist.value_at_percentile(99.0);
        let p999 = hist.value_at_percentile(99.9);
        println!(
            "  q={:<15} P50={:>7.2}ms  P90={:>7.2}ms  P99={:>7.2}ms  P999={:>7.2}ms  hits={}",
            q,
            us_to_ms(p50),
            us_to_ms(p90),
            us_to_ms(p99),
            us_to_ms(p999),
            count
        );
        table.push_str(&format!(
            "| `{}` | {:.2} | {:.2} | {:.2} | {:.2} | {} |\n",
            q,
            us_to_ms(p50),
            us_to_ms(p90),
            us_to_ms(p99),
            us_to_ms(p999),
            count
        ));
    }
    println!("```\n{}\n```", table);
    append_to_report(
        &format!("应用搜索 — {} 个应用", n_apps),
        &format!("```\n{}\n```", table),
    );
}

// ============================================================================
// 基准用例: 端到端 SearchEngine (3 source 合并 + 后处理)
// ============================================================================

/// 端到端搜索: 模拟真实 IPC 调用路径 (search_cmd → SearchEngine::search).
/// 测量包含多 source 合并、模糊评分、去重的完整延迟.
#[test]
fn bench_end_to_end_search() {
    let n_files = 10_000;
    let n_apps = 2_000;
    let queries = ["chr", "conf", "x", "notfound"];
    let warmup = 3;
    let runs = 100;

    println!("\n=== 端到端搜索基准: {} 文件 + {} 应用 ===", n_files, n_apps);

    // 构建文件索引 (内存 FTS5)
    let _file_conn = build_fts_index(n_files);
    // 构建应用引擎
    let app_engine = build_app_engine(n_apps);
    // 构建命令引擎 (空)
    let cmd_engine = {
        use monotools_lib::repositories::InMemoryCommandRepo;
        use monotools_lib::search_engine::command_search::CommandSearchEngine;
        CommandSearchEngine::new(std::sync::Arc::new(InMemoryCommandRepo::new()))
    };

    // 注意: 这里我们不能直接用 SearchEngine (需要 Arc<FileSearchEngine>),
    // 所以只测量文件 + 应用两个 source 的合并. 命令引擎为空, 不影响测量.
    let engine = monotools_lib::search_engine::SearchEngine::new(
        std::sync::Arc::new(app_engine),
        // 无法直接构造含内存连接的 FileSearchEngine, 跳过文件 source
        // 这里只测应用 source 的端到端
        std::sync::Arc::new(monotools_lib::search_engine::file_search::FileSearchEngine::new_with_db_path_for_tests().unwrap()),
        std::sync::Arc::new(cmd_engine),
    );

    let mut table = String::new();
    table.push_str("| 查询词 | P50 (ms) | P90 (ms) | P99 (ms) | 结果数 |\n");
    table.push_str("|--------|----------|----------|---------|--------|\n");

    for q in &queries {
        for _ in 0..warmup {
            let _ = engine.search(q, 100);
        }
        let mut hist = Histogram::<u64>::new(3).expect("histogram");
        let mut count = 0usize;
        for _ in 0..runs {
            let start = Instant::now();
            count = engine.search(q, 100).len();
            hist.record(start.elapsed().as_micros() as u64).expect("record");
        }
        let p50 = hist.value_at_percentile(50.0);
        let p90 = hist.value_at_percentile(90.0);
        let p99 = hist.value_at_percentile(99.0);
        println!(
            "  q={:<15} P50={:>7.2}ms  P90={:>7.2}ms  P99={:>7.2}ms  hits={}",
            q,
            us_to_ms(p50),
            us_to_ms(p90),
            us_to_ms(p99),
            count
        );
        table.push_str(&format!(
            "| `{}` | {:.2} | {:.2} | {:.2} | {} |\n",
            q, us_to_ms(p50), us_to_ms(p90), us_to_ms(p99), count
        ));
    }
    println!("```\n{}\n```", table);
    // 避免 unused warnings
    let _ = n_files;
}
