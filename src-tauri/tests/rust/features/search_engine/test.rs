use std::collections::HashSet;
use std::path::{Path, PathBuf};
use rand::prelude::SliceRandom;

use monotools_lib::engines::file_search::FileSearchEngine;

#[path = "../../common/paths.rs"]
mod paths;
#[path = "../../common/report.rs"]
mod report;
#[path = "../../common/table.rs"]
mod table;
#[path = "./config.rs"]
mod config;

use config::SearchEngineTestConfig;
use paths::{data_path, output_path, ensure_dir};
use report::{TestReport, TestResults};
use table::ValidationReport;

const MODULE_NAME: &str = "search_engine";

fn get_test_db_path() -> PathBuf {
    let dir = data_path(MODULE_NAME, "databases");
    ensure_dir(&dir);
    dir.join("test_index.db")
}

fn cleanup_test_data() {
    let data_dir = data_path(MODULE_NAME, "");
    if data_dir.exists() {
        let _ = std::fs::remove_dir_all(&data_dir);
    }
    ensure_dir(&data_dir);
}

fn get_system_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    for letter in ['C', 'D', 'E', 'F', 'G', 'H'] {
        let path = PathBuf::from(format!("{}:\\", letter));
        if path.exists() {
            roots.push(path);
        }
    }

    roots
}

fn get_common_search_keywords() -> Vec<&'static str> {
    vec![
        "notepad",
        "system32",
        "windows",
        "chrome",
        "explorer",
        "cmd",
        "powershell",
        "git",
        "python",
        "rust",
        "visual",
        "code",
        "dll",
        "exe",
        "config",
        "log",
        "txt",
        "pdf",
        "jpg",
        "png",
        "json",
        "xml",
        "html",
        "css",
        "js",
        "rs",
        "toml",
        "lock",
        "cache",
        "temp",
        "desktop",
        "download",
        "documents",
    ]
}

fn random_sample<T: Clone>(items: &[T], count: usize) -> Vec<T> {
    let mut rng = rand::thread_rng();
    let mut indices: Vec<usize> = (0..items.len()).collect();
    indices.shuffle(&mut rng);
    indices.into_iter()
        .take(count.min(items.len()))
        .map(|i| items[i].clone())
        .collect()
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

    let roots = get_system_roots();
    println!("DEBUG: 系统根目录: {:?}", roots);

    let engine = match FileSearchEngine::new_with_db_path(roots, db_path) {
        Ok(e) => e,
        Err(e) => {
            results.add_result("引擎创建", false, &format!("创建失败: {}", e), 0);
            report.add_section_item("错误", "引擎创建失败", &format!("{}", e));
            report.save(&output_path(MODULE_NAME, "summary.txt"));
            println!("引擎创建失败: {}", e);
            return;
        }
    };



    let t1 = test_index_building(&engine).await;
    results.add_result("索引构建", t1.passed, &t1.message, t1.duration_ms);
    let t1_status = if t1.passed { format!("通过, {} 个文件", t1.total_files) } else { "失败".to_string() };
    report.add_section_item("索引功能", "索引构建", &t1_status);

    if !t1.passed || t1.total_files == 0 {
        results.add_result("路径验证", false, "索引为空，跳过验证", 0);
        report.add_section_item("数据质量", "路径验证", "跳过（索引为空）");
        report.save(&output_path(MODULE_NAME, "summary.txt"));
        println!("索引为空，跳过后续测试");
        return;
    }

    let t2 = test_search_keywords(&engine, &config).await;
    results.add_result("关键词搜索", t2.passed, &t2.message, t2.duration_ms);
    let t2_status = format!("通过, 平均每关键词返回{}条", t2.avg_results);
    report.add_section_item("搜索功能", "关键词搜索", &t2_status);

    let t3 = test_path_validation(&engine, &config).await;
    results.add_result("路径验证", t3.passed, &t3.message, t3.duration_ms);
    let t3_status = format!("成功率{:.1}%", t3.validity_rate * 100.0);
    report.add_section_item("数据质量", "路径验证", &t3_status);

    let t4 = test_search_performance(&engine, &config).await;
    results.add_result("搜索性能", t4.passed, &t4.message, t4.duration_ms);
    let t4_status = format!("{}ms", t4.search_time_ms);
    report.add_section_item("性能指标", "搜索时间", &t4_status);

    let t5 = test_index_consistency(&engine).await;
    results.add_result("索引一致性", t5.passed, &t5.message, t5.duration_ms);
    let t5_status = if t5.passed { "通过".to_string() } else { "失败".to_string() };
    report.add_section_item("数据质量", "索引一致性", &t5_status);

    let (passed, failed) = results.summary();
    let total_str = format!("{}", passed + failed);
    let passed_str = format!("{}", passed);
    let failed_str = format!("{}", failed);
    report.add_section_item("测试结果", "总测试数", &total_str);
    report.add_section_item("测试结果", "通过", &passed_str);
    report.add_section_item("测试结果", "失败", &failed_str);

    let output_dir = output_path(MODULE_NAME, "");
    ensure_dir(&output_dir);
    report.save(&output_path(MODULE_NAME, "summary.txt"));

    println!("{}", results.generate_summary());
}

struct IndexBuildResult {
    passed: bool,
    message: String,
    duration_ms: u64,
    total_files: usize,
}

async fn test_index_building(engine: &FileSearchEngine) -> IndexBuildResult {
    let start = std::time::Instant::now();

    match engine.build_index().await {
        Ok(_) => {
            let total = engine.total();
            let duration_ms = start.elapsed().as_millis() as u64;

            if total == 0 {
                return IndexBuildResult {
                    passed: false,
                    message: "索引构建完成但文件数为零".to_string(),
                    duration_ms,
                    total_files: 0
                };
            }

            println!("DEBUG: 索引构建完成，总文件数: {}", total);
            IndexBuildResult {
                passed: true,
                message: format!("索引构建完成，共 {} 个文件", total),
                duration_ms,
                total_files: total
            }
        }
        Err(e) => {
            IndexBuildResult {
                passed: false,
                message: format!("索引构建失败: {}", e),
                duration_ms: 0,
                total_files: 0
            }
        }
    }
}

struct KeywordSearchResult {
    passed: bool,
    message: String,
    duration_ms: u64,
    avg_results: usize,
}

async fn test_search_keywords(engine: &FileSearchEngine, config: &SearchEngineTestConfig) -> KeywordSearchResult {
    let start = std::time::Instant::now();
    let keywords = get_common_search_keywords();
    let sample_keywords = random_sample(&keywords, config.search_keyword_count);

    let mut total_results = 0;
    let mut success_count = 0;

    for keyword in &sample_keywords {
        let results = engine.search(keyword, config.search_limit);
        total_results += results.len();
        if !results.is_empty() {
            success_count += 1;
        }
        println!("DEBUG: 搜索 '{}' 返回 {} 条结果", keyword, results.len());
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let avg_results = if sample_keywords.is_empty() { 0 } else { total_results / sample_keywords.len() };

    if success_count == 0 {
        return KeywordSearchResult {
            passed: false,
            message: "所有关键词搜索都返回空结果".to_string(),
            duration_ms,
            avg_results
        };
    }

    KeywordSearchResult {
        passed: true,
        message: format!("搜索 {} 个关键词，成功 {} 个，平均返回 {} 条", sample_keywords.len(), success_count, avg_results),
        duration_ms,
        avg_results
    }
}

struct PathValidationResult {
    passed: bool,
    message: String,
    duration_ms: u64,
    validity_rate: f64,
}

async fn test_path_validation(engine: &FileSearchEngine, config: &SearchEngineTestConfig) -> PathValidationResult {
    let start = std::time::Instant::now();
    let keywords = get_common_search_keywords();
    let sample_keywords = random_sample(&keywords, config.search_keyword_count);

    let mut all_results: Vec<String> = Vec::new();

    for keyword in &sample_keywords {
        let results = engine.search(keyword, config.search_limit);
        for r in results {
            all_results.push(r.id);
        }
    }

    if all_results.is_empty() {
        return PathValidationResult {
            passed: false,
            message: "没有找到任何搜索结果".to_string(),
            duration_ms: 0,
            validity_rate: 0.0
        };
    }

    let sample_size = std::cmp::min(config.sample_size, all_results.len());
    let samples = random_sample(&all_results, sample_size);

    let mut validation_report = ValidationReport::new("文件路径验证报告");
    let mut valid_count = 0;

    for path_str in &samples {
        let path = Path::new(path_str);
        let exists = path.exists();
        if exists {
            valid_count += 1;
        } else {
            println!("DEBUG: 路径不存在: {}", path_str);
        }

        let filename = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        validation_report.add_entry(&filename, path_str, exists);
    }

    let output_dir = output_path(MODULE_NAME, "");
    ensure_dir(&output_dir);
    validation_report.save(&output_path(MODULE_NAME, "path_validation.txt"));

    let validity_rate = valid_count as f64 / sample_size as f64;
    let duration_ms = start.elapsed().as_millis() as u64;

    if validity_rate < config.validity_threshold {
        return PathValidationResult {
            passed: false,
            message: format!("路径验证成功率不足: {:.2}%", validity_rate * 100.0),
            duration_ms,
            validity_rate
        };
    }

    PathValidationResult {
        passed: true,
        message: format!("验证 {} 个路径，成功 {} 个，成功率 {:.2}%", sample_size, valid_count, validity_rate * 100.0),
        duration_ms,
        validity_rate
    }
}

struct PerformanceResult {
    passed: bool,
    message: String,
    duration_ms: u64,
    search_time_ms: u64,
}

async fn test_search_performance(engine: &FileSearchEngine, config: &SearchEngineTestConfig) -> PerformanceResult {
    let keywords = get_common_search_keywords();
    let sample_keywords = random_sample(&keywords, 10);

    let start = std::time::Instant::now();
    for keyword in &sample_keywords {
        let _ = engine.search(keyword, config.search_limit);
    }
    let search_time_ms = start.elapsed().as_millis() as u64;

    if search_time_ms > config.max_search_time_ms {
        return PerformanceResult {
            passed: false,
            message: format!("搜索超时: {}ms", search_time_ms),
            duration_ms: search_time_ms,
            search_time_ms
        };
    }

    PerformanceResult {
        passed: true,
        message: format!("10次搜索耗时 {}ms", search_time_ms),
        duration_ms: search_time_ms,
        search_time_ms
    }
}

struct ConsistencyResult {
    passed: bool,
    message: String,
    duration_ms: u64,
}

async fn test_index_consistency(engine: &FileSearchEngine) -> ConsistencyResult {
    let start = std::time::Instant::now();
    let total = engine.total();

    if total == 0 {
        return ConsistencyResult {
            passed: true,
            message: "索引为空，跳过一致性检查".to_string(),
            duration_ms: 0
        };
    }

    let results = engine.search("", std::cmp::min(total, 1000) as u32);
    let paths: HashSet<String> = results.iter().map(|r| r.id.clone()).collect();

    let duration_ms = start.elapsed().as_millis() as u64;

    if paths.len() != results.len() {
        return ConsistencyResult {
            passed: false,
            message: format!("搜索结果有重复路径，总数{}，去重后{}", results.len(), paths.len()),
            duration_ms
        };
    }

    ConsistencyResult {
        passed: true,
        message: format!("一致性检查通过，{} 条结果无重复", results.len()),
        duration_ms
    }
}
