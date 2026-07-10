use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct TestLogger {
    file: Mutex<Option<File>>,
    enabled: bool,
    module_name: String,
    log_dir: PathBuf,
}

impl TestLogger {
    pub fn new(module_name: &str, log_dir: &PathBuf) -> Self {
        let mut logger = TestLogger {
            file: Mutex::new(None),
            enabled: true,
            module_name: module_name.to_string(),
            log_dir: log_dir.clone(),
        };
        logger.init();
        logger
    }

    fn init(&mut self) {
        if !self.enabled {
            return;
        }

        if let Err(e) = fs::create_dir_all(&self.log_dir) {
            eprintln!("无法创建日志目录: {}", e);
            self.enabled = false;
            return;
        }

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("log_{}.txt", timestamp);
        let filepath = self.log_dir.join(filename);

        match OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&filepath)
        {
            Ok(file) => {
                *self.file.lock().unwrap() = Some(file);
                self.info(&format!("日志文件已创建: {}", filepath.display()));
            }
            Err(e) => {
                eprintln!("无法打开日志文件: {}", e);
                self.enabled = false;
            }
        }
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
        if self.file.lock().unwrap().is_none() {
            self.init();
        }
    }

    fn log(&self, level: &str, message: &str) {
        if !self.enabled {
            return;
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let log_line = format!(
            "[{}] [{}] [{}] {}\n",
            timestamp, self.module_name, level, message
        );

        println!("{}", log_line.trim_end());

        if let Ok(mut guard) = self.file.lock() {
            if let Some(ref mut file) = *guard {
                let _ = file.write_all(log_line.as_bytes());
                let _ = file.flush();
            }
        }
    }

    pub fn debug(&self, message: &str) {
        self.log("DEBUG", message);
    }

    pub fn info(&self, message: &str) {
        self.log("INFO", message);
    }

    pub fn warn(&self, message: &str) {
        self.log("WARN", message);
    }

    pub fn error(&self, message: &str) {
        self.log("ERROR", message);
    }

    pub fn success(&self, message: &str) {
        self.log("SUCCESS", message);
    }

    pub fn section(&self, title: &str) {
        if !self.enabled {
            return;
        }

        let separator = "═".repeat(60);
        let line = format!("\n{}\n{}\n{}\n", separator, title, separator);

        println!("{}", line.trim_end());

        if let Ok(mut guard) = self.file.lock() {
            if let Some(ref mut file) = *guard {
                let _ = file.write_all(line.as_bytes());
                let _ = file.flush();
            }
        }
    }

    pub fn subsection(&self, title: &str) {
        if !self.enabled {
            return;
        }

        let line = format!("\n--- {}", title);

        println!("{}", line);

        if let Ok(mut guard) = self.file.lock() {
            if let Some(ref mut file) = *guard {
                let _ = file.write_all(line.as_bytes());
                let _ = file.write_all(b"\n");
                let _ = file.flush();
            }
        }
    }

    pub fn table(&self, headers: &[&str], rows: &[Vec<&str>]) {
        if !self.enabled || rows.is_empty() {
            return;
        }

        let mut col_widths = Vec::new();
        for (i, header) in headers.iter().enumerate() {
            let max_len = rows
                .iter()
                .filter(|r| i < r.len())
                .map(|r| r[i].len())
                .max()
                .unwrap_or(header.len());
            col_widths.push(std::cmp::max(header.len(), max_len));
        }

        let mut output = String::new();

        output.push_str(&format!(
            "{}",
            "┌".to_string()
                + &col_widths
                    .iter()
                    .map(|w| "─".repeat(*w + 2))
                    .collect::<Vec<_>>()
                    .join("┬")
                + "┐\n"
        ));

        output.push_str(&format!(
            "│{}│\n",
            headers
                .iter()
                .enumerate()
                .map(|(i, h)| format!(" {:<width$} ", h, width = col_widths[i]))
                .collect::<Vec<_>>()
                .join("│")
        ));

        output.push_str(&format!(
            "{}",
            "├".to_string()
                + &col_widths
                    .iter()
                    .map(|w| "─".repeat(*w + 2))
                    .collect::<Vec<_>>()
                    .join("┼")
                + "┤\n"
        ));

        for row in rows {
            output.push_str(&format!(
                "│{}│\n",
                row.iter()
                    .enumerate()
                    .map(|(i, v)| format!(" {:<width$} ", v, width = col_widths[i]))
                    .collect::<Vec<_>>()
                    .join("│")
            ));
        }

        output.push_str(&format!(
            "{}",
            "└".to_string()
                + &col_widths
                    .iter()
                    .map(|w| "─".repeat(*w + 2))
                    .collect::<Vec<_>>()
                    .join("┴")
                + "┘\n"
        ));

        println!("{}", output.trim_end());

        if let Ok(mut guard) = self.file.lock() {
            if let Some(ref mut file) = *guard {
                let _ = file.write_all(output.as_bytes());
                let _ = file.flush();
            }
        }
    }
}

impl Drop for TestLogger {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.file.lock() {
            if let Some(ref mut file) = *guard {
                let _ = file.flush();
            }
        }
    }
}
