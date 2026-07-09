use std::collections::HashMap;
use std::path::PathBuf;

pub struct TestReport {
    module_name: String,
    sections: Vec<ReportSection>,
    timestamp: String,
}

pub struct ReportSection {
    title: String,
    items: Vec<ReportItem>,
}

pub struct ReportItem {
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

    pub fn save(&self, path: &PathBuf) {
        let _ = std::fs::write(path, self.generate());
    }
}

pub struct TestResults {
    results: HashMap<String, TestResult>,
}

pub struct TestResult {
    passed: bool,
    message: String,
    duration_ms: u64,
}

impl TestResults {
    pub fn new() -> Self {
        TestResults {
            results: HashMap::new(),
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
