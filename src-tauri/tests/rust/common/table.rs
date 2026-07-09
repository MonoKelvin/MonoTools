pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    col_widths: Vec<usize>,
    separator: String,
}

impl Table {
    pub fn new(headers: &[&str]) -> Self {
        let headers: Vec<String> = headers.iter().map(|s| s.to_string()).collect();
        let col_widths = headers.iter().map(|h| h.len()).collect();

        Table {
            headers,
            rows: Vec::new(),
            col_widths,
            separator: "-".to_string(),
        }
    }

    pub fn add_row(&mut self, values: &[&str]) {
        let mut row: Vec<String> = Vec::new();
        for (i, value) in values.iter().enumerate() {
            row.push(value.to_string());
            if i < self.col_widths.len() && value.len() > self.col_widths[i] {
                self.col_widths[i] = value.len();
            }
        }
        self.rows.push(row);
    }

    pub fn add_path_validation_row(&mut self, filename: &str, path: &str, exists: bool) {
        let status = if exists { "√" } else { "×" };
        self.add_row(&[filename, path, "--", status]);
    }

    pub fn generate(&self) -> String {
        let mut output = String::new();

        let header_line = self.headers.iter()
            .enumerate()
            .map(|(i, h)| format!("{:<width$}", h, width = self.col_widths[i] + 4))
            .collect::<Vec<_>>()
            .join("");
        output.push_str(&header_line);
        output.push('\n');

        let separator_line = self.col_widths.iter()
            .map(|w| self.separator.repeat(*w + 4))
            .collect::<Vec<_>>()
            .join("");
        output.push_str(&separator_line);
        output.push('\n');

        for row in &self.rows {
            let row_line = row.iter()
                .enumerate()
                .map(|(i, v)| format!("{:<width$}", v, width = self.col_widths[i] + 4))
                .collect::<Vec<_>>()
                .join("");
            output.push_str(&row_line);
            output.push('\n');
        }

        output
    }

    pub fn save(&self, path: &std::path::PathBuf) {
        let _ = std::fs::write(path, self.generate());
    }

    pub fn generate_timestamp_filename(base_name: &str) -> String {
        let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        format!("{}_{}.txt", base_name, timestamp)
    }
}

pub struct ValidationReport {
    title: String,
    table: Table,
    passed_count: usize,
    failed_count: usize,
    total_count: usize,
    timestamp: String,
}

impl ValidationReport {
    pub fn new(title: &str) -> Self {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        ValidationReport {
            title: title.to_string(),
            table: Table::new(&["文件名", "完整路径", "分隔符", "状态"]),
            passed_count: 0,
            failed_count: 0,
            total_count: 0,
            timestamp,
        }
    }

    pub fn add_entry(&mut self, filename: &str, path: &str, exists: bool) {
        self.table.add_path_validation_row(filename, path, exists);
        if exists {
            self.passed_count += 1;
        } else {
            self.failed_count += 1;
        }
        self.total_count += 1;
    }

    pub fn generate(&self) -> String {
        let mut output = format!("{}\n", self.title);
        output.push_str(&"=".repeat(80));
        output.push('\n');
        output.push_str(&format!("输出时间: {}\n", self.timestamp));
        output.push('\n');

        output.push_str(&self.table.generate());

        output.push_str(&"\n");
        output.push_str(&"=".repeat(80));
        output.push('\n');

        let pass_rate = if self.total_count > 0 {
            (self.passed_count as f64 / self.total_count as f64) * 100.0
        } else {
            0.0
        };

        output.push_str(&format!("总样本数: {}\n", self.total_count));
        output.push_str(&format!("通过: {} ({}%)\n", self.passed_count, pass_rate));
        output.push_str(&format!("未通过: {} ({:.1}%)\n", self.failed_count, (100.0 - pass_rate)));
        output.push_str(&"=".repeat(80));

        output
    }

    pub fn save(&self, path: &std::path::PathBuf) {
        let _ = std::fs::write(path, self.generate());
    }
}
