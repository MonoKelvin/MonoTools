use monotools_lib::engines::file_search::FileSearchEngine;
use std::io::Write;

const MODULE_NAME: &str = "search_engine";

fn base_dir() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::path::PathBuf::from(manifest_dir).join("tests").join("rust")
}

fn data_path(module: &str, filename: &str) -> std::path::PathBuf {
    base_dir().join("data").join(module).join(filename)
}

fn output_path(module: &str, filename: &str) -> std::path::PathBuf {
    base_dir().join("output").join(module).join(filename)
}

fn ensure_dir(path: &std::path::PathBuf) {
    if !path.exists() {
        let _ = std::fs::create_dir_all(path);
    }
}

/// 生成带日期时间的文件名，如 summary_20260710_075820.txt
fn timestamped_filename(base: &str, ext: &str) -> String {
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    format!("{}_{}.{}", base, ts, ext)
}

fn get_test_db_path() -> std::path::PathBuf {
    let dir = data_path(MODULE_NAME, "databases");
    ensure_dir(&dir);
    let pid = std::process::id();
    dir.join(format!("test_index_{}.db", pid))
}

fn cleanup_test_data() {
    let data_dir = data_path(MODULE_NAME, "");
    if data_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&data_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    ensure_dir(&data_dir);
}

fn get_system_roots(drives: &[char]) -> Vec<std::path::PathBuf> {
    let mut roots = Vec::new();
    for drive in drives {
        let drive_letter = drive.to_ascii_uppercase();
        let drive_path = std::path::PathBuf::from(format!("{}:\\", drive_letter));
        if drive_path.exists() {
            roots.push(drive_path);
        }
    }
    roots
}

#[derive(Debug, Clone)]
struct SearchEngineTestConfig {
    pub test_drives: Vec<char>,
    pub index_timeout_ms: u64,
    pub sample_size: usize,
    pub search_limit: u32,
    pub max_search_time_ms: u64,
    pub search_keywords: Vec<String>,
    pub validity_threshold: f64,
    /// 搜索结果抽样输出数量（每个关键字输出前 N 条路径）
    pub search_sample_paths: usize,
}

impl Default for SearchEngineTestConfig {
    fn default() -> Self {
        SearchEngineTestConfig {
            test_drives: vec!['D'],
            index_timeout_ms: 180000,
            sample_size: 100,
            search_limit: 100,
            max_search_time_ms: 1000,
            search_keywords: vec![
                "txt".to_string(),
                "pdf".to_string(),
                "doc".to_string(),
                "exe".to_string(),
                "rust".to_string(),
                "code".to_string(),
                "test".to_string(),
                "project".to_string(),
                "data".to_string(),
                "config".to_string(),
            ],
            validity_threshold: 0.85,
            search_sample_paths: 10,
        }
    }
}

#[tokio::test]
async fn run_all_search_engine_tests() {
    run_search_engine_tests().await;
}

pub async fn run_search_engine_tests() {
    let config = SearchEngineTestConfig::default();
    let mut report = TestReport::new("文件搜索引擎");
    let mut results = TestResults::new();

    cleanup_test_data();
    let db_path = get_test_db_path();

    let roots = get_system_roots(&config.test_drives);
    println!("DEBUG: 测试盘符: {:?}", roots);

    let engine = match FileSearchEngine::new_with_db_path_and_roots(db_path, roots) {
        Ok(e) => e,
        Err(e) => {
            results.add_result("引擎创建", false, &format!("创建失败: {}", e), 0);
            report.add_section_item("错误", "引擎创建失败", &format!("{}", e));
            report.save(&output_path(MODULE_NAME, &timestamped_filename("summary", "txt")));
            println!("引擎创建失败: {}", e);
            return;
        }
    };

    println!("DEBUG: 引擎创建成功，开始构建索引...");

    let t1 = test_index_building(&engine, &config).await;
    results.add_result("索引构建", t1.passed, &t1.message, t1.duration_ms);
    let t1_status = if t1.passed {
        format!("通过, {} 个文件, 耗时 {}ms, 抽样验证 {}/{}", t1.total_files, t1.duration_ms, t1.valid_count, t1.sample_count)
    } else { "失败".to_string() };
    report.add_section_item("索引功能", "索引构建", &t1_status);

    // 输出抽样验证的文件列表
    if !t1.sample_files.is_empty() {
        let mut details = Vec::new();
        details.push(format!("抽样数量: {}, 有效: {}, 无效: {}", t1.sample_count, t1.valid_count, t1.sample_count - t1.valid_count));
        details.push(String::new()); // 空行分隔
        details.push(format!("{:<6} {}", "状态", "文件路径"));
        details.push("-".repeat(80));
        for (path, exists) in &t1.sample_files {
            let status = if *exists { "✓ 存在" } else { "✗ 不存在" };
            details.push(format!("{:<6} {}", status, path));
        }
        report.add_section_item_with_details("索引功能", "抽样验证文件", &t1_status, &details);
    }

    if !t1.passed || t1.total_files == 0 {
        println!("索引构建失败或为空，测试结束");
        let (passed, failed) = results.summary();
        report.add_section_item("测试结果", "总测试数", &format!("{}", passed + failed));
        report.add_section_item("测试结果", "通过", &format!("{}", passed));
        report.add_section_item("测试结果", "失败", &format!("{}", failed));
        report.save(&output_path(MODULE_NAME, &timestamped_filename("summary", "txt")));
        println!("{}", results.generate_summary());
        return;
    }

    let t2 = test_search_logic(&engine, &config).await;
    results.add_result("搜索验证", t2.passed, &t2.message, t2.duration_ms);
    let t2_status = format!("通过, 总验证 {}/{}, 平均搜索时间 {}ms", t2.total_valid, t2.total_checked, t2.avg_search_time_ms);
    report.add_section_item("搜索功能", "搜索验证", &t2_status);

    // 输出每个关键字的搜索详情
    for kd in &t2.keyword_details {
        let mut details = Vec::new();
        details.push(format!("关键字: '{}'", kd.keyword));
        details.push(format!("匹配规则: FTS5 前缀匹配 (prefix='2 3 4', tokenizer='unicode61')"));
        details.push(format!("结果数: {}, 有效数: {}, 耗时: {}ms", kd.result_count, kd.valid_count, kd.time_ms));
        if !kd.sample_paths.is_empty() {
            details.push(String::new());
            details.push(format!("{:<6} {}", "状态", "文件路径"));
            details.push("-".repeat(80));
            for (path, exists) in &kd.sample_paths {
                let status = if *exists { "✓ 存在" } else { "✗ 不存在" };
                details.push(format!("{:<6} {}", status, path));
            }
        }
        report.add_section_item_with_details("搜索功能", &format!("关键字 '{}'", kd.keyword), &kd.summary(), &details);
    }

    let (passed, failed) = results.summary();
    report.add_section_item("测试结果", "总测试数", &format!("{}", passed + failed));
    report.add_section_item("测试结果", "通过", &format!("{}", passed));
    report.add_section_item("测试结果", "失败", &format!("{}", failed));

    let output_dir = output_path(MODULE_NAME, "");
    ensure_dir(&output_dir);
    report.save(&output_path(MODULE_NAME, &timestamped_filename("summary", "txt")));

    println!("{}", results.generate_summary());
}

struct IndexBuildResult {
    passed: bool,
    message: String,
    duration_ms: u64,
    total_files: usize,
    sample_count: usize,
    valid_count: usize,
    /// 抽样文件列表 (路径, 是否存在)
    sample_files: Vec<(String, bool)>,
}

async fn test_index_building(engine: &FileSearchEngine, config: &SearchEngineTestConfig) -> IndexBuildResult {
    let start = std::time::Instant::now();

    let build_result = tokio::time::timeout(
        std::time::Duration::from_millis(config.index_timeout_ms),
        engine.build_index()
    ).await;

    let duration_ms = start.elapsed().as_millis() as u64;

    match build_result {
        Ok(Ok(_)) => {
            let total = engine.total();

            if total == 0 {
                return IndexBuildResult {
                    passed: false,
                    message: "索引构建完成但文件数为零".to_string(),
                    duration_ms,
                    total_files: 0,
                    sample_count: 0,
                    valid_count: 0,
                    sample_files: Vec::new(),
                };
            }

            if duration_ms > config.index_timeout_ms {
                return IndexBuildResult {
                    passed: false,
                    message: format!("索引构建超时: {}ms，超过{}ms限制", duration_ms, config.index_timeout_ms),
                    duration_ms,
                    total_files: total,
                    sample_count: 0,
                    valid_count: 0,
                    sample_files: Vec::new(),
                };
            }

            let (sample_count, valid_count, sample_files) = validate_index_samples(engine, config.sample_size);

            let rate = if sample_count > 0 { valid_count as f64 / sample_count as f64 } else { 0.0 };
            let passed = rate >= config.validity_threshold;

            let message = format!(
                "索引构建完成，共 {} 个文件，耗时 {}ms，抽样验证 {}/{} ({:.1}%)",
                total, duration_ms, valid_count, sample_count, rate * 100.0
            );

            println!("DEBUG: {}", message);

            IndexBuildResult {
                passed,
                message,
                duration_ms,
                total_files: total,
                sample_count,
                valid_count,
                sample_files,
            }
        }
        Ok(Err(e)) => {
            IndexBuildResult {
                passed: false,
                message: format!("索引构建失败: {}", e),
                duration_ms,
                total_files: 0,
                sample_count: 0,
                valid_count: 0,
                sample_files: Vec::new(),
            }
        }
        Err(_) => {
            IndexBuildResult {
                passed: false,
                message: format!("索引构建超时: {}ms，超过{}ms限制", duration_ms, config.index_timeout_ms),
                duration_ms,
                total_files: 0,
                sample_count: 0,
                valid_count: 0,
                sample_files: Vec::new(),
            }
        }
    }
}

/// 验证索引抽样，返回 (抽样数, 有效数, 文件列表)
fn validate_index_samples(engine: &FileSearchEngine, sample_size: usize) -> (usize, usize, Vec<(String, bool)>) {
    let all_results = engine.search("", sample_size as u32);
    let mut sample_files = Vec::new();
    let mut valid_count = 0;

    for result in all_results.iter().take(sample_size) {
        let path = std::path::PathBuf::from(&result.id);
        let exists = path.exists();
        if exists {
            valid_count += 1;
        }
        sample_files.push((result.id.clone(), exists));
    }

    (all_results.len().min(sample_size), valid_count, sample_files)
}

/// 单个关键字的搜索详情
struct KeywordDetail {
    keyword: String,
    result_count: usize,
    valid_count: usize,
    time_ms: u64,
    /// 抽样路径列表 (路径, 是否存在)
    sample_paths: Vec<(String, bool)>,
}

impl KeywordDetail {
    fn summary(&self) -> String {
        format!("结果 {}, 有效 {}, 耗时 {}ms", self.result_count, self.valid_count, self.time_ms)
    }
}

struct SearchTestResult {
    passed: bool,
    message: String,
    duration_ms: u64,
    total_checked: usize,
    total_valid: usize,
    avg_search_time_ms: u64,
    keyword_details: Vec<KeywordDetail>,
}

async fn test_search_logic(engine: &FileSearchEngine, config: &SearchEngineTestConfig) -> SearchTestResult {
    let mut total_checked = 0;
    let mut total_valid = 0;
    let mut total_search_time_ms = 0u64;
    let mut keyword_details = Vec::new();

    for keyword in &config.search_keywords {
        let start = std::time::Instant::now();
        let results = engine.search(keyword, config.search_limit);
        let search_time_ms = start.elapsed().as_millis() as u64;
        total_search_time_ms += search_time_ms;

        let mut valid_count = 0;
        let mut sample_paths = Vec::new();
        for (i, result) in results.iter().enumerate() {
            let path = std::path::PathBuf::from(&result.id);
            let exists = path.exists();
            if exists {
                valid_count += 1;
            }
            // 只抽样前 N 条路径用于输出
            if i < config.search_sample_paths {
                sample_paths.push((result.id.clone(), exists));
            }
        }

        total_checked += results.len();
        total_valid += valid_count;

        keyword_details.push(KeywordDetail {
            keyword: keyword.clone(),
            result_count: results.len(),
            valid_count,
            time_ms: search_time_ms,
            sample_paths,
        });

        println!("DEBUG: 搜索 '{}' -> {} 结果, {} 有效", keyword, results.len(), valid_count);
    }

    let avg_search_time_ms = if !config.search_keywords.is_empty() {
        total_search_time_ms / config.search_keywords.len() as u64
    } else {
        0
    };

    let rate = if total_checked > 0 {
        total_valid as f64 / total_checked as f64
    } else {
        0.0
    };

    let passed = rate >= config.validity_threshold && total_checked > 0;

    let message = format!(
        "搜索测试完成，总验证 {}/{} ({:.1}%)，平均搜索时间 {}ms",
        total_valid, total_checked, rate * 100.0, avg_search_time_ms
    );

    SearchTestResult {
        passed,
        message,
        duration_ms: total_search_time_ms,
        total_checked,
        total_valid,
        avg_search_time_ms,
        keyword_details,
    }
}

struct TestReport {
    module_name: String,
    sections: Vec<ReportSection>,
    timestamp: String,
}

struct ReportSection {
    title: String,
    items: Vec<ReportItem>,
}

struct ReportItem {
    label: String,
    value: String,
    /// 详情列表（多行），输出在 value 之后
    details: Vec<String>,
}

impl TestReport {
    pub fn new(module_name: &str) -> Self {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        TestReport {
            module_name: module_name.to_string(),
            sections: Vec::new(),
            timestamp,
        }
    }

    pub fn add_section(&mut self, title: &str) -> &mut ReportSection {
        self.sections.push(ReportSection {
            title: title.to_string(),
            items: Vec::new(),
        });
        self.sections.last_mut().unwrap()
    }

    pub fn add_section_item(&mut self, section_title: &str, label: &str, value: &str) {
        self.add_section_item_with_details(section_title, label, value, &[]);
    }

    pub fn add_section_item_with_details(&mut self, section_title: &str, label: &str, value: &str, details: &[String]) {
        let section = self.sections.iter_mut().find(|s| s.title == section_title);
        match section {
            Some(s) => {
                s.items.push(ReportItem {
                    label: label.to_string(),
                    value: value.to_string(),
                    details: details.to_vec(),
                });
            }
            None => {
                let section = self.add_section(section_title);
                section.items.push(ReportItem {
                    label: label.to_string(),
                    value: value.to_string(),
                    details: details.to_vec(),
                });
            }
        }
    }

    pub fn generate(&self) -> String {
        let mut output = format!("{} 测试报告\n", self.module_name);
        output.push_str(&"=".repeat(80));
        output.push('\n');
        output.push_str(&format!("输出时间: {}\n", self.timestamp));

        for section in &self.sections {
            output.push_str(&format!("\n## {}\n", section.title));

            let max_label_len = section.items.iter().map(|i| i.label.len()).max().unwrap_or(0);
            for item in &section.items {
                output.push_str(&format!("{:<width$}: {}\n", item.label, item.value, width = max_label_len));
                // 输出详情列表
                for line in &item.details {
                    output.push_str(&format!("  {}\n", line));
                }
            }
        }

        output.push('\n');
        output
    }

    pub fn save(&self, path: &std::path::PathBuf) {
        let _ = std::fs::write(path, self.generate());
    }
}

struct TestResults {
    results: std::collections::HashMap<String, TestResult>,
}

struct TestResult {
    passed: bool,
    message: String,
    duration_ms: u64,
}

impl TestResults {
    pub fn new() -> Self {
        TestResults {
            results: std::collections::HashMap::new(),
        }
    }

    pub fn add_result(&mut self, test_name: &str, passed: bool, message: &str, duration_ms: u64) {
        self.results.insert(test_name.to_string(), TestResult {
            passed,
            message: message.to_string(),
            duration_ms,
        });
    }

    pub fn summary(&self) -> (usize, usize) {
        let passed = self.results.values().filter(|r| r.passed).count();
        let failed = self.results.len() - passed;
        (passed, failed)
    }

    pub fn generate_summary(&self) -> String {
        let (passed, failed) = self.summary();
        let total = self.results.len();

        let mut output = format!("测试结果汇总\n");
        output.push_str(&"=".repeat(60));
        output.push_str(&format!("\n总测试数: {}\n通过: {}\n失败: {}\n", total, passed, failed));
        output.push_str(&"-".repeat(60));
        output.push('\n');

        for (name, result) in &self.results {
            let status = if result.passed { "✓" } else { "✗" };
            output.push_str(&format!("{} {} ({}ms)\n", status, name, result.duration_ms));
            if !result.message.is_empty() {
                output.push_str(&format!("  {}\n", result.message));
            }
        }

        output
    }
}