//! 文件搜索引擎 - 完全基于 NTFS MFT 和 USN Journal（类似 Everything）
//!
//! 架构设计：
//! - 全量索引：使用 FSCTL_ENUM_USN_DATA 批量读取 MFT，这是最快的全盘枚举方式
//! - 增量更新：使用 FSCTL_READ_USN_JOURNAL 读取变更记录
//! - 索引层：SQLite FTS5 虚拟表，实现毫秒级全文搜索
//! - 缓存层：路径缓存，通过父文件引用号快速重建完整路径
//!
//! 参考 Everything 实现原理：
//! 1. 直接读取 NTFS MFT，不进行递归目录遍历
//! 2. 利用 USN Journal 实现实时增量更新
//! 3. 使用前缀索引优化搜索性能

use crate::error::Result;
use crate::models::{FileResult, ResultType, SearchAction, SearchCategory, SearchResult};
use crate::platform::windows::usn::{NtfsIndexer, UsnChangeReason, UsnRecord};
use parking_lot::Mutex;
use rusqlite::{Connection, OpenFlags};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

const DB_NAME: &str = "monotools_file_index.db";

pub struct FileSearchEngine {
    db: Arc<Mutex<Connection>>,
    ntfs_indexer: Option<NtfsIndexer>,
    last_update: Mutex<i64>,
    indexed_paths: Arc<Mutex<HashSet<String>>>,
    is_indexing: Mutex<bool>,
    drives: Vec<char>,
}

impl FileSearchEngine {
    pub fn new(roots: Vec<PathBuf>) -> Result<Self> {
        let db_path = get_db_path();
        Self::new_with_db_path_and_roots(db_path, roots)
    }

    pub fn new_with_db_path(db_path: PathBuf) -> Result<Self> {
        Self::new_with_db_path_and_roots(db_path, Vec::new())
    }

    pub fn new_with_db_path_and_roots(db_path: PathBuf, roots: Vec<PathBuf>) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open_with_flags(
            &db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE |
            OpenFlags::SQLITE_OPEN_CREATE |
            OpenFlags::SQLITE_OPEN_FULL_MUTEX,
        )?;

        Self::init_db(&conn)?;

        let drives = extract_drives_from_roots(&roots);
        log::info!("文件搜索引擎配置的盘符: {:?}", drives);

        let ntfs_indexer = if drives.is_empty() {
            match NtfsIndexer::new() {
                Ok(i) => {
                    log::info!("NTFS索引器创建成功，检测到 {} 个卷", i.get_volumes().len());
                    Some(i)
                }
                Err(e) => {
                    log::warn!("NTFS索引器创建失败: {}", e);
                    None
                }
            }
        } else {
            match NtfsIndexer::new_with_drives(drives.clone()) {
                Ok(i) => {
                    log::info!("NTFS索引器创建成功（指定盘符），检测到 {} 个卷", i.get_volumes().len());
                    Some(i)
                }
                Err(e) => {
                    log::warn!("NTFS索引器创建失败: {}", e);
                    None
                }
            }
        };

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            ntfs_indexer,
            last_update: Mutex::new(0),
            indexed_paths: Arc::new(Mutex::new(HashSet::new())),
            is_indexing: Mutex::new(false),
            drives,
        })
    }

    fn init_db(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA cache_size=500000;
            PRAGMA temp_store=MEMORY;
            PRAGMA page_size=65536;
            PRAGMA mmap_size=268435456;

            CREATE TABLE IF NOT EXISTS files_meta (
                id        INTEGER PRIMARY KEY AUTOINCREMENT,
                path      TEXT NOT NULL UNIQUE,
                name      TEXT NOT NULL,
                ext       TEXT,
                size      INTEGER DEFAULT 0,
                modified  INTEGER DEFAULT 0,
                is_dir    INTEGER DEFAULT 0,
                depth     INTEGER DEFAULT 0
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
                name,
                path,
                content='files_meta',
                content_rowid='id',
                tokenize='unicode61',
                prefix='2 3 4'
            );
            CREATE TRIGGER IF NOT EXISTS files_ai AFTER INSERT ON files_meta BEGIN
                INSERT INTO files_fts(rowid, name, path) VALUES (new.id, new.name, new.path);
            END;
            CREATE TRIGGER IF NOT EXISTS files_ad AFTER DELETE ON files_meta BEGIN
                INSERT INTO files_fts(files_fts, rowid, name, path) VALUES('delete', old.id, old.name, old.path);
            END;
            CREATE TRIGGER IF NOT EXISTS files_au AFTER UPDATE ON files_meta BEGIN
                INSERT INTO files_fts(files_fts, rowid, name, path) VALUES('delete', old.id, old.name, old.path);
                INSERT INTO files_fts(rowid, name, path) VALUES (new.id, new.name, new.path);
            END;
            CREATE INDEX IF NOT EXISTS idx_files_meta_path ON files_meta(path);
            CREATE INDEX IF NOT EXISTS idx_files_meta_modified ON files_meta(modified);
            CREATE INDEX IF NOT EXISTS idx_files_meta_is_dir ON files_meta(is_dir);
            CREATE INDEX IF NOT EXISTS idx_files_meta_ext ON files_meta(ext);
            "#,
        )?;
        Ok(())
    }

    pub async fn build_index(&self) -> Result<()> {
        if *self.is_indexing.lock() {
            log::info!("索引构建已在进行中");
            return Ok(());
        }
        *self.is_indexing.lock() = true;

        log::info!("开始构建文件索引（基于NTFS MFT）");
        let now = std::time::Instant::now();

        let total = self.build_index_ntfs().await?;

        log::info!("文件索引构建完成: {} 个文件，耗时 {:?}", total, now.elapsed());
        *self.is_indexing.lock() = false;
        Ok(())
    }

    async fn build_index_ntfs(&self) -> Result<usize> {
        {
            let conn = self.db.lock();
            conn.execute("DELETE FROM files_meta", [])?;
        }

        let mut records = Vec::new();

        if let Some(indexer) = &self.ntfs_indexer {
            let volumes = indexer.get_volumes();
            log::info!("NTFS索引器可用，检测到 {} 个卷: {:?}", volumes.len(), volumes);

            for volume in volumes {
                log::info!("开始枚举卷: {}", volume);
                let count = self.enumerate_volume(indexer, volume, &mut records);
                log::info!("卷 {} 枚举完成: {} 个文件，当前累计: {}", volume, count, records.len());
            }
        } else {
            log::warn!("NTFS索引器不可用，无法构建索引");
            return Ok(0);
        }

        log::info!("开始批量写入数据库，共 {} 条记录", records.len());
        let mut conn = self.db.lock();
        self.batch_insert_records(&mut conn, &records)?;

        *self.last_update.lock() = chrono::Utc::now().timestamp();

        Ok(records.len())
    }

    fn enumerate_volume(&self, indexer: &NtfsIndexer, volume: &str, records: &mut Vec<UsnRecord>) -> usize {
        let mut count = 0;
        let mut skipped = 0;

        let result = indexer.enumerate_volume_files(volume, |record| {
            if self.should_skip_path(&record) {
                skipped += 1;
                return;
            }

            records.push(record);
            count += 1;
        });

        if let Err(e) = result {
            log::warn!("枚举卷 {} 失败: {}", volume, e);
        }

        log::info!("卷 {} 枚举完成，有效文件: {}, 跳过: {}", volume, count, skipped);
        count
    }

    fn should_skip_path(&self, record: &UsnRecord) -> bool {
        let name = record.file_name.to_lowercase();

        if name.starts_with('.') && !name.starts_with(".git") && !name.starts_with(".vscode") {
            return true;
        }

        if name.starts_with('$') {
            return true;
        }

        if name == "thumbs.db" || name == "desktop.ini" || name == "pagefile.sys" || name == "hiberfil.sys" {
            return true;
        }

        let path_str = record.full_path.to_string_lossy().to_lowercase();
        if path_str.contains("\\windows\\winsxs") ||
           path_str.contains("\\windows\\system32\\config") ||
           path_str.contains("\\windows\\softwaredistribution") {
            return true;
        }

        false
    }

    fn batch_insert_records(&self, conn: &mut Connection, records: &[UsnRecord]) -> Result<()> {
        conn.execute("PRAGMA synchronous=OFF", [])?;
        conn.execute("BEGIN TRANSACTION", [])?;

        conn.execute("DROP TRIGGER IF EXISTS files_ai", [])?;
        conn.execute("DROP TRIGGER IF EXISTS files_ad", [])?;
        conn.execute("DROP TRIGGER IF EXISTS files_au", [])?;

        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO files_meta(path, name, ext, size, modified, is_dir, depth)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;

        let mut inserted = 0;
        let batch_size = 50000;

        for record in records {
            let path_str = record.full_path.to_string_lossy().to_string();
            let depth = record.full_path.components().count() as i64;

            let _ = stmt.execute(rusqlite::params![
                path_str,
                record.file_name,
                record.extension.as_deref().unwrap_or(""),
                record.file_size as i64,
                record.last_write_time,
                if record.is_directory { 1 } else { 0 },
                depth,
            ]);

            inserted += 1;

            if inserted % batch_size == 0 {
                log::debug!("已插入 {} 条记录", inserted);
            }
        }

        drop(stmt);

        conn.execute_batch(
            r#"
            CREATE TRIGGER files_ai AFTER INSERT ON files_meta BEGIN
                INSERT INTO files_fts(rowid, name, path) VALUES (new.id, new.name, new.path);
            END;
            CREATE TRIGGER files_ad AFTER DELETE ON files_meta BEGIN
                INSERT INTO files_fts(files_fts, rowid, name, path) VALUES('delete', old.id, old.name, old.path);
            END;
            CREATE TRIGGER files_au AFTER UPDATE ON files_meta BEGIN
                INSERT INTO files_fts(files_fts, rowid, name, path) VALUES('delete', old.id, old.name, old.path);
                INSERT INTO files_fts(rowid, name, path) VALUES (new.id, new.name, new.path);
            END;
            INSERT INTO files_fts(files_fts) VALUES('rebuild');
            COMMIT;
            PRAGMA synchronous=NORMAL;
            "#,
        )?;

        log::info!("批量插入完成，共插入 {} 条记录", inserted);
        Ok(())
    }

    pub fn update_index(&self) -> Result<()> {
        if let Some(indexer) = &self.ntfs_indexer {
            return self.update_index_usn(indexer);
        }
        Ok(())
    }

    fn update_index_usn(&self, indexer: &NtfsIndexer) -> Result<()> {
        let changes = indexer.get_all_changes()?;
        if changes.is_empty() {
            return Ok(());
        }

        log::debug!("USN Journal 检测到 {} 个变化", changes.len());

        let conn = self.db.lock();
        let mut paths = self.indexed_paths.lock();

        for change in changes {
            let path_str = change.full_path.to_string_lossy().to_string();

            if self.should_skip_path(&change) {
                continue;
            }

            let depth = change.full_path.components().count() as i64;
            let ext = change.extension.as_deref().unwrap_or("");

            match change.reason {
                UsnChangeReason::Created | UsnChangeReason::RenamedNewName => {
                    if !paths.contains(&path_str) {
                        if !change.file_name.is_empty() {
                            conn.execute(
                                "INSERT OR REPLACE INTO files_meta(path, name, ext, size, modified, is_dir, depth)
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                                rusqlite::params![
                                    path_str,
                                    change.file_name,
                                    ext,
                                    change.file_size as i64,
                                    change.last_write_time,
                                    if change.is_directory { 1 } else { 0 },
                                    depth,
                                ],
                            )?;
                            paths.insert(path_str);
                        }
                    }
                }
                UsnChangeReason::Deleted | UsnChangeReason::RenamedOldName => {
                    conn.execute("DELETE FROM files_meta WHERE path = ?1", rusqlite::params![path_str])?;
                    paths.remove(&path_str);
                }
                UsnChangeReason::Modified => {
                    if paths.contains(&path_str) {
                        if !change.file_name.is_empty() {
                            conn.execute(
                                "UPDATE files_meta SET name = ?2, ext = ?3, size = ?4, modified = ?5, is_dir = ?6
                                 WHERE path = ?1",
                                rusqlite::params![
                                    path_str,
                                    change.file_name,
                                    ext,
                                    change.file_size as i64,
                                    change.last_write_time,
                                    if change.is_directory { 1 } else { 0 },
                                ],
                            )?;
                        }
                    }
                }
            }
        }

        *self.last_update.lock() = chrono::Utc::now().timestamp();
        Ok(())
    }

    pub fn search(&self, query: &str, limit: u32) -> Vec<SearchResult> {
        if query.is_empty() {
            return self.recent_files(limit)
                .into_iter()
                .map(|f| file_result_to_search_result(f))
                .collect();
        }

        let fts_query = build_fts_query(query);
        let conn = self.db.lock();
        let mut results = Vec::new();

        let sql = r#"
            SELECT m.path, m.name, m.ext, m.size, m.modified, m.is_dir
            FROM files_fts f
            JOIN files_meta m ON m.id = f.rowid
            WHERE files_fts MATCH ?1
            ORDER BY f.rank, m.is_dir DESC, m.modified DESC
            LIMIT ?2
        "#;

        if let Ok(mut stmt) = conn.prepare(sql) {
            if let Ok(iter) = stmt.query_map(rusqlite::params![fts_query, limit], |row| {
                Ok(FileResult {
                    path: PathBuf::from(row.get::<_, String>(0)?),
                    name: row.get(1)?,
                    extension: row.get(2)?,
                    size: row.get(3)?,
                    modified_at: row.get(4)?,
                    is_directory: row.get::<_, i32>(5)? != 0,
                })
            }) {
                results = iter.filter_map(|x| x.ok()).collect();
            }
        }

        results.into_iter().map(|f| file_result_to_search_result(f)).collect()
    }

    pub fn search_with_score(&self, query: &str, limit: u32) -> Vec<(f32, FileResult)> {
        if query.is_empty() {
            return self.recent_files(limit)
                .into_iter()
                .map(|f| (0.0, f))
                .collect();
        }

        let fts_query = build_fts_query(query);
        let conn = self.db.lock();
        let mut results = Vec::new();

        let sql = r#"
            SELECT m.path, m.name, m.ext, m.size, m.modified, m.is_dir, f.rank
            FROM files_fts f
            JOIN files_meta m ON m.id = f.rowid
            WHERE files_fts MATCH ?1
            ORDER BY f.rank, m.is_dir DESC, m.modified DESC
            LIMIT ?2
        "#;

        if let Ok(mut stmt) = conn.prepare(sql) {
            if let Ok(iter) = stmt.query_map(rusqlite::params![fts_query, limit], |row| {
                let rank: f32 = row.get(6)?;
                Ok((
                    rank,
                    FileResult {
                        path: PathBuf::from(row.get::<_, String>(0)?),
                        name: row.get(1)?,
                        extension: row.get(2)?,
                        size: row.get(3)?,
                        modified_at: row.get(4)?,
                        is_directory: row.get::<_, i32>(5)? != 0,
                    },
                ))
            }) {
                results = iter.filter_map(|x| x.ok()).collect();
            }
        }

        results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    fn recent_files(&self, limit: u32) -> Vec<FileResult> {
        let conn = self.db.lock();
        let mut results = Vec::new();

        let sql = r#"
            SELECT path, name, ext, size, modified, is_dir
            FROM files_meta
            ORDER BY modified DESC, is_dir DESC
            LIMIT ?1
        "#;

        if let Ok(mut stmt) = conn.prepare(sql) {
            if let Ok(iter) = stmt.query_map(rusqlite::params![limit], |row| {
                Ok(FileResult {
                    path: PathBuf::from(row.get::<_, String>(0)?),
                    name: row.get(1)?,
                    extension: row.get(2)?,
                    size: row.get(3)?,
                    modified_at: row.get(4)?,
                    is_directory: row.get::<_, i32>(5)? != 0,
                })
            }) {
                results = iter.filter_map(|x| x.ok()).collect();
            }
        }

        results
    }

    pub fn total(&self) -> usize {
        let conn = self.db.lock();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM files_meta", [], |row| row.get(0))
            .unwrap_or(0);
        count as usize
    }

    pub fn is_indexing(&self) -> bool {
        *self.is_indexing.lock()
    }

    pub fn get_ntfs_indexer(&self) -> Option<&NtfsIndexer> {
        self.ntfs_indexer.as_ref()
    }

    pub fn get_drives(&self) -> &[char] {
        &self.drives
    }
}

fn get_db_path() -> PathBuf {
    if let Ok(app_data) = std::env::var("APPDATA") {
        PathBuf::from(app_data).join("MonoTools").join(DB_NAME)
    } else {
        PathBuf::from(DB_NAME)
    }
}

fn extract_drives_from_roots(roots: &[PathBuf]) -> Vec<char> {
    let mut drives = HashSet::new();
    
    for root in roots {
        if let Some(drive) = root.as_os_str().to_str().and_then(|s| s.chars().next()) {
            if drive.is_ascii_alphabetic() {
                drives.insert(drive.to_ascii_uppercase());
            }
        }
    }
    
    drives.into_iter().collect()
}

fn build_fts_query(query: &str) -> String {
    let terms: Vec<String> = query
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| {
            let escaped = t
                .chars()
                .map(|c| match c {
                    '"' | '\'' | '\\' | '^' | '$' | '@' | '~' | '*' | '(' | ')' => format!("\\{}", c),
                    _ => c.to_string(),
                })
                .collect::<String>();
            format!("{}*", escaped)
        })
        .collect();

    if terms.is_empty() {
        return "*".to_string();
    }

    terms.join(" ")
}

impl Default for FileSearchEngine {
    fn default() -> Self {
        Self::new(Vec::new()).unwrap()
    }
}

fn file_result_to_search_result(f: FileResult) -> SearchResult {
    let path_str = f.path.to_string_lossy().to_string();
    let result_type = file_type_of(&f);
    let subtitle = if f.is_directory {
        path_str.clone()
    } else {
        format!("{} ({})", path_str, format_size(f.size))
    };

    SearchResult {
        id: path_str.clone(),
        title: f.name,
        subtitle,
        icon: f.extension.clone().map(|ext| format!("file:///{}.ico", ext)),
        category: SearchCategory::Files,
        result_type,
        action: if f.is_directory {
            SearchAction::Open(path_str)
        } else {
            SearchAction::Open(path_str)
        },
        score: 0.0,
    }
}

fn file_type_of(f: &FileResult) -> ResultType {
    if f.is_directory {
        return ResultType::Directory;
    }

    let ext = f.extension.as_deref().unwrap_or("").to_lowercase();

    match ext.as_str() {
        "exe" | "dll" | "msi" | "bat" | "cmd" | "ps1" => ResultType::Executable,
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "ico" | "svg" | "webp" => ResultType::Image,
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => ResultType::Video,
        "mp3" | "wav" | "flac" | "ogg" | "aac" | "wma" => ResultType::Audio,
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => ResultType::Archive,
        "txt" | "md" | "doc" | "docx" | "pdf" | "xls" | "xlsx" | "ppt" | "pptx" | "csv" | "json" | "xml" => ResultType::Document,
        _ => ResultType::OtherFile,
    }
}

fn format_size(bytes: i64) -> String {
    let bytes = bytes as f64;
    if bytes < 1024.0 {
        format!("{} B", bytes)
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.1} KB", bytes / 1024.0)
    } else if bytes < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} MB", bytes / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn start_update_loop<F>(mut update_fn: F, interval: std::time::Duration)
where
    F: Send + 'static + FnMut() -> Result<()>,
{
    std::thread::spawn(move || loop {
        std::thread::sleep(interval);
        let _ = update_fn();
    });
}
