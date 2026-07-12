//! 视觉更花哨的测试报告器：表盘样式边框、彩色的 ✓/✗、测试 / 表格 / 详情聚合。
//!
//! 适用场景：在 cargo test 输出需要更醒目摘要时使用。文件保存会自动剥离 ANSI 颜色。
//!
//! ## 典型用法
//!
//! ```rust
//! use monotools_tests::reporter::TestReporter;
//! use monotools_tests::paths::timestamped_output_path;
//!
//! let mut reporter = TestReporter::new("search_engine");
//! reporter.add_test("索引构建");
//! reporter.finish_test("索引构建", true, 1234, "成功");
//!
//! reporter.save(&timestamped_output_path("search_engine", "summary", "txt"));
//! ```

use std::path::PathBuf;

pub struct TestReporter {
    module_name: String,
    tests: Vec<TestResultItem>,
    timestamp: String,
    start_time: std::time::Instant,
    #[allow(dead_code)]
    search_stats: Vec<SearchStats>,
}

pub struct TestResultItem {
    pub name: String,
    pub passed: bool,
    pub duration_ms: u64,
    pub message: String,
    pub details: Vec<String>,
    pub tables: Vec<String>,
}

#[allow(dead_code)]
pub struct SearchStats {
    query: String,
    total_results: usize,
    valid_count: usize,
    avg_time_ms: u64,
    match_type: String,
}

impl SearchStats {
    #[allow(dead_code)]
    pub fn new(
        query: &str,
        total_results: usize,
        valid_count: usize,
        avg_time_ms: u64,
        match_type: &str,
    ) -> Self {
        SearchStats {
            query: query.to_string(),
            total_results,
            valid_count,
            avg_time_ms,
            match_type: match_type.to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn generate_table(stats_list: &[SearchStats]) -> String {
        if stats_list.is_empty() {
            return "无搜索统计数据".to_string();
        }

        let mut output = String::new();

        let max_query_len = stats_list.iter().map(|s| s.query.len()).max().unwrap_or(8);
        let max_type_len = stats_list.iter().map(|s| s.match_type.len()).max().unwrap_or(8);

        let q_len = std::cmp::max(max_query_len, 8);
        let t_len = std::cmp::max(max_type_len, 8);

        let total_width = q_len + t_len + 46;

        output.push('\n');
        output.push_str(&"─".repeat(total_width));
        output.push('\n');

        output.push_str(&format!(
            "{:<width$} | {:<type_width$} | {:>8} | {:>8} | {:>10}\n",
            "查询词", "匹配类型", "总结果", "有效率", "耗时(ms)",
            width = q_len,
            type_width = t_len
        ));

        output.push_str(&"─".repeat(total_width));
        output.push('\n');

        for stats in stats_list {
            let rate = if stats.total_results > 0 {
                (stats.valid_count as f64 / stats.total_results as f64 * 100.0).round() as usize
            } else {
                0
            };

            output.push_str(&format!(
                "{:<width$} | {:<type_width$} | {:>8} | {:>7}% | {:>10}\n",
                stats.query,
                stats.match_type,
                stats.total_results,
                rate,
                stats.avg_time_ms,
                width = q_len,
                type_width = t_len
            ));
        }

        output.push_str(&"─".repeat(total_width));
        output.push('\n');

        output
    }
}

impl TestReporter {
    pub fn new(module_name: &str) -> Self {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        TestReporter {
            module_name: module_name.to_string(),
            tests: Vec::new(),
            timestamp,
            start_time: std::time::Instant::now(),
            search_stats: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add_search_stats(&mut self, stats: SearchStats) {
        self.search_stats.push(stats);
    }

    pub fn add_test(&mut self, name: &str) -> &mut TestResultItem {
        self.tests.push(TestResultItem {
            name: name.to_string(),
            passed: false,
            duration_ms: 0,
            message: String::new(),
            details: Vec::new(),
            tables: Vec::new(),
        });
        self.tests.last_mut().unwrap()
    }

    pub fn finish_test(&mut self, name: &str, passed: bool, duration_ms: u64, message: &str) {
        if let Some(test) = self.tests.iter_mut().find(|t| t.name == name) {
            test.passed = passed;
            test.duration_ms = duration_ms;
            test.message = message.to_string();
        }
    }

    pub fn add_test_detail(&mut self, name: &str, detail: &str) {
        if let Some(test) = self.tests.iter_mut().find(|t| t.name == name) {
            test.details.push(detail.to_string());
        }
    }

    pub fn add_test_table(&mut self, name: &str, table: String) {
        if let Some(test) = self.tests.iter_mut().find(|t| t.name == name) {
            test.tables.push(table);
        }
    }

    pub fn generate(&self) -> String {
        let total_time_ms = self.start_time.elapsed().as_millis() as u64;
        let (passed, failed) = self.summary();
        let total = self.tests.len();

        let mut output = String::new();

        output.push('\n');
        output.push_str(&"╔════════════════════════════════════════════════════════════════════════════════╗\n");
        output.push_str(&format!(
            "║ {:^76} ║\n",
            format!("{} 测试报告", self.module_name)
        ));
        output.push_str(&"╠════════════════════════════════════════════════════════════════════════════════╣\n");
        output.push_str(&format!("║ 输出时间: {:<63} ║\n", self.timestamp));
        output.push_str(&format!(
            "║ 总耗时:   {:<63} ║\n",
            format!("{}ms", total_time_ms)
        ));
        output.push_str(&format!("║ 测试总数: {:<63} ║\n", total));
        output.push_str(&format!(
            "║ 通过:     {:<63} ║\n",
            format!(
                "{} ({:.1}%)",
                passed,
                if total > 0 {
                    passed as f64 / total as f64 * 100.0
                } else {
                    0.0
                }
            )
        ));
        output.push_str(&format!(
            "║ 失败:     {:<63} ║\n",
            format!(
                "{} ({:.1}%)",
                failed,
                if total > 0 {
                    failed as f64 / total as f64 * 100.0
                } else {
                    0.0
                }
            )
        ));
        output.push_str(&"╠════════════════════════════════════════════════════════════════════════════════╣\n");

        for (i, test) in self.tests.iter().enumerate() {
            let status = if test.passed { "✓" } else { "✗" };
            let status_color = if test.passed { "\x1b[32m" } else { "\x1b[31m" };
            let reset_color = "\x1b[0m";

            output.push_str(&format!(
                "║ {} [{:02}] {:<68} ║\n",
                format!("{}{}{}", status_color, status, reset_color),
                i + 1,
                test.name
            ));
            output.push_str(&"╠────────────────────────────────────────────────────────────────────────────────╣\n");

            output.push_str(&format!(
                "║   耗时: {}ms                        状态: {} ║\n",
                test.duration_ms,
                if test.passed { "通过" } else { "失败" }
            ));

            if !test.message.is_empty() {
                let lines = self.wrap_text(&test.message, 74);
                for line in lines {
                    output.push_str(&format!("║   {}\n", line));
                }
            }

            if !test.tables.is_empty() {
                output.push_str(&"╠────────────────────────────────────────────────────────────────────────────────╣\n");
                for table in &test.tables {
                    for line in table.lines() {
                        output.push_str(&format!("║ {}\n", line));
                    }
                }
            }

            if !test.details.is_empty() {
                output.push_str(&"╠────────────────────────────────────────────────────────────────────────────────╣\n");
                for (j, detail) in test.details.iter().enumerate() {
                    let lines = self.wrap_text(detail, 74);
                    for (k, line) in lines.iter().enumerate() {
                        if k == 0 {
                            output.push_str(&format!("║   [{:02}] {}\n", j + 1, line));
                        } else {
                            output.push_str(&format!("║         {}\n", line));
                        }
                    }
                }
            }

            if i < self.tests.len() - 1 {
                output.push_str(&"╟────────────────────────────────────────────────────────────────────────────────╢\n");
            }
        }

        output.push_str(&"╚════════════════════════════════════════════════════════════════════════════════╝\n");

        output
    }

    fn wrap_text(&self, text: &str, width: usize) -> Vec<String> {
        let mut result = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            if current_line.len() + word.len() + 1 <= width {
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            } else {
                result.push(current_line);
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            result.push(current_line);
        }

        result
    }

    pub fn summary(&self) -> (usize, usize) {
        let passed = self.tests.iter().filter(|t| t.passed).count();
        let failed = self.tests.len() - passed;
        (passed, failed)
    }

    pub fn print(&self) {
        println!("{}", self.generate());
    }

    pub fn save(&self, path: &PathBuf) {
        let content = self
            .generate()
            .replace("\x1b[32m", "")
            .replace("\x1b[31m", "")
            .replace("\x1b[0m", "");
        let _ = std::fs::write(path, content);
    }
}
