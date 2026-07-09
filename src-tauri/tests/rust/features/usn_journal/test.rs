use std::path::PathBuf;
use rand::Rng;

use monotools_lib::platform::windows::usn::{NtfsIndexer, UsnChangeReason, UsnRecord};

#[path = "../../common/paths.rs"]
mod paths;
#[path = "../../common/report.rs"]
mod report;
#[path = "../../common/table.rs"]
mod table;
#[path = "./config.rs"]
mod config;

use config::UsnJournalTestConfig;
use paths::{data_path, output_path, ensure_dir};
use report::{TestReport, TestResults};
use table::ValidationReport;

const MODULE_NAME: &str = "usn_journal";

fn setup_test_dir() -> PathBuf {
    let dir = data_path(MODULE_NAME, "test_files");
    ensure_dir(&dir);
    dir
}

fn cleanup_test_data() {
    let data_dir = data_path(MODULE_NAME, "");
    if data_dir.exists() {
        let _ = std::fs::remove_dir_all(&data_dir);
    }
    ensure_dir(&data_dir);
}

fn cleanup_test(test_dir: &PathBuf, created_files: &[PathBuf]) {
    for path in created_files {
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn get_indexer() -> Option<NtfsIndexer> {
    match NtfsIndexer::new() {
        Ok(i) => Some(i),
        Err(e) => {
            println!("NtfsIndexer 创建失败: {}", e);
            None
        }
    }
}

#[tokio::test]
async fn run_all_usn_journal_tests() {
    run_usn_journal_tests().await;
}

pub async fn run_usn_journal_tests() {
    let config = UsnJournalTestConfig::default();
    let mut report = TestReport::new("USN Journal 监控");
    let mut results = TestResults::new();

    cleanup_test_data();

    let indexer = match get_indexer() {
        Some(i) => i,
        None => {
            report.add_item("状态", "NtfsIndexer 创建失败");
            report.save(&output_path(MODULE_NAME, "summary.txt"));
            println!("NtfsIndexer 创建失败，跳过所有测试");
            return;
        }
    };

    let volumes = indexer.get_volumes();
    if volumes.is_empty() {
        report.add_item("状态", "未检测到 NTFS 卷");
        report.save(&output_path(MODULE_NAME, "summary.txt"));
        println!("未检测到 NTFS 卷，跳过所有测试");
        return;
    }

    let t1 = test_usn_journal_state(&indexer, &volumes);
    results.add_result("USN Journal状态", t1.passed, &t1.message, t1.duration_ms);
    report.add_section_item("状态检测", "检测到卷数", &format!("{}", t1.volume_count));

    let t2 = test_usn_change_monitor(&indexer, &config);
    results.add_result("变化监控", t2.passed, &t2.message, t2.duration_ms);
    report.add_section_item("变化监控", "总变化数", &format!("{}", t2.total_changes));
    report.add_section_item("变化监控", "创建事件", &format!("{}", t2.create_count));
    report.add_section_item("变化监控", "修改事件", &format!("{}", t2.modify_count));
    report.add_section_item("变化监控", "删除事件", &format!("{}", t2.delete_count));

    let t3 = test_ntfs_volume_enumeration(&indexer, &volumes, &config);
    results.add_result("卷枚举", t3.passed, &t3.message, t3.duration_ms);
    report.add_section_item("卷枚举", "总记录数", &format!("{}", t3.total_count));
    report.add_section_item("卷枚举", "目录数", &format!("{}", t3.dir_count));
    report.add_section_item("卷枚举", "文件数", &format!("{}", t3.file_count));

    let t4 = test_usn_record_path_validation(&indexer, &volumes, &config);
    results.add_result("路径验证", t4.passed, &t4.message, t4.duration_ms);
    report.add_section_item("数据质量", "路径验证成功率", &format!("{:.1}%", t4.validity_rate * 100.0));

    let t5 = test_usn_change_reason_classification(&indexer, &config);
    results.add_result("变化分类", t5.passed, &t5.message, t5.duration_ms);
    report.add_section_item("变化分类", "总变化数", &format!("{}", t5.total_changes));

    let output_dir = output_path(MODULE_NAME, "");
    ensure_dir(&output_dir);
    report.save(&output_path(MODULE_NAME, "summary.txt"));

    let (passed, failed) = results.summary();
    report.add_section_item("测试结果", "总测试数", &format!("{}", passed + failed));
    report.add_section_item("测试结果", "通过", &format!("{}", passed));
    report.add_section_item("测试结果", "失败", &format!("{}", failed));

    println!("{}", results.generate_summary());
}

struct JournalStateResult {
    passed: bool,
    message: String,
    duration_ms: u64,
    volume_count: usize,
}

fn test_usn_journal_state(indexer: &NtfsIndexer, volumes: &[String]) -> JournalStateResult {
    let start = std::time::Instant::now();

    let mut active_count = 0;
    for volume in volumes {
        if indexer.get_journal_state(volume).is_some() {
            active_count += 1;
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    if active_count == 0 {
        return JournalStateResult { passed: false, message: "没有可用的 USN Journal".to_string(), duration_ms, volume_count: volumes.len() };
    }

    JournalStateResult { passed: true, message: format!("{}个卷有Journal", active_count), duration_ms, volume_count: volumes.len() }
}

struct ChangeMonitorResult {
    passed: bool,
    message: String,
    duration_ms: u64,
    total_changes: usize,
    create_count: usize,
    modify_count: usize,
    delete_count: usize,
}

fn test_usn_change_monitor(indexer: &NtfsIndexer, config: &UsnJournalTestConfig) -> ChangeMonitorResult {
    let start = std::time::Instant::now();
    let test_dir = setup_test_dir();

    let mut created_files = Vec::new();

    for i in 0..10 {
        let filename = format!("usn_test_create_{}_{}.txt", i, rand::thread_rng().gen::<u32>());
        let path = test_dir.join(&filename);
        let _ = std::fs::write(&path, "test content");
        created_files.push(path);
    }

    for i in 0..5 {
        let path = &created_files[i];
        let _ = std::fs::write(path, "modified content");
    }

    for i in 0..3 {
        let path = &created_files[7 + i];
        let _ = std::fs::remove_file(path);
    }

    std::thread::sleep(std::time::Duration::from_secs(config.monitor_wait_seconds));

    let changes = match indexer.get_all_changes() {
        Ok(c) => c,
        Err(e) => {
            cleanup_test(&test_dir, &created_files);
            return ChangeMonitorResult { passed: false, message: format!("获取 USN 变化失败: {}", e), duration_ms: 0, total_changes: 0, create_count: 0, modify_count: 0, delete_count: 0 };
        }
    };

    let sample_size = std::cmp::min(config.sample_size, changes.len());
    let samples: Vec<UsnRecord> = changes.clone().into_iter().take(sample_size).collect();

    let mut create_count = 0;
    let mut modify_count = 0;
    let mut delete_count = 0;

    for change in &samples {
        match change.reason {
            UsnChangeReason::Created => create_count += 1,
            UsnChangeReason::Modified => modify_count += 1,
            UsnChangeReason::Deleted => delete_count += 1,
            _ => {}
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    cleanup_test(&test_dir, &created_files);

    ChangeMonitorResult { passed: true, message: "".to_string(), duration_ms, total_changes: changes.len(), create_count, modify_count, delete_count }
}

struct VolumeEnumerationResult {
    passed: bool,
    message: String,
    duration_ms: u64,
    total_count: usize,
    dir_count: usize,
    file_count: usize,
}

fn test_ntfs_volume_enumeration(indexer: &NtfsIndexer, volumes: &[String], _config: &UsnJournalTestConfig) -> VolumeEnumerationResult {
    let start = std::time::Instant::now();
    let volume = &volumes[0];

    let mut total_count = 0;
    let mut dir_count = 0;
    let mut file_count = 0;

    let result = indexer.enumerate_volume_files(volume, |record| {
        total_count += 1;
        if record.is_directory {
            dir_count += 1;
        } else {
            file_count += 1;
        }
    });

    let duration_ms = start.elapsed().as_millis() as u64;

    if let Err(e) = result {
        return VolumeEnumerationResult { passed: false, message: format!("枚举失败: {}", e), duration_ms, total_count: 0, dir_count: 0, file_count: 0 };
    }

    if total_count == 0 {
        return VolumeEnumerationResult { passed: false, message: "未枚举到任何文件".to_string(), duration_ms, total_count, dir_count, file_count };
    }

    VolumeEnumerationResult { passed: true, message: "".to_string(), duration_ms, total_count, dir_count, file_count }
}

struct PathValidationResult {
    passed: bool,
    message: String,
    duration_ms: u64,
    validity_rate: f64,
}

fn test_usn_record_path_validation(indexer: &NtfsIndexer, volumes: &[String], config: &UsnJournalTestConfig) -> PathValidationResult {
    let start = std::time::Instant::now();
    let volume = &volumes[0];

    let mut records = Vec::new();

    let result = indexer.enumerate_volume_files(volume, |record| {
        if records.len() < config.max_sample_records {
            records.push(record);
        }
    });

    let duration_ms = start.elapsed().as_millis() as u64;

    if let Err(e) = result {
        return PathValidationResult { passed: false, message: format!("枚举失败: {}", e), duration_ms, validity_rate: 0.0 };
    }

    let mut validation_report = ValidationReport::new("USN Record 路径验证报告");

    let mut valid_count = 0;
    for record in &records {
        let exists = record.full_path.exists();
        if exists {
            valid_count += 1;
        }

        let filename = record.full_path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        let path_str = record.full_path.to_string_lossy().to_string();
        validation_report.add_entry(&filename, &path_str, exists);
    }

    let output_dir = output_path(MODULE_NAME, "");
    ensure_dir(&output_dir);
    validation_report.save(&output_path(MODULE_NAME, "path_validation.txt"));

    let validity_rate = if records.is_empty() { 1.0 } else { valid_count as f64 / records.len() as f64 };

    if validity_rate < config.validity_threshold {
        return PathValidationResult { passed: false, message: format!("路径验证成功率不足: {:.2}%", validity_rate * 100.0), duration_ms, validity_rate };
    }

    PathValidationResult { passed: true, message: "".to_string(), duration_ms, validity_rate }
}

struct ReasonClassificationResult {
    passed: bool,
    message: String,
    duration_ms: u64,
    total_changes: usize,
}

fn test_usn_change_reason_classification(indexer: &NtfsIndexer, config: &UsnJournalTestConfig) -> ReasonClassificationResult {
    let start = std::time::Instant::now();
    let test_dir = setup_test_dir();

    let test_file = test_dir.join("usn_reason_test.txt");
    let _ = std::fs::write(&test_file, "initial");

    std::thread::sleep(std::time::Duration::from_secs(1));

    let _ = std::fs::write(&test_file, "modified");

    std::thread::sleep(std::time::Duration::from_secs(1));

    let _ = std::fs::remove_file(&test_file);

    std::thread::sleep(std::time::Duration::from_secs(config.monitor_wait_seconds));

    let changes = match indexer.get_all_changes() {
        Ok(c) => c,
        Err(e) => {
            return ReasonClassificationResult { passed: false, message: format!("获取 USN 变化失败: {}", e), duration_ms: 0, total_changes: 0 };
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    ReasonClassificationResult { passed: true, message: "".to_string(), duration_ms, total_changes: changes.len() }
}
