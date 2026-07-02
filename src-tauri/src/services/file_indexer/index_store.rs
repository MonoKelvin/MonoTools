use crate::models::file_entry::FileEntry;
use anyhow::Result;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct IndexStore {
    conn: Arc<RwLock<Connection>>,
}

impl IndexStore {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(RwLock::new(conn)),
        }
    }

    /// 初始化数据库表
    pub async fn init_tables(&self) -> Result<()> {
        let conn = self.conn.read().await;

        // 为每个 ASCII 和范围创建 41 个表 (0~40)
        for i in 0..=40 {
            let sql = format!(
                r#"
                CREATE TABLE IF NOT EXISTS list{} (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL COLLATE NOCASE,
                    path TEXT NOT NULL,
                    parent_path TEXT NOT NULL,
                    ext TEXT,
                    size INTEGER DEFAULT 0,
                    created_at INTEGER DEFAULT 0,
                    modified_at INTEGER DEFAULT 0,
                    accessed_at INTEGER DEFAULT 0,
                    is_dir INTEGER DEFAULT 0,
                    ascii_sum INTEGER DEFAULT 0,
                    pinyin TEXT,
                    priority INTEGER DEFAULT 0,
                    volume TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_list{}_name ON list{}(name);
                CREATE INDEX IF NOT EXISTS idx_list{}_pinyin ON list{}(pinyin);
                CREATE INDEX IF NOT EXISTS idx_list{}_parent ON list{}(parent_path);
                CREATE INDEX IF NOT EXISTS idx_list{}_modified ON list{}(modified_at);
                "#,
                i, i, i, i, i, i, i, i, i
            );

            conn.execute_batch(&sql)?;
        }

        // 应用表
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS apps (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL COLLATE NOCASE,
                exe_path TEXT NOT NULL,
                icon_path TEXT,
                description TEXT,
                keywords TEXT,
                pinyin TEXT,
                launch_count INTEGER DEFAULT 0,
                last_accessed INTEGER,
                source TEXT,
                created_at INTEGER DEFAULT (strftime('%s', 'now'))
            );
            CREATE INDEX IF NOT EXISTS idx_apps_name ON apps(name);
            CREATE INDEX IF NOT EXISTS idx_apps_pinyin ON apps(pinyin);
            "#,
            [],
        )?;

        // 搜索历史表
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS search_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query TEXT NOT NULL,
                result_type TEXT,
                result_id TEXT,
                selected_at INTEGER DEFAULT (strftime('%s', 'now')),
                execution_time_ms INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_history_query ON search_history(query);
            CREATE INDEX IF NOT EXISTS idx_history_time ON search_history(selected_at);
            "#,
            [],
        )?;

        // 元数据表
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT,
                updated_at INTEGER DEFAULT (strftime('%s', 'now'))
            );
            "#,
            [],
        )?;

        // 初始化元数据
        conn.execute(
            "INSERT OR IGNORE INTO meta (key, value) VALUES ('index_version', '1')",
            [],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO meta (key, value) VALUES ('last_full_index', '0')",
            [],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO meta (key, value) VALUES ('total_indexed_files', '0')",
            [],
        )?;

        Ok(())
    }

    /// 插入文件记录
    pub async fn insert_file(&self, file: &FileEntry) -> Result<()> {
        let conn = self.conn.read().await;
        let table_id = (file.ascii_sum / 100).min(40);

        conn.execute(
            &format!(
                r#"
                INSERT INTO list{} (name, path, parent_path, ext, size, created_at, modified_at, accessed_at, is_dir, ascii_sum, pinyin, priority, volume)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                "#,
                table_id
            ),
            [
                &file.name,
                &file.path,
                &file.parent_path,
                file.ext.as_deref().unwrap_or(""),
                &file.size.to_string(),
                &file.created_at.to_string(),
                &file.modified_at.to_string(),
                &file.accessed_at.to_string(),
                &file.is_dir.to_string(),
                &file.ascii_sum.to_string(),
                file.pinyin.as_deref().unwrap_or(""),
                &file.priority.to_string(),
                &file.volume,
            ],
        )?;

        Ok(())
    }

    /// 批量插入文件
    pub async fn insert_files_batch(&self, files: &[FileEntry]) -> Result<()> {
        let mut conn = self.conn.write().await;
        let tx = conn.transaction()?;

        for file in files {
            let table_id = (file.ascii_sum / 100).min(40);

            tx.execute(
                &format!(
                    r#"
                    INSERT INTO list{} (name, path, parent_path, ext, size, created_at, modified_at, accessed_at, is_dir, ascii_sum, pinyin, priority, volume)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                    "#,
                    table_id
                ),
                [
                    &file.name,
                    &file.path,
                    &file.parent_path,
                    file.ext.as_deref().unwrap_or(""),
                    &file.size.to_string(),
                    &file.created_at.to_string(),
                    &file.modified_at.to_string(),
                    &file.accessed_at.to_string(),
                    &file.is_dir.to_string(),
                    &file.ascii_sum.to_string(),
                    file.pinyin.as_deref().unwrap_or(""),
                    &file.priority.to_string(),
                    &file.volume,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// 搜索文件
    pub async fn search_files(&self, query: &str, limit: usize) -> Result<Vec<FileEntry>> {
        let conn = self.conn.read().await;
        let ascii_sum = crate::utils::calc_ascii_sum(query);
        let table_id = (ascii_sum / 100).min(40);
        let table_name = format!("list{}", table_id);

        let mut results = Vec::new();

        // 1. 精确匹配
        let mut stmt = conn.prepare(&format!(
            "SELECT * FROM {} WHERE name = ?1 LIMIT ?2",
            table_name
        ))?;
        let rows = stmt.query_map([query, &limit.to_string()], Self::row_to_file)?;
        for row in rows {
            results.push(row?);
        }

        if results.len() >= limit {
            return Ok(results);
        }

        // 2. 前缀匹配
        let mut stmt = conn.prepare(&format!(
            "SELECT * FROM {} WHERE name LIKE ?1 LIMIT ?2",
            table_name
        ))?;
        let pattern = format!("{}%", query);
        let rows = stmt.query_map([&pattern, &limit.to_string()], Self::row_to_file)?;
        for row in rows {
            if results.len() >= limit {
                break;
            }
            results.push(row?);
        }

        // TODO: 3. 包含匹配（跨所有表）
        // TODO: 4. 拼音匹配

        Ok(results)
    }

    /// 将数据库行转换为 FileEntry
    fn row_to_file(row: &rusqlite::Row) -> Result<FileEntry, rusqlite::Error> {
        Ok(FileEntry {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            parent_path: row.get(3)?,
            ext: row.get(4)?,
            size: row.get(5)?,
            created_at: row.get(6)?,
            modified_at: row.get(7)?,
            accessed_at: row.get(8)?,
            is_dir: row.get(9)?,
            ascii_sum: row.get(10)?,
            pinyin: row.get(11)?,
            priority: row.get(12)?,
            volume: row.get(13)?,
        })
    }

    /// 更新元数据
    pub async fn update_meta(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.read().await;
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value, updated_at) VALUES (?1, ?2, strftime('%s', 'now'))",
            [key, value],
        )?;
        Ok(())
    }
}
