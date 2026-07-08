//! SQLite FTS5 文件搜索引擎
//!
//! 使用 SQLite FTS5 虚拟表实现高性能全文搜索，类似 Everything 的体验。
//! 文件元数据存储在 `files_meta` 表中，FTS5 索引在 `files_fts` 虚拟表上。
//!
//! 架构：
//! - `files_meta`: 存储文件路径、名称、扩展名、大小、修改时间等结构化数据
//! - `files_fts`: FTS5 虚拟表，使用 external content from files_meta，
//!   仅对文件名字段建立全文索引
//! - Trigger 保持 FTS 索引与元数据同步

use crate::error::{AppError, Result};
use crate::models::FileResult;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// SQLite 行结构（由 rusqlite 查询映射）
struct MetaRow {
    path: String,
    name: String,
    ext: Option<String>,
    size: u64,
    modified: i64,
    is_dir: i64,
}

/// 内部文件记录
#[derive(Debug, Clone)]
struct Record {
    path: PathBuf,
    name: String,
    extension: Option<String>,
    size: u64,
    modified_at: i64,
    is_directory: bool,
}

/// FTS5 搜索引擎实例
pub struct FileFts5Engine {
    db: Arc<Mutex<Connection>>,
    roots: Vec<PathBuf>,
    #[allow(dead_code)]
    db_path: PathBuf,
}

impl FileFts5Engine {
    /// 创建或打开数据库
    pub fn new<P: AsRef<Path>>(db_path: P, roots: Vec<PathBuf>) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&db_path)?;
        Self::migrate(&conn)?;
        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            roots,
            db_path,
        })
    }

    /// 执行数据库迁移
    fn migrate(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS files_meta (
                id        INTEGER PRIMARY KEY AUTOINCREMENT,
                path      TEXT NOT NULL UNIQUE,
                name      TEXT NOT NULL,
                ext       TEXT,
                size      INTEGER DEFAULT 0,
                modified  INTEGER DEFAULT 0,
                is_dir    INTEGER DEFAULT 0
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
                name,
                content='files_meta',
                content_rowid='id',
                tokenize='unicode61'
            );
            CREATE TRIGGER IF NOT EXISTS files_ai AFTER INSERT ON files_meta BEGIN
                INSERT INTO files_fts(rowid, name) VALUES (new.id, new.name);
            END;
            CREATE TRIGGER IF NOT EXISTS files_ad AFTER DELETE ON files_meta BEGIN
                INSERT INTO files_fts(files_fts, rowid, name) VALUES('delete', old.id, old.name);
            END;
            CREATE TRIGGER IF NOT EXISTS files_au AFTER UPDATE ON files_meta BEGIN
                INSERT INTO files_fts(files_fts, rowid, name) VALUES('delete', old.id, old.name);
                INSERT INTO files_fts(rowid, name) VALUES (new.id, new.name);
            END;
            CREATE INDEX IF NOT EXISTS idx_files_meta_modified ON files_meta(modified);
            "#,
        )?;
        Ok(())
    }

    /// 构建索引（从根目录扫描所有文件）
    pub fn build_index(&self) -> Result<()> {
        log::info!("开始构建 FTS5 文件索引 - roots: {:?}", self.roots);
        let now = std::time::Instant::now();
        let count = self.do_build_index()?;
        log::info!("FTS5 索引构建完成: {} 个文件，耗时 {:?}", count, now.elapsed());
        Ok(())
    }

    fn do_build_index(&self) -> Result<usize> {
        let mut conn = self.db.lock();
        let mut total: u64 = 0;

        conn.execute("DELETE FROM files_meta", [])?;

        for root in &self.roots {
            if !root.exists() {
                continue;
            }
            let walker = walkdir::WalkDir::new(root)
                .max_depth(8)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| !is_hidden(e.file_name()));

            let mut batch = Vec::new();
            for entry in walker.filter_map(|e| e.ok()) {
                if entry.depth() == 0 {
                    continue;
                }
                let rec = build_record(entry.path().to_path_buf());
                if rec.name.is_empty() {
                    continue;
                }
                batch.push((
                    rec.path.to_string_lossy().to_string(),
                    rec.name.clone(),
                    rec.extension.clone().unwrap_or_default(),
                    rec.size,
                    rec.modified_at,
                    if rec.is_directory { 1 } else { 0 },
                ));
                total += 1;
                if total % 5000 == 0 {
                    log::debug!("索引进度: {} 个文件...", total);
                }
            }

            if !batch.is_empty() {
                let tx = conn.transaction()?;
                {
                    let mut stmt = tx.prepare(
                        "INSERT OR REPLACE INTO files_meta(path, name, ext, size, modified, is_dir) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
                    )?;
                    for (path, name, ext, size, modified, is_dir) in &batch {
                        stmt.execute(rusqlite::params![path, name, ext, *size, *modified, *is_dir])?;
                    }
                }
                tx.commit()?;
                log::debug!("批量写入 {} 条记录", batch.len());
            }
        }

        // VACUUM 回收空间（仅在有数据时）
        if total > 0 {
            conn.execute_batch("VACUUM")?;
        }

        Ok(total as usize)
    }

    /// 增量更新索引
    pub fn update_index(&self) -> Result<()> {
        let now = std::time::Instant::now();
        let added = self.do_incremental_update()?;
        if added > 0 {
            log::info!("FTS5 增量更新完成: 新增 {} 个文件，耗时 {:?}", added, now.elapsed());
        }
        Ok(())
    }

    fn do_incremental_update(&self) -> Result<usize> {
        let conn = self.db.lock();

        // 获取数据库中最新修改时间
        let latest: Option<i64> = conn
            .query_row("SELECT MAX(modified) FROM files_meta", [], |row| row.get(0))
            .ok()
            .flatten();

        let latest_ts = latest.unwrap_or(0);
        let mut added = 0;

        // 收集现有路径用于检测删除
        let existing: HashSet<String> = conn
            .prepare("SELECT path FROM files_meta")?
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        let mut current_paths = HashSet::new();

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

                // 跳过已有的文件（除非被修改过）
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

                let rec = build_record(entry.path().to_path_buf());
                if rec.name.is_empty() {
                    continue;
                }

                conn.execute(
                    "INSERT OR REPLACE INTO files_meta(path, name, ext, size, modified, is_dir) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        path_str,
                        rec.name,
                        rec.extension.unwrap_or_default(),
                        rec.size,
                        rec.modified_at,
                        if rec.is_directory { 1 } else { 0 },
                    ],
                )?;
                added += 1;
            }
        }

        // 删除不存在的文件
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
            conn.execute(&sql, rusqlite::params_from_iter(params))?;
        }

        Ok(added)
    }

    /// 全文搜索
    pub fn search(&self, query: &str, limit: u32) -> Vec<FileResult> {
        if query.is_empty() {
            return self.recent_files(limit);
        }

        let fts_query = match parse_query(query) {
            Ok(q) => q,
            Err(_) => escape_fts5_term(query),
        };

        let conn = self.db.lock();
        let mut results = Vec::new();

        let sql = r#"
            SELECT m.path, m.name, m.ext, m.size, m.modified, m.is_dir
            FROM files_fts f
            JOIN files_meta m ON m.id = f.rowid
            WHERE files_fts MATCH ?1
            ORDER BY rank
            LIMIT ?2
        "#;

        let mut stmt = match conn.prepare(sql) {
            Ok(s) => s,
            Err(_) => return results,
        };

        let rows = stmt.query_map(rusqlite::params![fts_query, limit], |row| {
            Ok(MetaRow {
                path: row.get(0)?,
                name: row.get(1)?,
                ext: row.get(2)?,
                size: row.get(3)?,
                modified: row.get(4)?,
                is_dir: row.get(5)?,
            })
        });

        if let Ok(iter) = rows {
            for r in iter.filter_map(|x| x.ok()) {
                results.push(FileResult {
                    path: PathBuf::from(r.path),
                    name: r.name,
                    extension: r.ext,
                    size: r.size,
                    modified_at: r.modified,
                    is_directory: r.is_dir != 0,
                });
            }
        }

        results
    }

    /// 无查询时返回最近修改的文件
    fn recent_files(&self, limit: u32) -> Vec<FileResult> {
        let conn = self.db.lock();
        let mut results = Vec::new();

        let sql = r#"
            SELECT path, name, ext, size, modified, is_dir
            FROM files_meta
            ORDER BY modified DESC
            LIMIT ?1
        "#;

        if let Ok(mut stmt) = conn.prepare(sql) {
            if let Ok(iter) = stmt.query_map(rusqlite::params![limit], |row| {
                Ok(MetaRow {
                    path: row.get(0)?,
                    name: row.get(1)?,
                    ext: row.get(2)?,
                    size: row.get(3)?,
                    modified: row.get(4)?,
                    is_dir: row.get(5)?,
                })
            }) {
                for r in iter.filter_map(|x| x.ok()) {
                    results.push(FileResult {
                        path: PathBuf::from(r.path),
                        name: r.name,
                        extension: r.ext,
                        size: r.size,
                        modified_at: r.modified,
                        is_directory: r.is_dir != 0,
                    });
                }
            }
        }

        results
    }

    /// 获取索引文件总数
    pub fn total(&self) -> usize {
        let conn = self.db.lock();
        conn.query_row("SELECT COUNT(*) FROM files_meta", [], |row| row.get(0))
            .unwrap_or(0)
    }
}

fn is_hidden(name: &std::ffi::OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

fn build_record(path: PathBuf) -> Record {
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
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());

    Record {
        path,
        name,
        extension,
        size,
        modified_at: modified,
        is_directory: is_dir,
    }
}

/// 解析查询字符串为 FTS5 查询表达式
/// - 支持双引号精确短语
/// - 支持前缀搜索（末尾 *）
/// - 自动转义特殊字符
fn parse_query(query: &str) -> Result<String> {
    let mut terms: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in query.chars() {
        match ch {
            '"' => {
                if in_quotes {
                    if !current.is_empty() {
                        terms.push(format!("\"{}\"", escape_fts5_term(&current)));
                        current.clear();
                    }
                    in_quotes = false;
                } else {
                    if !current.trim().is_empty() {
                        terms.push(escape_fts5_term(current.trim()));
                        current.clear();
                    }
                    in_quotes = true;
                }
            }
            ' ' | '\t' => {
                if in_quotes {
                    current.push(ch);
                } else if !current.trim().is_empty() {
                    terms.push(escape_fts5_term(current.trim()));
                    current.clear();
                }
            }
            '*' => {
                if !current.is_empty() || in_quotes {
                    current.push(ch);
                }
            }
            _ => current.push(ch),
        }
    }

    // 处理剩余的未闭合短语
    if in_quotes && !current.is_empty() {
        terms.push(escape_fts5_term(&current));
    } else if !current.trim().is_empty() {
        terms.push(escape_fts5_term(current.trim()));
    }

    if terms.is_empty() {
        return Err(AppError::Other("empty query".into()));
    }

    Ok(terms.join(" "))
}

/// 转义 FTS5 特殊字符
fn escape_fts5_term(term: &str) -> String {
    let escaped: String = term
        .chars()
        .map(|c| match c {
            '"' => "\\\"".to_string(),
            '^' | '$' | '@' | '~' => format!("\\{}", c),
            _ => c.to_string(),
        })
        .collect();
    escaped
}

/// 启动后台索引更新线程（传入更新闭包）
pub fn start_update_loop<F>(mut update_fn: F, interval: std::time::Duration)
where
    F: Send + 'static + FnMut() -> Result<()>,
{
    std::thread::spawn(move || loop {
        std::thread::sleep(interval);
        let _ = update_fn();
    });
}
