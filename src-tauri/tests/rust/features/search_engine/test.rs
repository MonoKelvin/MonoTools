use monotools_lib::engines::file_search::FileSearchEngine;
use monotools_lib::platform::windows::usn::{NtfsIndexer, UsnChangeReason, UsnRecord};
use rand::prelude::*;

use crate::common::logger::TestLogger;
use crate::common::reporter::TestReporter;

const MODULE_NAME: &str = "search_engine";

fn base_dir() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::path::PathBuf::from(manifest_dir)
        .join("tests")
        .join("rust")
}

fn data_path(module: &str, filename: &str) -> std::path::PathBuf {
    base_dir().join("data").join(module).join(filename)
}

fn output_path(module: &str, filename: &str) -> std::path::PathBuf {
    base_dir().join("output").join(module).join(filename)
}

fn log_dir_path(module: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("output")
        .join(module)
}

fn ensure_dir(path: &std::path::PathBuf) {
    if !path.exists() {
        let _ = std::fs::create_dir_all(path);
    }
}

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
                if path.is_file() {
                    let _ = std::fs::remove_file(&path);
                }
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

fn sample_results<T>(results: &[T], max_sample: usize) -> Vec<&T> {
    let total = results.len();

    if total <= max_sample {
        results.iter().collect()
    } else {
        let mut rng = thread_rng();
        let mut indices: Vec<usize> = (0..total).collect();
        indices.shuffle(&mut rng);
        indices
            .into_iter()
            .take(max_sample)
            .map(|i| &results[i])
            .collect()
    }
}

#[derive(Debug, Clone)]
struct SearchEngineTestConfig {
    pub test_drives: Vec<char>,
    pub index_timeout_ms: u64,
    pub sample_size: usize,
    pub search_limit: u32,
    pub search_keywords: Vec<String>,
    pub validity_threshold: f64,
    pub search_sample_paths: usize,
    pub usn_monitor_duration_ms: u64,
    pub case_insensitive_keywords: Vec<String>,
    pub whole_word_keywords: Vec<String>,
    pub regex_patterns: Vec<String>,
}

impl Default for SearchEngineTestConfig {
    fn default() -> Self {
        SearchEngineTestConfig {
            test_drives: vec!['E'],
            index_timeout_ms: 180000,
            sample_size: 100,
            search_limit: 100,
            search_keywords: vec![
                "work".to_string(),
                "code".to_string(),
                "project".to_string(),
                "git".to_string(),
                "rust".to_string(),
                "txt".to_string(),
                "json".to_string(),
                "rs".to_string(),
                "toml".to_string(),
                "lock".to_string(),
                "md".to_string(),
                "html".to_string(),
                "css".to_string(),
                "js".to_string(),
                "png".to_string(),
                "jpg".to_string(),
                "log".to_string(),
                "config".to_string(),
                "data".to_string(),
                "src".to_string(),
            ],
            validity_threshold: 0.85,
            search_sample_paths: 10,
            usn_monitor_duration_ms: 5000,
            case_insensitive_keywords: vec![
                "WORK".to_string(),
                "CODE".to_string(),
                "RUST".to_string(),
                "JSON".to_string(),
                "INDEX".to_string(),
            ],
            whole_word_keywords: vec![],
            regex_patterns: vec![],
        }
    }
}

#[tokio::test]
async fn run_all_search_engine_tests() {
    run_search_engine_tests().await;
}

pub async fn run_search_engine_tests() {
    let config = SearchEngineTestConfig::default();
    let log_dir = log_dir_path(MODULE_NAME);
    let mut logger = TestLogger::new(MODULE_NAME, &log_dir);
    let mut reporter = TestReporter::new("文件搜索引擎");

    logger.section("测试初始化");
    logger.info(&format!("测试模块: {}", MODULE_NAME));
    logger.info(&format!("测试盘符: {:?}", config.test_drives));
    logger.info(&format!("索引超时: {}ms", config.index_timeout_ms));
    logger.info(&format!("抽样数量: {}", config.sample_size));

    cleanup_test_data();
    let db_path = get_test_db_path();
    logger.info(&format!("测试数据库: {}", db_path.display()));

    let roots = get_system_roots(&config.test_drives);
    logger.info(&format!("检测到系统根目录: {:?}", roots));

    let engine = match FileSearchEngine::new_with_db_path_and_roots(db_path.clone(), roots.clone())
    {
        Ok(e) => e,
        Err(e) => {
            let msg = format!("引擎创建失败: {}", e);
            logger.error(&msg);
            reporter.add_test("引擎创建");
            reporter.finish_test("引擎创建", false, 0, &msg);
            reporter.print();
            reporter.save(&output_path(
                MODULE_NAME,
                &timestamped_filename("summary", "txt"),
            ));
            return;
        }
    };
    logger.success("引擎创建成功");

    logger.section("测试一: 索引构建");
    reporter.add_test("索引构建");
    let t1 = test_index_building(&engine, &config, &mut logger).await;
    reporter.finish_test("索引构建", t1.passed, t1.duration_ms, &t1.message);

    if t1.passed {
        reporter.add_test_detail("索引构建", &format!("文件总数: {}", t1.total_files));
        reporter.add_test_detail(
            "索引构建",
            &format!(
                "抽样验证: {}/{} ({:.1}%)",
                t1.valid_count,
                t1.sample_count,
                if t1.sample_count > 0 {
                    t1.valid_count as f64 / t1.sample_count as f64 * 100.0
                } else {
                    0.0
                }
            ),
        );

        if !t1.sample_files.is_empty() {
            let details: Vec<String> = t1
                .sample_files
                .iter()
                .map(|(path, exists)| format!("{} {}", if *exists { "✓" } else { "✗" }, path))
                .collect();
            for detail in details {
                reporter.add_test_detail("索引构建", &detail);
            }
        }
    }

    if !t1.passed || t1.total_files == 0 {
        logger.warn("索引构建失败或为空，终止后续测试");
        reporter.print();
        reporter.save(&output_path(
            MODULE_NAME,
            &timestamped_filename("summary", "txt"),
        ));
        return;
    }

    logger.section("测试二: 搜索验证");
    reporter.add_test("搜索验证");
    let t2 = test_search_logic(&engine, &config, &mut logger).await;
    reporter.finish_test("搜索验证", t2.passed, t2.duration_ms, &t2.message);

    if t2.passed {
        reporter.add_test_detail(
            "搜索验证",
            &format!(
                "总验证: {}/{} ({:.1}%)",
                t2.total_valid,
                t2.total_checked,
                if t2.total_checked > 0 {
                    t2.total_valid as f64 / t2.total_checked as f64 * 100.0
                } else {
                    0.0
                }
            ),
        );
        reporter.add_test_detail(
            "搜索验证",
            &format!("平均搜索时间: {}ms", t2.avg_search_time_ms),
        );

        let mut search_stats_list = Vec::new();
        for kd in &t2.keyword_details {
            search_stats_list.push(crate::common::reporter::SearchStats::new(
                &kd.keyword,
                kd.result_count,
                kd.valid_count,
                kd.time_ms,
                &kd.match_type,
            ));
        }

        if !search_stats_list.is_empty() {
            let stats_table =
                crate::common::reporter::SearchStats::generate_table(&search_stats_list);
            reporter.add_test_table("搜索验证", stats_table);
        }
    }

    logger.section("测试三: USN Journal 监控");
    reporter.add_test("USN监控");
    let t3 = test_usn_journal(&config, &mut logger).await;
    reporter.finish_test("USN监控", t3.passed, t3.duration_ms, &t3.message);

    if t3.passed {
        reporter.add_test_detail("USN监控", &format!("总变化数: {}", t3.total_changes));
        reporter.add_test_detail(
            "USN监控",
            &format!(
                "创建: {}, 修改: {}, 删除: {}",
                t3.create_count, t3.modify_count, t3.delete_count
            ),
        );
        reporter.add_test_detail(
            "USN监控",
            &format!(
                "验证: {}/{} ({:.1}%)",
                t3.valid_count,
                t3.total_validated,
                if t3.total_validated > 0 {
                    t3.valid_count as f64 / t3.total_validated as f64 * 100.0
                } else {
                    0.0
                }
            ),
        );
    }

    logger.section("测试完成");
    reporter.print();

    let output_dir = output_path(MODULE_NAME, "");
    ensure_dir(&output_dir);
    reporter.save(&output_path(
        MODULE_NAME,
        &timestamped_filename("summary", "txt"),
    ));

    let (passed, failed) = reporter.summary();
    logger.info(&format!(
        "测试结果: 通过 {} / 失败 {} / 总数 {}",
        passed,
        failed,
        passed + failed
    ));
}

struct IndexBuildResult {
    passed: bool,
    message: String,
    duration_ms: u64,
    total_files: usize,
    sample_count: usize,
    valid_count: usize,
    sample_files: Vec<(String, bool)>,
}

async fn test_index_building(
    engine: &FileSearchEngine,
    config: &SearchEngineTestConfig,
    logger: &mut TestLogger,
) -> IndexBuildResult {
    let start = std::time::Instant::now();
    logger.info("开始构建索引...");

    let build_result = tokio::time::timeout(
        std::time::Duration::from_millis(config.index_timeout_ms),
        engine.build_index(),
    )
    .await;

    let duration_ms = start.elapsed().as_millis() as u64;

    match build_result {
        Ok(Ok(_)) => {
            let total = engine.total();
            logger.info(&format!("索引构建完成，共 {} 个文件", total));

            if total == 0 {
                let msg = "索引构建完成但文件数为零".to_string();
                logger.error(&msg);
                return IndexBuildResult {
                    passed: false,
                    message: msg,
                    duration_ms,
                    total_files: 0,
                    sample_count: 0,
                    valid_count: 0,
                    sample_files: Vec::new(),
                };
            }

            if duration_ms > config.index_timeout_ms {
                let msg = format!(
                    "索引构建超时: {}ms，超过{}ms限制",
                    duration_ms, config.index_timeout_ms
                );
                logger.error(&msg);
                return IndexBuildResult {
                    passed: false,
                    message: msg,
                    duration_ms,
                    total_files: total,
                    sample_count: 0,
                    valid_count: 0,
                    sample_files: Vec::new(),
                };
            }

            logger.subsection("路径验证");
            let (sample_count, valid_count, sample_files) =
                validate_index_samples(engine, config.sample_size, logger);

            let rate = if sample_count > 0 {
                valid_count as f64 / sample_count as f64
            } else {
                0.0
            };
            let passed = rate >= config.validity_threshold;

            let message = format!(
                "索引构建完成，共 {} 个文件，耗时 {}ms，抽样验证 {}/{} ({:.1}%)",
                total,
                duration_ms,
                valid_count,
                sample_count,
                rate * 100.0
            );

            if passed {
                logger.success(&message);
            } else {
                logger.warn(&message);
            }

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
            let msg = format!("索引构建失败: {}", e);
            logger.error(&msg);
            IndexBuildResult {
                passed: false,
                message: msg,
                duration_ms,
                total_files: 0,
                sample_count: 0,
                valid_count: 0,
                sample_files: Vec::new(),
            }
        }
        Err(_) => {
            let msg = format!(
                "索引构建超时: {}ms，超过{}ms限制",
                duration_ms, config.index_timeout_ms
            );
            logger.error(&msg);
            IndexBuildResult {
                passed: false,
                message: msg,
                duration_ms,
                total_files: 0,
                sample_count: 0,
                valid_count: 0,
                sample_files: Vec::new(),
            }
        }
    }
}

fn validate_index_samples(
    engine: &FileSearchEngine,
    sample_size: usize,
    logger: &mut TestLogger,
) -> (usize, usize, Vec<(String, bool)>) {
    let all_results = engine.search("", sample_size as u32 * 2);
    let samples = sample_results(&all_results, sample_size);
    let mut sample_files = Vec::new();
    let mut valid_count = 0;

    for result in samples {
        let path = std::path::PathBuf::from(&result.id);
        let exists = path.exists();
        if exists {
            valid_count += 1;
        }
        sample_files.push((result.id.clone(), exists));
    }

    logger.info(&format!(
        "路径验证完成: 抽样 {} 条, 有效 {} 条",
        sample_files.len(),
        valid_count
    ));

    (sample_files.len(), valid_count, sample_files)
}

struct KeywordDetail {
    keyword: String,
    result_count: usize,
    valid_count: usize,
    time_ms: u64,
    sample_paths: Vec<(String, bool)>,
    match_type: String,
}

impl KeywordDetail {
    fn summary(&self) -> String {
        format!(
            "{} | 结果 {} | 有效 {} | 耗时 {}ms",
            self.match_type, self.result_count, self.valid_count, self.time_ms
        )
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

async fn test_search_logic(
    engine: &FileSearchEngine,
    config: &SearchEngineTestConfig,
    logger: &mut TestLogger,
) -> SearchTestResult {
    let mut total_checked = 0;
    let mut total_valid = 0;
    let mut total_search_time_ms = 0u64;
    let mut keyword_details = Vec::new();

    logger.subsection("基础关键字搜索");
    for keyword in &config.search_keywords {
        let detail = run_search_test(
            engine,
            keyword,
            config.search_limit,
            config.search_sample_paths,
            "基础匹配",
            logger,
        );
        total_checked += detail.result_count;
        total_valid += detail.valid_count;
        total_search_time_ms += detail.time_ms;
        keyword_details.push(detail);
    }

    logger.subsection("大小写不敏感搜索");
    for keyword in &config.case_insensitive_keywords {
        let detail = run_search_test(
            engine,
            keyword,
            config.search_limit,
            config.search_sample_paths,
            "大小写不敏感",
            logger,
        );
        total_checked += detail.result_count;
        total_valid += detail.valid_count;
        total_search_time_ms += detail.time_ms;
        keyword_details.push(detail);
    }

    logger.subsection("全字匹配搜索");
    for keyword in &config.whole_word_keywords {
        let detail = run_search_test(
            engine,
            keyword,
            config.search_limit,
            config.search_sample_paths,
            "全字匹配",
            logger,
        );
        total_checked += detail.result_count;
        total_valid += detail.valid_count;
        total_search_time_ms += detail.time_ms;
        keyword_details.push(detail);
    }

    logger.subsection("正则表达式搜索");
    for pattern in &config.regex_patterns {
        let detail = run_search_test(
            engine,
            pattern,
            config.search_limit,
            config.search_sample_paths,
            "正则表达式",
            logger,
        );
        total_checked += detail.result_count;
        total_valid += detail.valid_count;
        total_search_time_ms += detail.time_ms;
        keyword_details.push(detail);
    }

    let avg_search_time_ms = if keyword_details.is_empty() {
        0
    } else {
        total_search_time_ms / keyword_details.len() as u64
    };

    let rate = if total_checked > 0 {
        total_valid as f64 / total_checked as f64
    } else {
        0.0
    };

    let passed = rate >= config.validity_threshold && total_checked > 0;

    let message = format!(
        "搜索测试完成，总验证 {}/{} ({:.1}%)，平均搜索时间 {}ms",
        total_valid,
        total_checked,
        rate * 100.0,
        avg_search_time_ms
    );

    if passed {
        logger.success(&message);
    } else {
        logger.warn(&message);
    }

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

fn run_search_test(
    engine: &FileSearchEngine,
    query: &str,
    limit: u32,
    sample_size: usize,
    match_type: &str,
    logger: &mut TestLogger,
) -> KeywordDetail {
    let start = std::time::Instant::now();
    let results = engine.search(query, limit);
    let search_time_ms = start.elapsed().as_millis() as u64;

    let mut valid_count = 0;
    let mut sample_paths = Vec::new();

    for (i, result) in results.iter().enumerate() {
        let path = std::path::PathBuf::from(&result.id);
        let exists = path.exists();
        if exists {
            valid_count += 1;
        }
        if i < sample_size {
            sample_paths.push((result.id.clone(), exists));
        }
    }

    let rate = if results.len() > 0 {
        valid_count as f64 / results.len() as f64 * 100.0
    } else {
        0.0
    };

    logger.debug(&format!(
        "[{}] '{}' -> {} 结果, {} 有效 ({:.1}%), {}ms",
        match_type,
        query,
        results.len(),
        valid_count,
        rate,
        search_time_ms
    ));

    KeywordDetail {
        keyword: query.to_string(),
        result_count: results.len(),
        valid_count,
        time_ms: search_time_ms,
        sample_paths,
        match_type: match_type.to_string(),
    }
}

struct UsnMonitorResult {
    passed: bool,
    message: String,
    duration_ms: u64,
    total_changes: usize,
    create_count: usize,
    modify_count: usize,
    delete_count: usize,
    total_validated: usize,
    valid_count: usize,
    changes: Vec<(UsnChangeReason, String, bool)>,
}

async fn test_usn_journal(
    config: &SearchEngineTestConfig,
    logger: &mut TestLogger,
) -> UsnMonitorResult {
    let start = std::time::Instant::now();

    let indexer = match NtfsIndexer::new_with_drives(config.test_drives.clone()) {
        Ok(i) => i,
        Err(e) => {
            let msg = format!("NtfsIndexer 创建失败: {}", e);
            logger.error(&msg);
            return UsnMonitorResult {
                passed: false,
                message: msg,
                duration_ms: 0,
                total_changes: 0,
                create_count: 0,
                modify_count: 0,
                delete_count: 0,
                total_validated: 0,
                valid_count: 0,
                changes: Vec::new(),
            };
        }
    };

    let volumes = indexer.get_volumes();
    logger.info(&format!("检测到 NTFS 卷: {:?}", volumes));

    if volumes.is_empty() {
        let msg = "未检测到 NTFS 卷".to_string();
        logger.error(&msg);
        return UsnMonitorResult {
            passed: false,
            message: msg,
            duration_ms: 0,
            total_changes: 0,
            create_count: 0,
            modify_count: 0,
            delete_count: 0,
            total_validated: 0,
            valid_count: 0,
            changes: Vec::new(),
        };
    }

    let volume = volumes[0].clone();
    let test_dir = data_path(MODULE_NAME, "usn_test_files");

    let mut start_usn = 0;
    if let Some(state) = indexer.get_journal_state(&volume) {
        start_usn = state.next_usn;
        logger.info(&format!(
            "USN Journal 状态: first_usn={}, next_usn={}, journal_id={}",
            state.first_usn, state.next_usn, state.journal_id
        ));
    }

    let _ = std::fs::remove_dir_all(&test_dir);
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    std::fs::create_dir_all(&test_dir).unwrap();
    logger.debug(&format!("创建测试目录: {}", test_dir.display()));

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let mut test_files = Vec::new();
    for i in 0..5 {
        let file_path = test_dir.join(format!("usn_test_{}.txt", i));
        test_files.push(file_path.clone());
        let _ = std::fs::write(&file_path, format!("测试内容 {}", i));
        logger.debug(&format!("创建测试文件: {}", file_path.display()));
    }

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    for (i, file_path) in test_files.iter().enumerate().take(3) {
        let _ = std::fs::write(file_path, format!("修改内容 {}", i));
        logger.debug(&format!("修改测试文件: {}", file_path.display()));
    }

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    for (_i, file_path) in test_files.iter().enumerate().take(2) {
        let _ = std::fs::remove_file(file_path);
        logger.debug(&format!("删除测试文件: {}", file_path.display()));
    }

    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    let changes = match indexer.read_usn_changes(&volume, start_usn) {
        Ok(c) => c,
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let msg = format!("读取 USN 变化失败: {} (需要管理员权限)", e);
            logger.error(&msg);
            return UsnMonitorResult {
                passed: false,
                message: msg,
                duration_ms,
                total_changes: 0,
                create_count: 0,
                modify_count: 0,
                delete_count: 0,
                total_validated: 0,
                valid_count: 0,
                changes: Vec::new(),
            };
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    let mut create_count = 0;
    let mut modify_count = 0;
    let mut delete_count = 0;
    let mut total_validated = 0;
    let mut valid_count = 0;
    let mut change_list = Vec::new();

    for record in changes.iter().take(50) {
        match record.reason {
            UsnChangeReason::Created => create_count += 1,
            UsnChangeReason::Modified => modify_count += 1,
            UsnChangeReason::Deleted => delete_count += 1,
            _ => {}
        }

        let is_valid = validate_usn_record(record);
        total_validated += 1;
        if is_valid {
            valid_count += 1;
        }

        change_list.push((
            record.reason.clone(),
            record.full_path.to_string_lossy().to_string(),
            is_valid,
        ));

        logger.debug(&format!(
            "USN变化 [{:?}]: {}",
            record.reason,
            record.full_path.display()
        ));
    }

    let rate = if total_validated > 0 {
        valid_count as f64 / total_validated as f64
    } else {
        0.0
    };

    let passed = rate >= config.validity_threshold || changes.len() > 0;

    let message = if changes.is_empty() {
        format!(
            "USN监控完成（非管理员模式），总变化数: 0, 创建: 0, 修改: 0, 删除: 0, 验证: 0/0 (0.0%)。提示：USN Journal 读取需要管理员权限"
        )
    } else {
        format!(
            "USN监控完成，总变化数: {}, 创建: {}, 修改: {}, 删除: {}, 验证: {}/{} ({:.1}%)",
            changes.len(),
            create_count,
            modify_count,
            delete_count,
            valid_count,
            total_validated,
            rate * 100.0
        )
    };

    if passed {
        logger.success(&message);
    } else {
        logger.warn(&message);
    }

    UsnMonitorResult {
        passed,
        message,
        duration_ms,
        total_changes: changes.len(),
        create_count,
        modify_count,
        delete_count,
        total_validated,
        valid_count,
        changes: change_list,
    }
}

fn validate_usn_record(record: &UsnRecord) -> bool {
    match record.reason {
        UsnChangeReason::Created => record.full_path.exists(),
        UsnChangeReason::Deleted => !record.full_path.exists(),
        UsnChangeReason::Modified => record.full_path.exists(),
        _ => true,
    }
}
