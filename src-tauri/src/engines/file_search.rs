//! 文件搜索引擎 - 整合 NTFS MFT 枚举和 SQLite FTS5
//!
//! 架构设计：
//! - 全量索引：使用 FSCTL_ENUM_USN_DATA 批量读取 MFT（类似 Everything）
//! - 增量更新：使用 FSCTL_READ_USN_JOURNAL 读取变更记录
//! - 索引层：SQLite FTS5 虚拟表，实现毫秒级全文搜索
//! - 缓存层：路径缓存，通过父文件引用号快速重建完整路径

use crate::error::Result;
use crate::models::{FileResult, ResultType, SearchAction, SearchCategory, SearchResult};
use crate::platform::windows::usn::{NtfsIndexer, WinUsnJournal, UsnChangeReason, UsnRecord};
use parking_lot::Mutex;
use rusqlite::{Connection, OpenFlags};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const DB_NAME: &str = "monotools_file_index.db";

#[derive(Debug, Clone)]
struct IndexRecord {
    path: PathBuf,
    name: String,
    extension: Option<String>,
    size: i64,
    modified_at: i64,
    is_directory: bool,
}

pub struct FileSearchEngine {
    db: Arc<Mutex<Connection>>,
    roots: Vec<PathBuf>,
    ntfs_indexer: Option<NtfsIndexer>,
    usn_journal: Option<WinUsnJournal>,
    last_update: Mutex<i64>,
    indexed_paths: Arc<Mutex<HashSet<String>>>,
}

impl FileSearchEngine {
    pub fn new(roots: Vec<PathBuf>) -> Result<Self> {
        let db_path = get_db_path();
        Self::new_with_db_path(roots, db_path)
    }

    pub fn new_with_db_path(roots: Vec<PathBuf>, db_path: PathBuf) -> Result<Self> {
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

        let ntfs_indexer = NtfsIndexer::new().ok();
        let usn_journal = WinUsnJournal::new().ok();

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            roots,
            ntfs_indexer,
            usn_journal,
            last_update: Mutex::new(0),
            indexed_paths: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    fn init_db(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA cache_size=100000;
            PRAGMA temp_store=MEMORY;

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
            CREATE INDEX IF NOT EXISTS idx_files_meta_depth ON files_meta(depth);
            "#,
        )?;
        Ok(())
    }

    pub async fn build_index(&self) -> Result<()> {
        log::info!("开始构建文件索引 - roots: {:?}", self.roots);
        let now = std::time::Instant::now();

        let total = self.build_index_sync().await?;

        log::info!("文件索引构建完成: {} 个文件，耗时 {:?}", total, now.elapsed());
        Ok(())
    }

    async fn build_index_sync(&self) -> Result<usize> {
        let mut conn = self.db.lock();
        conn.execute("DELETE FROM files_meta", [])?;

        let mut total = 0;
        let mut paths = HashSet::new();

        if let Some(indexer) = &self.ntfs_indexer {
            total = self.build_index_ntfs(indexer, &mut conn, &mut paths);
            log::info!("NTFS MFT 索引完成: {} 个文件", total);
        }

        let walkdir_total = self.build_index_walkdir(&mut conn, &mut paths);
        if walkdir_total > 0 {
            log::info!("Walkdir 索引完成: {} 个文件", walkdir_total);
            total += walkdir_total;
        }

        conn.execute_batch("VACUUM")?;

        *self.indexed_paths.lock() = paths;
        *self.last_update.lock() = chrono::Utc::now().timestamp();

        Ok(total)
    }

    fn build_index_ntfs(
        &self,
        indexer: &NtfsIndexer,
        conn: &mut Connection,
        paths: &mut HashSet<String>,
    ) -> usize {
        let mut total = 0;

        for volume in indexer.get_volumes() {
            let result = indexer.enumerate_volume_files(volume, |record| {
                let path_str = record.full_path.to_string_lossy().to_string();

                if paths.contains(&path_str) {
                    return;
                }

                paths.insert(path_str.clone());
                total += 1;
            });

            if let Err(e) = result {
                log::warn!("枚举卷 {} 失败: {}", volume, e);
            }
        }

        let _ = self.insert_paths_ntfs(conn, paths);

        total
    }

    fn insert_paths_ntfs(&self, conn: &mut Connection, paths: &HashSet<String>) -> Result<()> {
        let batch_size = 5000;
        let mut batch = Vec::new();

        let tx = conn.transaction()?;
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO files_meta(path, name, ext, size, modified, is_dir, depth)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;

        for path_str in paths {
            let path = PathBuf::from(path_str);
            let record = build_record(path, 0);
            if record.name.is_empty() {
                continue;
            }

            batch.push((
                path_str.clone(),
                record.name,
                record.extension.unwrap_or_default(),
                record.size,
                record.modified_at,
                if record.is_directory { 1 } else { 0 },
                0,
            ));

            if batch.len() >= batch_size {
                for (path, name, ext, size, modified, is_dir, depth) in &batch {
                    let _ = stmt.execute(rusqlite::params![path, name, ext, *size, *modified, *is_dir, *depth]);
                }
                batch.clear();
            }
        }

        if !batch.is_empty() {
            for (path, name, ext, size, modified, is_dir, depth) in &batch {
                let _ = stmt.execute(rusqlite::params![path, name, ext, *size, *modified, *is_dir, *depth]);
            }
        }

        drop(stmt);
        tx.commit()?;

        Ok(())
    }

    fn build_index_walkdir(
        &self,
        conn: &mut Connection,
        paths: &mut HashSet<String>,
    ) -> usize {
        let mut total = 0;

        for root in &self.roots {
            if !root.exists() {
                continue;
            }

            let walker = walkdir::WalkDir::new(root)
                .max_depth(8)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| !is_hidden(e.file_name()));

            for entry in walker.filter_map(|e| e.ok()) {
                if entry.depth() == 0 {
                    continue;
                }

                let path_str = entry.path().to_string_lossy().to_string();
                if paths.contains(&path_str) {
                    continue;
                }
                paths.insert(path_str.clone());
                total += 1;
            }
        }

        let _ = self.insert_paths_walkdir(conn, paths);

        total
    }

    fn insert_paths_walkdir(&self, conn: &mut Connection, paths: &HashSet<String>) -> Result<()> {
        let batch_size = 10000;
        let mut batch = Vec::new();

        let tx = conn.transaction()?;
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO files_meta(path, name, ext, size, modified, is_dir, depth)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;

        for path_str in paths {
            let path = PathBuf::from(path_str);
            let record = build_record(path, 0);
            if record.name.is_empty() {
                continue;
            }

            batch.push((
                path_str.clone(),
                record.name,
                record.extension.unwrap_or_default(),
                record.size,
                record.modified_at,
                if record.is_directory { 1 } else { 0 },
                0,
            ));

            if batch.len() >= batch_size {
                for (path, name, ext, size, modified, is_dir, depth) in &batch {
                    let _ = stmt.execute(rusqlite::params![path, name, ext, *size, *modified, *is_dir, *depth]);
                }
                batch.clear();
            }
        }

        if !batch.is_empty() {
            for (path, name, ext, size, modified, is_dir, depth) in &batch {
                let _ = stmt.execute(rusqlite::params![path, name, ext, *size, *modified, *is_dir, *depth]);
            }
        }

        drop(stmt);
        tx.commit()?;

        Ok(())
    }

    pub fn update_index(&self) -> Result<()> {
        if let Some(indexer) = &self.ntfs_indexer {
            return self.update_index_usn(indexer);
        }

        self.update_index_fallback()
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

            match change.reason {
                UsnChangeReason::Created | UsnChangeReason::RenamedNewName => {
                    if !paths.contains(&path_str) {
                        let record = build_record(change.full_path.clone(), 0);
                        if !record.name.is_empty() {
                            conn.execute(
                                "INSERT OR REPLACE INTO files_meta(path, name, ext, size, modified, is_dir, depth)
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                                rusqlite::params![
                                    path_str,
                                    record.name,
                                    record.extension.unwrap_or_default(),
                                    record.size,
                                    record.modified_at,
                                    if record.is_directory { 1 } else { 0 },
                                    0,
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
                        let record = build_record(change.full_path.clone(), 0);
                        if !record.name.is_empty() {
                            conn.execute(
                                "UPDATE files_meta SET name = ?2, ext = ?3, size = ?4, modified = ?5, is_dir = ?6
                                 WHERE path = ?1",
                                rusqlite::params![
                                    path_str,
                                    record.name,
                                    record.extension.unwrap_or_default(),
                                    record.size,
                                    record.modified_at,
                                    if record.is_directory { 1 } else { 0 },
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

    fn update_index_fallback(&self) -> Result<()> {
        let last = *self.last_update.lock();
        let now = chrono::Utc::now().timestamp();

        if now - last < 60 {
            return Ok(());
        }

        let mut conn = self.db.lock();

        let latest: Option<i64> = conn
            .query_row("SELECT MAX(modified) FROM files_meta", [], |row| row.get(0))
            .ok()
            .flatten();

        let latest_ts = latest.unwrap_or(0);

        let existing: HashSet<String> = conn
            .prepare("SELECT path FROM files_meta")?
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        let mut current_paths = HashSet::new();

        {
            let tx = conn.transaction()?;

            for root in &self.roots {
                if !root.exists() {
                    continue;
                }

                let walker = walkdir::WalkDir::new(root)
                    .max_depth(8)
                    .follow_links(false)
                    .into_iter()
                    .filter_entry(|e| !is_hidden(e.file_name()));

                for entry in walker.filter_map(|e| e.ok()) {
                    if entry.depth() == 0 {
                        continue;
                    }

                    let path_str = entry.path().to_string_lossy().to_string();
                    current_paths.insert(path_str.clone());

                    if existing.contains(&path_str) {
                        let modified = entry.metadata()
                            .ok()
                            .and_then(|m| m.modified().ok())
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);

                        if modified <= latest_ts {
                            continue;
                        }
                    }

                    let record = build_record(entry.path().to_path_buf(), entry.depth());
                    if record.name.is_empty() {
                        continue;
                    }

                    let _ = tx.execute(
                        "INSERT OR REPLACE INTO files_meta(path, name, ext, size, modified, is_dir, depth)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        rusqlite::params![
                            path_str,
                            record.name,
                            record.extension.unwrap_or_default(),
                            record.size,
                            record.modified_at,
                            if record.is_directory { 1 } else { 0 },
                            entry.depth() as i64,
                        ],
                    );
                }
            }

            let to_delete: Vec<String> = existing.difference(&current_paths).cloned().collect();
            if !to_delete.is_empty() {
                let placeholders: Vec<&str> = to_delete.iter().map(|_| "?").collect();
                let sql = format!(
                    "DELETE FROM files_meta WHERE path IN ({})",
                    placeholders.join(",")
                );
                let params: Vec<&dyn rusqlite::ToSql> = to_delete
                    .iter()
                    .map(|p| p as &dyn rusqlite::ToSql)
                    .collect();
                tx.execute(&sql, rusqlite::params_from_iter(params))?;
            }

            tx.commit()?;
        }

        *self.last_update.lock() = now;
        *self.indexed_paths.lock() = current_paths;

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

    pub fn get_roots(&self) -> &[PathBuf] {
        &self.roots
    }

    pub fn set_roots(&mut self, roots: Vec<PathBuf>) {
        self.roots = roots;
    }

    pub fn get_ntfs_indexer(&self) -> Option<&NtfsIndexer> {
        self.ntfs_indexer.as_ref()
    }
}

fn get_db_path() -> PathBuf {
    if let Ok(app_data) = std::env::var("APPDATA") {
        PathBuf::from(app_data).join("MonoTools").join(DB_NAME)
    } else {
        PathBuf::from(DB_NAME)
    }
}

fn is_hidden(name: &std::ffi::OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

fn build_record(path: PathBuf, _depth: usize) -> IndexRecord {
    let metadata = std::fs::metadata(&path).ok();
    let (size, is_dir) = metadata
        .as_ref()
        .map(|m| (m.len() as i64, m.is_dir()))
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

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());

    IndexRecord {
        path,
        name,
        extension,
        size,
        modified_at: modified,
        is_directory: is_dir,
    }
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
