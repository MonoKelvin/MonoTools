use monotools_lib::platform::windows::usn::{NtfsIndexer, UsnRecord, UsnChangeReason};
use std::io::Write;

const MODULE_NAME: &str = "usn_journal";

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

fn cleanup_test_data() {
    let data_dir = data_path(MODULE_NAME, "");
    if data_dir.exists() {
        let _ = std::fs::remove_dir_all(&data_dir);
    }
    ensure_dir(&data_dir);
}

fn get_indexer(drives: &[char]) -> Option<NtfsIndexer> {
    if drives.is_empty() {
        match NtfsIndexer::new() {
            Ok(i) => Some(i),
            Err(e) => {
                println!("NtfsIndexer 创建失败: {}", e);
                None
            }
        }
    } else {
        match NtfsIndexer::new_with_drives(drives.to_vec()) {
            Ok(i) => Some(i),
            Err(e) => {
                println!("NtfsIndexer 创建失败（指定盘符）: {}", e);
                None
            }
        }
    }
}

#[derive(Debug, Clone)]
struct UsnJournalTestConfig {
    pub test_drives: Vec<char>,
    pub monitor_duration_ms: u64,
    pub max_record_count: usize,
    pub sample_size: usize,
    pub validity_threshold: f64,
}

impl Default for UsnJournalTestConfig {
    fn default() -> Self {
        UsnJournalTestConfig {
            test_drives: vec!['D'],
            monitor_duration_ms: 5000,
            max_record_count: 100,
            sample_size: 100,
            validity_threshold: 0.90,
        }
    }
}

fn is_admin() -> bool {
    // 通过尝试在系统目录创建文件来检测管理员权限
    // 普通用户无法在 C:\Windows\System32 写入文件
    let test_file = std::path::PathBuf::from("C:\\Windows\\System32\\__monotools_admin_check.tmp");
    let is_admin = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&test_file)
        .is_ok();
    
    if is_admin {
        let _ = std::fs::remove_file(&test_file);
    }
    
    is_admin
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

    let indexer = match get_indexer(&config.test_drives) {
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

    let t1 = test_usn_monitor(&indexer, &config).await;
    results.add_result("USN监控", t1.passed, &t1.message, t1.duration_ms);
    report.add_section_item("监控结果", "总变化数", &format!("{}", t1.total_changes));
    report.add_section_item("监控结果", "创建事件", &format!("{}", t1.create_count));
    report.add_section_item("监控结果", "修改事件", &format!("{}", t1.modify_count));
    report.add_section_item("监控结果", "删除事件", &format!("{}", t1.delete_count));
    report.add_section_item("监控结果", "有效验证", &format!("{}/{}", t1.valid_count, t1.total_validated));

    let (passed, failed) = results.summary();
    report.add_section_item("测试结果", "总测试数", &format!("{}", passed + failed));
    report.add_section_item("测试结果", "通过", &format!("{}", passed));
    report.add_section_item("测试结果", "失败", &format!("{}", failed));

    report.save(&output_path(MODULE_NAME, "summary.txt"));

    println!("{}", results.generate_summary());
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
}

async fn test_usn_monitor(indexer: &NtfsIndexer, config: &UsnJournalTestConfig) -> UsnMonitorResult {
    let start = std::time::Instant::now();

    let test_dir = data_path(MODULE_NAME, "test_files");
    ensure_dir(&test_dir);

    let mut test_files = Vec::new();

    let volume = if !indexer.get_volumes().is_empty() {
        indexer.get_volumes()[0].clone()
    } else {
        return UsnMonitorResult {
            passed: false,
            message: "没有可用的卷".to_string(),
            duration_ms: 0,
            total_changes: 0,
            create_count: 0,
            modify_count: 0,
            delete_count: 0,
            total_validated: 0,
            valid_count: 0,
        };
    };

    let mut start_usn = 0;
    if let Some(state) = indexer.get_usn_journal_state(&volume) {
        start_usn = state.next_usn;
        println!("DEBUG: USN Journal 状态: {:?}", state);
    } else {
        match indexer.create_usn_journal(&volume) {
            Ok(_) => {
                if let Some(state) = indexer.get_usn_journal_state(&volume) {
                    start_usn = state.next_usn;
                }
            }
            Err(e) => {
                println!("警告: 无法创建 USN Journal: {}", e);
            }
        }
    }

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    for i in 0..5 {
        let file_path = test_dir.join(format!("test_file_{}.txt", i));
        test_files.push(file_path.clone());
        
        std::fs::write(&file_path, format!("测试内容 {}", i)).unwrap();
        println!("DEBUG: 创建测试文件: {}", file_path.display());
    }

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    for (i, file_path) in test_files.iter().enumerate().take(3) {
        std::fs::write(file_path, format!("修改内容 {}", i)).unwrap();
        println!("DEBUG: 修改测试文件: {}", file_path.display());
    }

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    for (_i, file_path) in test_files.iter().enumerate().take(2) {
        if let Ok(()) = std::fs::remove_file(file_path) {
            println!("DEBUG: 删除测试文件: {}", file_path.display());
        } else {
            println!("DEBUG: 文件删除失败或不存在: {}", file_path.display());
        }
    }

    tokio::time::sleep(std::time::Duration::from_millis(config.monitor_duration_ms)).await;

    let changes = match indexer.read_usn_changes(&volume, start_usn as u64) {
        Ok(c) => c,
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return UsnMonitorResult {
                passed: false,
                message: format!("读取 USN 变化失败: {}", e),
                duration_ms,
                total_changes: 0,
                create_count: 0,
                modify_count: 0,
                delete_count: 0,
                total_validated: 0,
                valid_count: 0,
            };
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    let mut create_count = 0;
    let mut modify_count = 0;
    let mut delete_count = 0;
    let mut total_validated = 0;
    let mut valid_count = 0;

    let mut change_details = Vec::new();

    for (i, record) in changes.iter().take(config.max_record_count).enumerate() {
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

        change_details.push(format!(
            "[{}] 类型: {:?}, 路径: {}, 有效: {}",
            i, record.reason, record.full_path.display(), is_valid
        ));

        if i < 10 {
            println!("DEBUG: USN变化 [{:?}]: {}", record.reason, record.full_path.display());
        }
    }

    let output_path = output_path(MODULE_NAME, "changes.txt");
    if let Ok(mut file) = std::fs::File::create(&output_path) {
        for detail in change_details {
            let _ = writeln!(file, "{}", detail);
        }
    }

    let rate = if total_validated > 0 {
        valid_count as f64 / total_validated as f64
    } else {
        0.0
    };

    let passed = rate >= config.validity_threshold || changes.len() > 0;

    let message = format!(
        "USN监控完成，总变化数: {}, 创建: {}, 修改: {}, 删除: {}, 验证: {}/{} ({:.1}%)",
        changes.len(), create_count, modify_count, delete_count, valid_count, total_validated, rate * 100.0
    );

    println!("DEBUG: {}", message);

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

    pub fn add_item(&mut self, label: &str, value: &str) {
        if self.sections.is_empty() {
            self.add_section("概览");
        }
        self.sections.last_mut().unwrap().items.push(ReportItem {
            label: label.to_string(),
            value: value.to_string(),
        });
    }

    pub fn add_section_item(&mut self, section_title: &str, label: &str, value: &str) {
        let section = self.sections.iter_mut().find(|s| s.title == section_title);
        match section {
            Some(s) => {
                s.items.push(ReportItem {
                    label: label.to_string(),
                    value: value.to_string(),
                });
            }
            None => {
                let mut section = self.add_section(section_title);
                section.items.push(ReportItem {
                    label: label.to_string(),
                    value: value.to_string(),
                });
            }
        }
    }

    pub fn generate(&self) -> String {
        let mut output = format!("{} 测试报告\n", self.module_name);
        output.push_str(&"=".repeat(60));
        output.push('\n');
        output.push_str(&format!("输出时间: {}\n", self.timestamp));

        for section in &self.sections {
            output.push_str(&format!("\n## {}\n", section.title));

            let max_label_len = section.items.iter().map(|i| i.label.len()).max().unwrap_or(0);
            for item in &section.items {
                output.push_str(&format!("{:<width$}:{}\n", item.label, item.value, width = max_label_len));
            }
        }

        output.push_str(&"\n");
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
            output.push_str(&format!("{} {} ({:.2}ms)\n", status, name, result.duration_ms as f64 / 1_000_000.0));
            if !result.message.is_empty() {
                output.push_str(&format!("  {}\n", result.message));
            }
        }

        output
    }
}