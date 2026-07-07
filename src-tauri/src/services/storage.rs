//! SQLite 存储服务 - 持久化设置、命令、启动项、统计、历史
use crate::error::{AppError, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use parking_lot::Mutex;

pub struct StorageService {
    conn: Arc<Mutex<Connection>>,
    #[allow(dead_code)]
    path: PathBuf,
}

impl StorageService {
    pub fn new(app: &AppHandle) -> Result<Self> {
        let app_data = app
            .path()
            .app_data_dir()
            .map_err(|e| AppError::Other(format!("获取数据目录失败: {e}")))?;
        std::fs::create_dir_all(&app_data)?;
        let db_path = app_data.join("monotools.db");

        let conn = Connection::open(&db_path)?;
        let service = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: db_path,
        };
        service.migrate()?;
        Ok(service)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER DEFAULT (strftime('%s','now'))
            );
            CREATE TABLE IF NOT EXISTS custom_commands (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                keyword TEXT NOT NULL,
                command TEXT NOT NULL,
                args TEXT,
                working_dir TEXT,
                icon TEXT,
                category TEXT DEFAULT 'Custom',
                enabled INTEGER DEFAULT 1,
                run_as_admin INTEGER DEFAULT 0,
                created_at INTEGER,
                last_used_at INTEGER
            );
            CREATE TABLE IF NOT EXISTS startup_items (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                command TEXT NOT NULL,
                args TEXT,
                working_dir TEXT,
                enabled INTEGER DEFAULT 1,
                delay_seconds INTEGER DEFAULT 0,
                run_as_admin INTEGER DEFAULT 0,
                source TEXT NOT NULL DEFAULT 'Custom',
                created_at INTEGER
            );
            CREATE TABLE IF NOT EXISTS app_stats (
                app_path TEXT PRIMARY KEY,
                app_name TEXT,
                launch_count INTEGER DEFAULT 0,
                last_launched INTEGER
            );
            CREATE TABLE IF NOT EXISTS search_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query TEXT NOT NULL,
                result_count INTEGER DEFAULT 0,
                selected_result_id TEXT,
                timestamp INTEGER
            );
            CREATE TABLE IF NOT EXISTS theme_presets (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                mode TEXT NOT NULL,
                accent_color TEXT,
                created_at INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_custom_commands_keyword ON custom_commands(keyword);
            CREATE INDEX IF NOT EXISTS idx_search_history_ts ON search_history(timestamp);
            "#,
        )?;
        Ok(())
    }

    pub fn get_setting<T: DeserializeOwned>(
        &self,
        key: &str,
        default: T,
    ) -> Result<T> {
        let conn = self.conn.lock();
        let value: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?;
        match value {
            Some(v) => Ok(serde_json::from_str(&v)?),
            None => Ok(default),
        }
    }

    pub fn set_setting<T: Serialize>(&self, key: &str, value: T) -> Result<()> {
        let conn = self.conn.lock();
        let json = serde_json::to_string(&value)?;
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, strftime('%s','now'))
             ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
            params![key, json],
        )?;
        Ok(())
    }
}
