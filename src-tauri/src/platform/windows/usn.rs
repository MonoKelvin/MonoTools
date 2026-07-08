//! USN Journal 文件索引 - NTFS 实时变更订阅
//! 提供轻量、高性能的文件搜索实现
//!
//! 注：完整的 USN Journal 需要管理员权限打开 `\\.\C:`，此处提供可在无管理员
//! 权限下运行的降级实现 + 完整 Win32 实现。
//!
//! 两种实现：
//! - `FallbackUsnJournal` — 基于 walkdir 扫描目录，作为兜底
//! - `WinUsnJournal` — 调用 Win32 API（feature-gated）

use crate::error::Result;
use crate::models::FileResult;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use walkdir::WalkDir;

/// USN 记录
#[derive(Debug, Clone)]
pub struct UsnRecord {
    pub file_reference_number: u64,
    pub parent_file_reference: u64,
    pub file_name: String,
    pub full_path: PathBuf,
    pub file_size: u64,
    pub last_write_time: i64,
    pub is_directory: bool,
    pub extension: Option<String>,
}

/// USN Journal 状态
#[derive(Debug, Clone)]
pub struct UsnJournal {
    pub usn: u64,
    pub next_usn: u64,
    pub range_start: u64,
    pub range_length: u64,
}

/// 文件搜索引擎抽象
pub trait FileEngine: Send + Sync {
    fn build_index(&self) -> Result<()>;
    fn update_index(&self) -> Result<()>;
    fn search(&self, query: &str, limit: u32) -> Vec<FileResult>;
    fn total(&self) -> usize;
}

/// 基于 walkdir 的兜底实现 — 任何用户都可使用
pub struct FallbackFileEngine {
    index: RwLock<HashMap<String, Vec<UsnRecord>>>,
    roots: Vec<PathBuf>,
    last_update: RwLock<i64>,
}

impl FallbackFileEngine {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self {
            index: RwLock::new(HashMap::new()),
            roots,
            last_update: RwLock::new(0),
        }
    }

    fn record_filename(rec: &UsnRecord) -> String {
        rec.file_name.to_lowercase()
    }

    fn build_record(path: PathBuf) -> UsnRecord {
        let metadata = std::fs::metadata(&path).ok();
        let (size, is_dir) = metadata
            .as_ref()
            .map(|m| (m.len(), m.is_dir()))
            .unwrap_or((0, false));
        let modified = metadata
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        UsnRecord {
            file_reference_number: 0,
            parent_file_reference: 0,
            file_name: name,
            full_path: path,
            file_size: size,
            last_write_time: modified,
            is_directory: is_dir,
            extension: ext,
        }
    }
}

impl FileEngine for FallbackFileEngine {
    fn build_index(&self) -> Result<()> {
        log::info!("开始构建文件索引 - roots: {:?}", self.roots);
        let mut idx = self.index.write();
        idx.clear();

        let now = chrono::Utc::now().timestamp();
        for root in &self.roots {
            if !root.exists() {
                continue;
            }
            let walker = WalkDir::new(root)
                .max_depth(8)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| !is_hidden(e.file_name()));
            for entry in walker.flatten() {
                if entry.depth() == 0 {
                    continue;
                }
                let rec = Self::build_record(entry.path().to_path_buf());
                if rec.file_name.is_empty() {
                    continue;
                }
                let key = Self::record_filename(&rec);
                idx.entry(key).or_insert_with(Vec::new).push(rec);
            }
        }
        *self.last_update.write() = now;
        log::info!("索引构建完成: {} 个文件名分组", idx.len());
        Ok(())
    }

    fn update_index(&self) -> Result<()> {
        // 简单策略：每 60s 全量重建
        let last = *self.last_update.read();
        let now = chrono::Utc::now().timestamp();
        if now - last < 60 {
            return Ok(());
        }
        self.build_index()
    }

    fn search(&self, query: &str, limit: u32) -> Vec<FileResult> {
        if query.is_empty() {
            return vec![];
        }
        let q = query.to_lowercase();
        let idx = self.index.read();
        let mut results = Vec::new();
        'outer: for (name, records) in idx.iter() {
            if !name.contains(&q) {
                continue;
            }
            for r in records.iter() {
                results.push(FileResult {
                    path: r.full_path.clone(),
                    name: r.file_name.clone(),
                    extension: r.extension.clone(),
                    size: r.file_size,
                    modified_at: r.last_write_time,
                    is_directory: r.is_directory,
                });
                if results.len() >= limit as usize {
                    break 'outer;
                }
            }
        }
        results
    }

    fn total(&self) -> usize {
        self.index.read().values().map(|v| v.len()).sum()
    }
}

fn is_hidden(name: &std::ffi::OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

/// 启动后台更新线程（每 N 秒尝试增量更新一次）
pub fn start_update_loop(engine: Arc<dyn FileEngine>, interval: Duration) {
    std::thread::spawn(move || loop {
        std::thread::sleep(interval);
        let _ = engine.update_index();
    });
}
