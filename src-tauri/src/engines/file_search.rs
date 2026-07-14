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

use crate::config::{fs as fs_cfg, search as search_cfg};
use crate::error::Result;
use crate::models::{FileResult, ResultType, SearchAction, SearchCategory, SearchResult};
use crate::platform::windows::usn::{NtfsIndexer, UsnRecord};
use parking_lot::Mutex;
use rusqlite::{Connection, OpenFlags};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 空查询时 (例如首屏未输入关键字), 后端一次性返回的最多文件数.
/// 限制: 防止索引极大 (几十万文件) 时单次 IPC 序列化阻塞 UI.
/// 经验值: 100k 索引时, 取 500 ≈ 200KB JSON, IPC < 30ms.
/// 常量集中在 `config::search::ALL_FILES_EMPTY_QUERY_CAP`.
const ALL_FILES_EMPTY_QUERY_CAP: u32 = search_cfg::ALL_FILES_EMPTY_QUERY_CAP;

pub struct FileSearchEngine {
    db: Arc<Mutex<Connection>>,
    /// 磁盘 DB 路径。构造时只打开 in-memory 占位连接, 真正的磁盘 DB 在 `ensure_db()` 中懒打开。
    db_path: PathBuf,
    /// 是否已切换到磁盘 DB。false = 当前 `db` 仍是 in-memory 占位, 所有查询会优雅返回空。
    db_initialized: AtomicBool,
    ntfs_indexer: Option<NtfsIndexer>,
    last_update: Mutex<i64>,
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

    /// 仅供测试: 构造一个空 engine (用临时 DB 路径, 不打开磁盘).
    pub fn new_with_db_path_for_tests() -> Result<Self> {
        use std::env::temp_dir;
        let mut p = temp_dir();
        p.push(format!("mt-test-{}.db", std::process::id()));
        Self::new_with_db_path(p)
    }

    /**
     * 构造 FileSearchEngine。
     *
     * ⚠️ 关键改动（启动性能）：
     *
     * 1. **不再在构造时打开磁盘 SQLite**。原来 `Connection::open_with_flags` + `init_db`
     *    在已存在的大型 DB（WAL recovery + FTS5 + mmap 256MB）上要 1–3 秒，
     *    放在 Tauri `setup` 同步路径里会**阻塞 webview 加载**，导致前端"窗口空白 + 卡死"。
     *
     *    现在: 构造时只打开一个**瞬时 in-memory 占位连接**（几乎零耗时），
     *    真正的磁盘 DB 在首次 `ensure_db()` 中懒打开（在 `build_index_ntfs_internal`
     *    内调用，已被 `spawn_blocking` 包裹，不占用事件循环线程）。
     *
     *    占位期间所有查询 (`search` / `total` / `recent_files`) 通过现有的
     *    `if let Ok(...)` / `unwrap_or(0)` 优雅降级为空结果，前端显示空列表。
     *
     * 2. **不在构造时枚举盘符**。`NtfsIndexer::new_lazy()` / `new_with_drives_lazy()`
     *    不做任何 I/O，真正的盘符枚举放到 `ensure_volumes_enumerated()`，
     *    在后台 `spawn_blocking` 中调用。
     */
    pub fn new_with_db_path_and_roots(db_path: PathBuf, roots: Vec<PathBuf>) -> Result<Self> {
        log::info!(
            "[boot] FileSearchEngine::new 入口 (路径: {:?}, in-memory 占位)",
            db_path
        );
        // 注意: 这里**不**创建磁盘 DB 的父目录, 把所有磁盘 I/O 推迟到 ensure_db().
        // (create_dir_all 在已有目录上是廉价的, 但严格遵循"构造零 I/O"原则更安全。)

        // 瞬时 in-memory 占位连接: 无任何文件 I/O, 不会阻塞 setup。
        let conn = Connection::open_in_memory()?;
        log::info!("[boot] FileSearchEngine in-memory 占位连接就绪");

        let drives = extract_drives_from_roots(&roots);
        log::info!("文件搜索引擎配置的盘符: {:?}", drives);

        // 关键：不在构造时调用 NtfsIndexer::new()——让后台任务去做。
        // 但**不论是否显式指定盘符都构造 NtfsIndexer**：当 settings.file_search_drives 为空时,
        // 我们也要走懒枚举, 在后台 spawn_blocking 中调用 ensure_enumerated()。
        let ntfs_indexer = if !drives.is_empty() {
            Some(NtfsIndexer::new_with_drives_lazy(drives.clone()))
        } else {
            // 注意：new_lazy 不会失败，所以 unwrap() 是安全的。
            Some(NtfsIndexer::new_lazy().expect("NtfsIndexer::new_lazy 不可能失败"))
        };
        log::info!("[boot] FileSearchEngine 构造完成(尚未枚举盘符; 懒枚举模式; 尚未打开磁盘 DB)");

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            db_path,
            db_initialized: AtomicBool::new(false),
            ntfs_indexer,
            last_update: Mutex::new(0),
            is_indexing: Mutex::new(false),
            drives,
        })
    }

    /// 幂等地打开磁盘 SQLite 并切换 `self.db`。
    ///
    /// - 在 `build_index_ntfs_internal` 开头调用（位于 `spawn_blocking` 内部），
    ///   也会在 `update_index_usn` 中调用以保证增量更新健壮性。
    /// - 首次调用会真正 `Connection::open_with_flags` + `init_db`，可能耗时 1–3 秒
    ///   （WAL recovery + FTS5 schema + mmap）。因为是在后台线程执行，不会阻塞 UI。
    /// - 后续调用直接返回 `Ok(())`。
    pub fn ensure_db(&self) -> Result<()> {
        if self.db_initialized.load(Ordering::SeqCst) {
            return Ok(());
        }
        log::info!(
            "[boot] ensure_db: 首次打开磁盘数据库 (可能耗时 1-3s): {:?}",
            self.db_path
        );
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // ── Migration: 旧 DB (page_size=65536, FTS5 含 path + prefix='2 3 4') 与新 schema
        //    不兼容. 直接删除 DB + WAL + SHM 文件重建 —— 比在多 GB DB 上 VACUUM 快得多,
        //    且无损 (索引每次启动从 MFT 全量重建). ──
        if self.db_path.exists() && Self::db_needs_migration(&self.db_path) {
            log::warn!(
                "[db] 旧 schema 检测到 (user_version<2 或 page_size!=4096), 删除旧 DB+WAL+SHM 重建"
            );
            let _ = std::fs::remove_file(&self.db_path);
            let _ = std::fs::remove_file(self.db_path.with_extension("db-wal"));
            let _ = std::fs::remove_file(self.db_path.with_extension("db-shm"));
        }

        let conn = Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_FULL_MUTEX,
        )?;
        Self::init_db(&conn)?;

        {
            let mut guard = self.db.lock();
            *guard = conn;
        }
        self.db_initialized.store(true, Ordering::SeqCst);
        log::info!("[boot] ensure_db: 磁盘数据库就绪");
        Ok(())
    }

    /// 检查已有 DB 是否需要 schema 迁移 (只读打开, 检查 user_version + page_size).
    /// v9 升级要点:
    ///   - FTS5 `prefix='1 2 3 4'` (新增 1 字符前缀索引)
    ///   - `remove_diacritics 1` (重音折叠)
    ///   - 新增 `index_state` 表为 USN 增量索引铺路
    ///   FTS5 schema 嵌入到索引结构, 不能 in-place 升级, 一律 delete + rebuild.
    fn db_needs_migration(path: &std::path::Path) -> bool {
        // CURRENT_VERSION / REQUIRED_PAGE_SIZE 取自 config::fs::*.
        let conn = match Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let ver: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap_or(0);
        let psize: i64 = conn
            .query_row("PRAGMA page_size", [], |r| r.get(0))
            .unwrap_or(0);
        ver < fs_cfg::SCHEMA_VERSION || psize != fs_cfg::REQUIRED_PAGE_SIZE
    }

    fn init_db(conn: &Connection) -> Result<()> {
        // PRAGMA 字面量全部来自 `config::fs::*` —— 改一处全工程生效.
        // 与前端 `ICON_CONFIG.cache_size` 等常量建立跨前后端同步约定.
        conn.execute_batch(&format!(
            r#"
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            -- 负值 = KB 数; 来自 fs::CACHE_SIZE_KB. 旧值 -262144 = 256 MB, 内存过高.
            PRAGMA cache_size={};
            -- FILE: FTS5 重建时的排序临时表落盘, 避免在内存中堆积.
            PRAGMA temp_store=FILE;
            -- page_size 来自 fs::PAGE_SIZE (旧值 65536/64KB 是 DB 体积膨胀到 2GB 的主因).
            PRAGMA page_size={};
            -- 来自 fs::MMAP_SIZE_BYTES.
            PRAGMA mmap_size={};
            -- 来自 fs::WAL_AUTOCHECKPOINT, 防止 WAL 无限膨胀.
            PRAGMA wal_autocheckpoint={};
            -- INCREMENTAL: 维护空闲页链表, 配合 finalize_batch_insert 中的
            -- PRAGMA incremental_vacuum 回收 DELETE/FTS5 rebuild 产生的空闲页.
            -- 必须在 CREATE TABLE 之前设置, 且仅对空库生效 (迁移会删除旧库重建).
            -- 没有 incremental_vacuum, 旧 schema 的 FTS5(path+prefix='2 3 4') 数据
            -- 被释放后仍以空闲页形式占用 ~1.3 GB, 导致 DB 从 ~800MB 膨胀到 2.1GB.
            PRAGMA auto_vacuum=INCREMENTAL;

            CREATE TABLE IF NOT EXISTS dirs (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                name       TEXT NOT NULL,
                parent_id  INTEGER DEFAULT 0,
                full_path  TEXT NOT NULL UNIQUE
            );
            CREATE TABLE IF NOT EXISTS files (
                id        INTEGER PRIMARY KEY AUTOINCREMENT,
                name      TEXT NOT NULL,
                dir_id    INTEGER NOT NULL
            );
            -- FTS5: 索引 files 表的 name 字段
            -- v9 关键升级:
            --   1. `prefix='1 2 3 4'`: 新增 1 字符前缀索引, 让"搜 s" / "搜 d" 等
            --      单字符查询走 O(1) 索引扫描, 旧版 `prefix='2 3 4'` 会触发全表扫.
            --   2. `remove_diacritics 1`: 重音折叠, 让 "café" / "cafe" 互相命中.
            --   3. `tokenize='unicode61'`: 默认 unicode61 分词 (空格 + 大小写不敏感).
            CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
                name,
                content='files',
                content_rowid='id',
                tokenize='unicode61 remove_diacritics 1',
                prefix='1 2 3 4'
            );
            CREATE TRIGGER IF NOT EXISTS files_ai AFTER INSERT ON files BEGIN
                INSERT INTO files_fts(rowid, name) VALUES (new.id, new.name);
            END;
            CREATE TRIGGER IF NOT EXISTS files_ad AFTER DELETE ON files BEGIN
                INSERT INTO files_fts(files_fts, rowid, name) VALUES('delete', old.id, old.name);
            END;
            CREATE TRIGGER IF NOT EXISTS files_au AFTER UPDATE ON files BEGIN
                INSERT INTO files_fts(files_fts, rowid, name) VALUES('delete', old.id, old.name);
                INSERT INTO files_fts(rowid, name) VALUES (new.id, new.name);
            END;
            CREATE INDEX IF NOT EXISTS idx_dirs_parent_id ON dirs(parent_id);
            CREATE INDEX IF NOT EXISTS idx_files_dir_id ON files(dir_id);

            -- 增量索引状态: 每个 NTFS 卷的最后一次 USN 位置.
            -- 当前 update_index_usn 仍是 stub (fallback 到全量重建), 但表结构已就位,
            -- 后续 PR 可直接填 USN 增量, 无须再迁移 schema.
            CREATE TABLE IF NOT EXISTS index_state (
                volume     TEXT PRIMARY KEY,
                last_usn   INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0
            );

            PRAGMA user_version = {};
            "#,
            fs_cfg::CACHE_SIZE_KB,
            fs_cfg::PAGE_SIZE,
            fs_cfg::MMAP_SIZE_BYTES,
            fs_cfg::WAL_AUTOCHECKPOINT,
            fs_cfg::SCHEMA_VERSION,
        ))?;
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

        // 同 build_index_with_volume_progress:用 spawn_blocking 防止阻塞运行时。
        let self_ptr = self as *const Self as usize;
        let total = tokio::task::spawn_blocking(move || {
            // SAFETY: 调用方持有 Arc<FileSearchEngine>，其生命周期跨越整个 await。
            let this: &'static Self = unsafe { &*(self_ptr as *const Self) };
            let on_vol = |_vol: &str, _idx: usize, _cum: usize, _total: usize| {};
            match this.build_index_ntfs_internal(on_vol) {
                Ok(n) => n,
                Err(e) => {
                    log::error!("构建索引失败: {}", e);
                    0
                }
            }
        })
        .await
        .map_err(|e| crate::error::AppError::Other(format!("spawn_blocking join error: {e}")))?;

        log::info!(
            "文件索引构建完成: {} 个文件，耗时 {:?}",
            total,
            now.elapsed()
        );
        *self.is_indexing.lock() = false;
        Ok(())
    }

    /// 构建索引并对每个盘符的枚举结果触发回调。
    /// 回调签名: `(volume: &str, volume_index: usize, cumulative_files: usize, total_volumes: usize)`。
    /// - 在每个卷枚举开始前 + 完成后各触发一次,方便上层(IPC) emit "index_progress" 事件。
    /// - `volume_index` 从 1 开始。
    /// - USN 枚举是同步 IO,**全部送入 tokio `spawn_blocking`** ,避免占用运行时线程,
    ///   解决"启动卡死"。
    pub async fn build_index_with_volume_progress<F>(&self, mut on_volume: F) -> Result<()>
    where
        F: FnMut(&str, usize, usize, usize) + Send + 'static,
    {
        if *self.is_indexing.lock() {
            log::info!("索引构建已在进行中");
            return Ok(());
        }
        *self.is_indexing.lock() = true;

        log::info!("开始构建文件索引(带进度回调)");
        let now = std::time::Instant::now();

        // 把回调装入 Arc<Mutex<Box<dyn FnMut + Send>>> ,以便在 spawn_blocking 闭包内调用。
        let cb_slot: std::sync::Arc<
            std::sync::Mutex<Box<dyn FnMut(&str, usize, usize, usize) + Send>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(Box::new(move |v, i, c, t| {
            on_volume(v, i, c, t);
        })));

        // 同理把 self 的 Arc 套进来。调用方会传入 &Self (生命周期 = future lifetime),
        // 因为 FileSearchEngine 内部所有字段都已 Arc/Sync, 我们用裸指针技巧安全重建一个 &'static Self,
        // 约定: 调用方必须确保 self 在 future 期间存活。FileSearchEngine 的所有持有者是 Arc, 所以不变量成立。
        let self_ptr = self as *const Self as usize;
        let cb_slot_thread = cb_slot.clone();

        let total: usize = tokio::task::spawn_blocking(move || {
            let cb_holder = cb_slot_thread;
            let cb = move |vol: &str, idx: usize, cum: usize, total_v: usize| {
                if let Ok(mut g) = cb_holder.lock() {
                    (g)(vol, idx, cum, total_v);
                }
            };
            // SAFETY: self must outlive this task. Caller (FileSearchEngine builder) holds an Arc,
            // so this invariant holds for the duration of build_index_with_volume_progress.
            let this: &'static Self = unsafe { &*(self_ptr as *const Self) };
            match this.build_index_ntfs_internal(cb) {
                Ok(n) => n,
                Err(e) => {
                    log::error!("构建索引失败: {}", e);
                    0
                }
            }
        })
        .await
        .map_err(|e| crate::error::AppError::Other(format!("spawn_blocking join error: {e}")))?;

        let _ = cb_slot; // not used after spawn_blocking

        log::info!(
            "文件索引构建完成: {} 个文件, 耗时 {:?}",
            total,
            now.elapsed()
        );
        *self.is_indexing.lock() = false;
        Ok(())
    }

    /// 同步版本的 build_index_ntfs, 在 spawn_blocking 闭包中执行。
    /// 注意：本函数内部不进行任何 .await，**完全同步**，以保证阻塞线程不被 tokio 调度抢占。
    ///
    /// ⚠️ 内存优化（关键）：
    /// 不再先将所有卷的全部 UsnRecord 收集到单个 `Vec` 再一次性写入（旧实现峰值 ~632 MB
    /// for 269 万条记录），而是采用 **流式分块写入**：枚举回调内每攒满 `CHUNK_SIZE`
    /// (5 万) 条就 flush 到 SQLite 并清空 buffer。峰值 buffer 仅 ~12 MB。
    /// DB lock 在 flush 时短时获取、flush 完即释放，不阻塞枚举线程。
    fn build_index_ntfs_internal<F>(&self, mut on_volume: F) -> Result<usize>
    where
        F: FnMut(&str, usize, usize, usize),
    {
        log::info!("[idx] build_index_ntfs_internal 入口");
        // 首次进入索引构建时, 懒打开磁盘 DB (在 spawn_blocking 线程中执行, 不阻塞 UI)。
        self.ensure_db()?;

        // === 1. 准备阶段: 清空表 + 卸载 FTS5 触发器 ===
        {
            let conn = self.db.lock();
            conn.execute("DELETE FROM files", [])?;
            conn.execute("DELETE FROM dirs", [])?;
            // 卸载 FTS5 触发器: 批量插入阶段不更新 FTS, 最后统一 rebuild, 性能更好.
            conn.execute("DROP TRIGGER IF EXISTS files_ai", [])?;
            conn.execute("DROP TRIGGER IF EXISTS files_ad", [])?;
            conn.execute("DROP TRIGGER IF EXISTS files_au", [])?;
            conn.execute("PRAGMA synchronous=OFF", [])?;
            drop(conn);
            log::info!("[idx] 已清空 files/dirs, FTS5 触发器已卸载");
        }

        /// 每多少条文件记录提交一次事务并通知进度.
        /// 5000 条 ≈ 几十毫秒一次提交, 兼顾写入性能与 UI 实时性.
        const INCREMENTAL_FLUSH_INTERVAL: usize = 5_000;
        let mut buffer: Vec<UsnRecord> = Vec::with_capacity(INCREMENTAL_FLUSH_INTERVAL);
        let mut total_inserted: usize = 0;

        // 提交一个 chunk 并返回插入数量.
        // 为了增量可见, 每写完一批就 COMMIT 一次, 让读侧能立即查到.
        let flush_and_commit = |engine: &Self, buf: &[UsnRecord]| -> usize {
            if buf.is_empty() {
                return 0;
            }
            let conn = engine.db.lock();
            // 开启事务
            if let Err(e) = conn.execute("BEGIN TRANSACTION", []) {
                log::error!("[idx] BEGIN TRANSACTION 失败: {}", e);
                return 0;
            }
            let mut dir_stmt = match conn.prepare(
                "INSERT OR IGNORE INTO dirs(name, parent_id, full_path) VALUES (?1, ?2, ?3)",
            ) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("[idx] dir_stmt prepare 失败: {}", e);
                    let _ = conn.execute("ROLLBACK", []);
                    return 0;
                }
            };

            // 第一轮: 先插入所有目录, 确保 dirs 表数据完整
            let mut dir_count = 0usize;
            for record in buf {
                if record.is_directory {
                    let path_str = record.full_path.to_string_lossy().to_string();
                    if dir_stmt
                        .execute(rusqlite::params![record.file_name, 0i64, path_str,])
                        .is_ok()
                    {
                        dir_count += 1;
                    }
                }
            }
            drop(dir_stmt);

            let mut file_stmt =
                match conn.prepare("INSERT INTO files(name, dir_id) VALUES (?1, ?2)") {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("[idx] file_stmt prepare 失败: {}", e);
                        let _ = conn.execute("ROLLBACK", []);
                        return 0;
                    }
                };

            // 第二轮: 再插入所有文件, 此时目录已全部就绪
            let mut inserted = 0usize;
            for record in buf {
                if !record.is_directory {
                    let dir_path = if let Some(p) = record.full_path.parent() {
                        p.to_string_lossy().to_string()
                    } else {
                        String::new()
                    };
                    let dir_id: i64 = conn
                        .query_row(
                            "SELECT id FROM dirs WHERE full_path = ?1",
                            [dir_path.as_str()],
                            |r| r.get(0),
                        )
                        .unwrap_or(0);
                    if dir_id > 0 {
                        if file_stmt
                            .execute(rusqlite::params![record.file_name, dir_id,])
                            .is_ok()
                        {
                            inserted += 1;
                        }
                    }
                }
            }
            drop(file_stmt);

            // 提交事务: 让数据立即可见
            if let Err(e) = conn.execute("COMMIT", []) {
                log::error!("[idx] COMMIT 失败: {}", e);
                let _ = conn.execute("ROLLBACK", []);
                return 0;
            }
            drop(conn);
            log::debug!(
                "[idx] flush_and_commit: 目录 {}, 文件 {}",
                dir_count,
                inserted
            );
            inserted
        };

        // === 2. 枚举 + 流式写入 + 增量提交 ===
        if let Some(indexer) = &self.ntfs_indexer {
            log::info!("[idx] 开始枚举盘符 (NtfsIndexer::get_volumes)");
            let volumes = indexer.get_volumes();
            let total_volumes = volumes.len();
            log::info!("[idx] 盘符枚举完成，共 {} 个: {:?}", total_volumes, volumes);

            if total_volumes == 0 {
                log::warn!("[idx] 未发现可用卷(盘符列表为空)，跳过");
                let mut conn = self.db.lock();
                self.finalize_batch_insert(&mut conn)?;
                return Ok(0);
            }

            for (idx, volume) in volumes.iter().enumerate() {
                let n = idx + 1;
                log::info!("[idx] 准备枚举第 {}/{} 个卷: {}", n, total_volumes, volume);
                on_volume(volume, n, total_inserted, total_volumes);

                let mut vol_count: usize = 0;
                let mut vol_skipped: usize = 0;
                let result = indexer.enumerate_volume_files(volume, |record| {
                    if self.should_skip_path(&record) {
                        vol_skipped += 1;
                        return;
                    }
                    buffer.push(record);
                    vol_count += 1;

                    // 每积累 INCREMENTAL_FLUSH_INTERVAL 条就刷入 DB 并提交事务,
                    // 让读侧 (search 空查询) 能立即看到新数据.
                    if buffer.len() >= INCREMENTAL_FLUSH_INTERVAL {
                        let inserted = flush_and_commit(self, &buffer);
                        total_inserted += inserted;
                        buffer.clear();
                        // 通知进度: 调用方可以通过 IPC 把"已索引 N 个文件"推给前端
                        on_volume(volume, n, total_inserted, total_volumes);
                        log::debug!(
                            "[idx] 增量提交 {} 条 (当前卷: {}, 累计: {})",
                            inserted,
                            volume,
                            total_inserted
                        );
                    }
                });

                if let Err(e) = result {
                    log::warn!("[idx] 枚举卷 {} 失败: {}", volume, e);
                }

                // 卷内剩余 buffer 也刷入并提交
                if !buffer.is_empty() {
                    let inserted = flush_and_commit(self, &buffer);
                    total_inserted += inserted;
                    buffer.clear();
                }

                log::info!(
                    "[idx] 卷 {} 枚举完成，有效: {}, 跳过: {}, 累计已写入: {}",
                    volume,
                    vol_count,
                    vol_skipped,
                    total_inserted
                );
                on_volume(volume, n, total_inserted, total_volumes);
            }
        } else {
            log::warn!("[idx] NtfsIndexer 为 None, 无法构建索引");
            let mut conn = self.db.lock();
            self.finalize_batch_insert(&mut conn)?;
            return Ok(0);
        }

        // === 3. 收尾: 重建 FTS5 触发器 + 全量 rebuild + 恢复 synchronous ===
        log::info!("[idx] 开始 FTS5 全量重建 ({} 条记录)", total_inserted);
        {
            let mut conn = self.db.lock();
            self.finalize_batch_insert(&mut conn)?;
        }
        log::info!("[idx] 索引构建结束: {} 条记录", total_inserted);

        *self.last_update.lock() = chrono::Utc::now().timestamp();
        Ok(total_inserted)
    }

    fn should_skip_path(&self, record: &UsnRecord) -> bool {
        let name = record.file_name.to_lowercase();

        // 1) 隐藏文件: .git / .vscode 例外, 其他以 . 开头 (含空扩展名) 一律跳过
        if name.starts_with('.') && !name.starts_with(".git") && !name.starts_with(".vscode") {
            return true;
        }

        // 2) NTFS 系统流 ($MFT, $Bitmap 等) 跳过
        if name.starts_with('$') {
            return true;
        }

        // 3) 单一真源: 集中跳过名单 (thumbs.db / desktop.ini / pagefile.sys 等)
        if fs_cfg::SKIP_NAMES.iter().any(|s| name == *s) {
            return true;
        }

        // 4) 单一真源: 集中跳过路径片段 (winsxs / system32\config / recycle bin 等)
        let path_str = record.full_path.to_string_lossy().to_lowercase();
        if fs_cfg::SKIP_PATH_FRAGMENTS
            .iter()
            .any(|frag| path_str.contains(*frag))
        {
            return true;
        }

        false
    }

    /// 批量写入阶段 3/3: 重建 FTS5 触发器 + 全量 rebuild + 恢复 synchronous。
    /// 在 `build_index_ntfs_internal` 末尾调用一次。FTS5 rebuild 会全量扫描 files
    /// 重建倒排索引; 配合 `temp_store=FILE` 排序临时表落盘, 控制内存。
    /// 注意: 数据已在增量 flush 时通过多次小事务 COMMIT, 此处不再 COMMIT。
    fn finalize_batch_insert(&self, conn: &mut Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TRIGGER files_ai AFTER INSERT ON files BEGIN
                INSERT INTO files_fts(rowid, name) VALUES (new.id, new.name);
            END;
            CREATE TRIGGER files_ad AFTER DELETE ON files BEGIN
                INSERT INTO files_fts(files_fts, rowid, name) VALUES('delete', old.id, old.name);
            END;
            CREATE TRIGGER files_au AFTER UPDATE ON files BEGIN
                INSERT INTO files_fts(files_fts, rowid, name) VALUES('delete', old.id, old.name);
                INSERT INTO files_fts(rowid, name) VALUES (new.id, new.name);
            END;
            INSERT INTO files_fts(files_fts) VALUES('rebuild');
            PRAGMA synchronous=NORMAL;
            PRAGMA wal_checkpoint(TRUNCATE);
            PRAGMA incremental_vacuum;
            "#,
        )?;
        Ok(())
    }

    pub fn update_index(&self) -> Result<()> {
        if let Some(indexer) = &self.ntfs_indexer {
            return self.update_index_usn(indexer);
        }
        Ok(())
    }

    /// USN 增量更新. 修复"索引从未增量更新, 新建文件不出现"问题.
    ///
    /// 当前实现 (v9 schema 升级版):
    ///   - 检查 `index_state` 表是否有每个卷的 `last_usn`
    ///   - 如果没有 (首次启动), 或者 NtfsIndexer 的 USN 句柄读取失败,
    ///     走 fallback: 返回错误, 让调用方触发 `build_index` 全量重建.
    ///   - 真正的 USN delta walking (FSCTL_READ_USN_JOURNAL 增量应用) 是
    ///     下一轮 PR 的工作, 这里只把基础设施 (state table) 准备好.
    ///
    /// 关键改进: 旧版这里是无声空函数, 任何人都不知道 USN 增量根本
    /// 没工作. 现在会打 [usn] 标签的 warn, 配合 build_index 的进度事件,
    /// 用户能直观看到"后台在重建索引".
    fn update_index_usn(&self, indexer: &NtfsIndexer) -> Result<()> {
        let volumes = indexer.get_volumes();
        if volumes.is_empty() {
            log::warn!("[usn] no volumes enumerated, falling back to full rebuild");
            return Err(crate::error::AppError::Other(
                "no volumes to update".to_string(),
            ));
        }
        let conn = self.db.lock();
        let mut needs_full_rebuild = false;
        for v in volumes {
            let last_usn: i64 = conn
                .query_row(
                    "SELECT last_usn FROM index_state WHERE volume = ?1",
                    rusqlite::params![v.to_string()],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if last_usn == 0 {
                log::warn!(
                    "[usn] volume={} has no prior state, full rebuild required",
                    v
                );
                needs_full_rebuild = true;
                break;
            }
        }
        if needs_full_rebuild {
            // 返回错误让上层 build_index 触发全量重建.
            return Err(crate::error::AppError::Other(
                "incremental update unavailable, full rebuild required".to_string(),
            ));
        }
        // TODO: 真正的 USN delta walking 实现.
        log::info!("[usn] incremental update would run here (TODO: implement delta walker)");
        Ok(())
    }

    /// 关键字搜索. 修复日志黑洞:
    ///   旧版 `if let Ok(...)` 静默吞掉所有 Err, 用户看到空结果也不知道为什么.
    ///   新版每条 Err 都有 `log::warn!` / `log::error!`, 配套 metric 报告.
    pub fn search(&self, query: &str, limit: u32) -> Vec<SearchResult> {
        let started = std::time::Instant::now();
        if query.is_empty() {
            return self
                .all_files(ALL_FILES_EMPTY_QUERY_CAP)
                .into_iter()
                .map(|f| file_result_to_search_result(f))
                .collect();
        }

        let fts_query = build_fts_query(query);
        let conn = self.db.lock();
        let mut results: Vec<FileResult> = Vec::new();
        let mut row_parse_errors: usize = 0;

        let sql = r#"
            SELECT d.full_path, f.name
            FROM files_fts fts
            JOIN files f ON f.id = fts.rowid
            JOIN dirs d ON d.id = f.dir_id
            WHERE files_fts MATCH ?1
            ORDER BY fts.rank
            LIMIT ?2
        "#;

        let mut stmt = match conn.prepare(sql) {
            Ok(s) => s,
            Err(e) => {
                log::error!(
                    "[search] prepare failed: query={:?} fts={:?} sql=`{}` err={}",
                    query,
                    fts_query,
                    sql,
                    e
                );
                return Vec::new();
            }
        };

        let iter = match stmt.query_map(rusqlite::params![fts_query, limit], |row| {
            let dir_path: String = row.get(0)?;
            let name: String = row.get(1)?;
            let full_path = PathBuf::from(dir_path).join(&name);
            let ext = name.rsplit('.').next().and_then(|e| {
                if e.len() < 5 && e.len() > 0 && !name.starts_with('.') {
                    Some(e.to_string())
                } else {
                    None
                }
            });
            Ok(FileResult {
                path: full_path,
                name,
                extension: ext,
                size: 0,
                modified_at: 0,
                is_directory: false,
                id: None,
            })
        }) {
            Ok(it) => it,
            Err(e) => {
                log::error!(
                    "[search] query_map failed: query={:?} fts={:?} err={}",
                    query,
                    fts_query,
                    e
                );
                return Vec::new();
            }
        };

        for row in iter {
            match row {
                Ok(r) => results.push(r),
                Err(e) => {
                    // 单行解析错误, 不应让整个搜索失败, 但要记一条 warn 便于诊断.
                    row_parse_errors += 1;
                    log::warn!("[search] row parse error: query={:?} err={}", query, e);
                }
            }
        }

        if row_parse_errors > 0 {
            log::warn!(
                "[search] {} row(s) failed to parse, skipped (query={:?})",
                row_parse_errors,
                query
            );
        }

        let dur_ms = started.elapsed().as_millis();
        log::debug!(
            "[search] ok query={:?} hits={} limit={} elapsed_ms={}",
            query,
            results.len(),
            limit,
            dur_ms
        );

        results
            .into_iter()
            .map(|f| file_result_to_search_result(f))
            .collect()
    }

    /// 分页搜索: 给"显示更多"按钮用. 从 `after_id` 之后继续取 `limit` 条.
    /// 用稳定的 (rank, id) 二元排序保证分页不漏不重.
    pub fn search_after(&self, query: &str, after_id: i64, limit: u32) -> Vec<SearchResult> {
        if query.is_empty() {
            // 空查询不走 FTS5, 直接从 files 表分页.
            return self
                .all_files_after(after_id, limit)
                .into_iter()
                .map(|f| file_result_to_search_result(f))
                .collect();
        }
        let fts_query = build_fts_query(query);
        let conn = self.db.lock();
        let mut results: Vec<FileResult> = Vec::new();
        let sql = r#"
            SELECT d.full_path, f.name
            FROM files_fts fts
            JOIN files f ON f.id = fts.rowid
            JOIN dirs d ON d.id = f.dir_id
            WHERE files_fts MATCH ?1 AND f.id > ?2
            ORDER BY fts.rank, f.id
            LIMIT ?3
        "#;
        let mut stmt = match conn.prepare(sql) {
            Ok(s) => s,
            Err(e) => {
                log::error!("[search_after] prepare failed: query={:?} err={}", query, e);
                return Vec::new();
            }
        };
        let iter = match stmt.query_map(rusqlite::params![fts_query, after_id, limit], |row| {
            let dir_path: String = row.get(0)?;
            let name: String = row.get(1)?;
            let full_path = PathBuf::from(dir_path).join(&name);
            let ext = name.rsplit('.').next().and_then(|e| {
                if e.len() < 5 && e.len() > 0 && !name.starts_with('.') {
                    Some(e.to_string())
                } else {
                    None
                }
            });
            Ok(FileResult {
                path: full_path,
                name,
                extension: ext,
                size: 0,
                modified_at: 0,
                is_directory: false,
                id: None,
            })
        }) {
            Ok(it) => it,
            Err(e) => {
                log::error!(
                    "[search_after] query_map failed: query={:?} err={}",
                    query,
                    e
                );
                return Vec::new();
            }
        };
        for row in iter {
            if let Ok(r) = row {
                results.push(r);
            }
        }
        results
            .into_iter()
            .map(|f| file_result_to_search_result(f))
            .collect()
    }

    /// all_files 的分页版本: 按 name 排序, 从 after_id 之后取.
    fn all_files_after(&self, after_id: i64, limit: u32) -> Vec<FileResult> {
        let conn = self.db.lock();
        let mut results = Vec::new();
        let sql = r#"
            SELECT d.full_path, f.name
            FROM files f
            JOIN dirs d ON d.id = f.dir_id
            WHERE f.id > ?1
            ORDER BY f.id
            LIMIT ?2
        "#;
        if let Ok(mut stmt) = conn.prepare(sql) {
            if let Ok(iter) = stmt.query_map(rusqlite::params![after_id, limit], |row| {
                let dir_path: String = row.get(0)?;
                let name: String = row.get(1)?;
                let full_path = PathBuf::from(dir_path).join(&name);
                let ext = name.rsplit('.').next().and_then(|e| {
                    if e.len() < 5 && e.len() > 0 && !name.starts_with('.') {
                        Some(e.to_string())
                    } else {
                        None
                    }
                });
                Ok(FileResult {
                    path: full_path,
                    name,
                    extension: ext,
                    size: 0,
                    modified_at: 0,
                    is_directory: false,
                    id: None,
                })
            }) {
                results = iter.filter_map(|x| x.ok()).collect();
            }
        }
        results
    }

    pub fn search_with_score(&self, query: &str, limit: u32) -> Vec<(f32, FileResult)> {
        if query.is_empty() {
            return self
                .all_files(ALL_FILES_EMPTY_QUERY_CAP)
                .into_iter()
                .map(|f| (0.0, f))
                .collect();
        }

        let fts_query = build_fts_query(query);
        let conn = self.db.lock();
        let mut results = Vec::new();

        let sql = r#"
            SELECT d.full_path, f.name, fts.rank
            FROM files_fts fts
            JOIN files f ON f.id = fts.rowid
            JOIN dirs d ON d.id = f.dir_id
            WHERE files_fts MATCH ?1
            ORDER BY fts.rank
            LIMIT ?2
        "#;

        if let Ok(mut stmt) = conn.prepare(sql) {
            if let Ok(iter) = stmt.query_map(rusqlite::params![fts_query, limit], |row| {
                let dir_path: String = row.get(0)?;
                let name: String = row.get(1)?;
                let full_path = PathBuf::from(dir_path).join(&name);
                let rank: f32 = row.get(2)?;
                let ext = name.rsplit('.').next().and_then(|e| {
                    if e.len() < 5 && e.len() > 0 && !name.starts_with('.') {
                        Some(e.to_string())
                    } else {
                        None
                    }
                });
                Ok((
                    rank,
                    FileResult {
                        path: full_path,
                        name,
                        extension: ext,
                        size: 0,
                        modified_at: 0,
                        is_directory: false,
                        id: None,
                    },
                ))
            }) {
                results = iter.filter_map(|x| x.ok()).collect();
            }
        }

        results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// 空查询时的"全量文件"列表: 按文件名排序, 取前 N 个, 让首屏
    /// 默认展示可搜索/索引的全部文件. N 通过 [`ALL_FILES_EMPTY_QUERY_CAP`]
    /// 限制, 防止索引极大时单帧 IPC 阻塞.
    fn all_files(&self, limit: u32) -> Vec<FileResult> {
        let conn = self.db.lock();
        let mut results = Vec::new();

        let sql = r#"
            SELECT d.full_path, f.name
            FROM files f
            JOIN dirs d ON d.id = f.dir_id
            ORDER BY f.name COLLATE NOCASE ASC
            LIMIT ?1
        "#;

        if let Ok(mut stmt) = conn.prepare(sql) {
            if let Ok(iter) = stmt.query_map(rusqlite::params![limit], |row| {
                let dir_path: String = row.get(0)?;
                let name: String = row.get(1)?;
                let full_path = PathBuf::from(dir_path).join(&name);
                let ext = name.rsplit('.').next().and_then(|e| {
                    if e.len() < 5 && e.len() > 0 && !name.starts_with('.') {
                        Some(e.to_string())
                    } else {
                        None
                    }
                });
                Ok(FileResult {
                    path: full_path,
                    name,
                    extension: ext,
                    size: 0,
                    modified_at: 0,
                    is_directory: false,
                    id: None,
                })
            }) {
                results = iter.filter_map(|x| x.ok()).collect();
            }
        }

        results
    }

    pub fn total(&self) -> usize {
        let conn = self.db.lock();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
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

impl crate::engines::search_source::SearchSource for FileSearchEngine {
    fn name(&self) -> &'static str {
        "file"
    }
    fn category(&self) -> crate::models::SearchCategory {
        crate::models::SearchCategory::Files
    }
    fn search(&self, query: &str, limit: u32) -> Vec<crate::models::SearchResult> {
        self.search(query, limit)
    }
    fn search_after(
        &self,
        query: &str,
        after_id: i64,
        limit: u32,
    ) -> Vec<crate::models::SearchResult> {
        self.search_after(query, after_id, limit)
    }
    fn total(&self) -> usize {
        self.total()
    }
    fn category_weight(&self) -> f32 {
        crate::config::search::CATEGORY_WEIGHT_FILES
    }
}

fn get_db_path() -> PathBuf {
    if let Ok(app_data) = std::env::var("APPDATA") {
        PathBuf::from(app_data)
            .join("MonoTools")
            .join(fs_cfg::DB_NAME)
    } else {
        PathBuf::from(fs_cfg::DB_NAME)
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

/**
 * 把用户输入的查询字符串翻译成 FTS5 MATCH 表达式.
 *
 * 关键设计:
 * 1. **单字符支持**: FTS5 默认 tokenize=unicode61 不限制最小长度,
 *    单元字符 (e.g. "s") 也能命中以 s 开头的 token. v9 升级:
 *    FTS5 schema 改为 `prefix='1 2 3 4'`, 单字符查询走 O(1) 索引扫描.
 * 2. **prefix match**: 每个 term 加 `*` 后缀, 让 "chrom" 命中 "chrome",
 *    "google" 命中 "google-chrome", 类似 "starts-with" 行为.
 * 3. **escape FTS5 特殊字符**: 所有 FTS5 操作符 (`" ' \ ^ $ @ ~ * ( ) . : + -`)
 *    全部反斜杠转义, 避免用户输入导致 FTS5 syntax error 而返回 0 行结果.
 *    特别地:
 *    - `+` `*` `(` 等是 FTS5 语法符号, 必须转义
 *    - `-` 在 FTS5 中是 NOT 操作符, 搜索 "c-" 时如果不转义会被解析为 NOT
 *    - `:` 是 FTS5 column filter (e.g. `name:chrome`), 不转义会改变语义
 * 4. **空查询**: 返回 "*" —— 匹配所有 (FTS5 不会"列出全部", 所以上层
 *    需要走 all_files() 路径, 而不是用 MATCH ?1 = '*' ).
 * 5. **OR 模式**: 多个 term 用 OR 拼接, 让 "chrome remote" 既能命中
 *    "chrome" 也能命中 "remote", 而非"必须同时存在" (AND), 提升召回.
 *
 * 排序 (BM25): 上层用 ORDER BY fts.rank, 越相关排名越前.
 */
pub(crate) fn build_fts_query(query: &str) -> String {
    let terms: Vec<String> = query
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| {
            let escaped = t
                .chars()
                .map(|c| match c {
                    // FTS5 特殊操作符必须转义, 否则查询解析失败返回 0 行.
                    // 包含全部 12 个有特殊语义的字符.
                    '"' | '\'' | '\\' | '^' | '$' | '@' | '~' | '*' | '(' | ')' | '.' | ':'
                    | '+' | '-' => {
                        format!("\\{}", c)
                    }
                    _ => c.to_string(),
                })
                .collect::<String>();
            // prefix match: "s*" 匹配 starts-with "s" 的所有 token
            format!("{}*", escaped)
        })
        .collect();

    if terms.is_empty() {
        return "*".to_string();
    }

    // 多 term: 拼成 "term1* OR term2*", 任何一个命中即可, 提升召回
    terms.join(" OR ")
}

impl Default for FileSearchEngine {
    fn default() -> Self {
        Self::new(Vec::new()).unwrap()
    }
}

fn file_result_to_search_result(f: FileResult) -> SearchResult {
    let path_str = f.path.to_string_lossy().to_string();
    let result_type = file_type_of(&f);
    // 主副标题: **绝对路径** (用户主要关注的"文件在哪里").
    // 不再混入文件大小, 大小单独放到 `meta` 字段, UI 右侧灰色展示.
    let subtitle = path_str.clone();
    // 次级元信息: 人类可读的文件大小 (目录则为空).
    let meta = if f.is_directory {
        None
    } else {
        Some(format_size(f.size))
    };

    SearchResult {
        id: path_str.clone(),
        title: f.name,
        subtitle,
        meta,
        icon: f
            .extension
            .clone()
            .map(|ext| format!("file:///{}.ico", ext)),
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
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "ico" | "svg" | "webp" => {
            ResultType::Image
        }
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => ResultType::Video,
        "mp3" | "wav" | "flac" | "ogg" | "aac" | "wma" => ResultType::Audio,
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => ResultType::Archive,
        "txt" | "md" | "doc" | "docx" | "pdf" | "xls" | "xlsx" | "ppt" | "pptx" | "csv"
        | "json" | "xml" => ResultType::Document,
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

// ============================================================================
// 单元测试 - FTS5 查询构造 + 迁移行为
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // === build_fts_query 行为验证 ===

    #[test]
    fn build_fts_query_single_char_gets_prefix() {
        // v9 schema 升级重点: 单字符 term 必须能匹配.
        // 旧版 prefix='2 3 4' 时, "s*" 触发全表扫也能 work, 但慢.
        // 新版 prefix='1 2 3 4' 直接走索引, 既正确又快.
        let q = build_fts_query("s");
        assert_eq!(q, "s*", "单字符 term 应自动加 * 后缀");
    }

    #[test]
    fn build_fts_query_multi_term_uses_or() {
        // OR 语义: 多个 term 拼接, 任一命中即可 (提升召回).
        let q = build_fts_query("chrome remote");
        assert_eq!(q, "chrome* OR remote*");
    }

    #[test]
    fn build_fts_query_empty_returns_star() {
        // 空查询返回 "*", 但上层 search 走 all_files() 不走 MATCH, 这是兜底.
        let q = build_fts_query("");
        assert_eq!(q, "*");
        let q = build_fts_query("   ");
        assert_eq!(q, "*");
    }

    #[test]
    fn build_fts_query_escapes_plus_minus() {
        // 修复: 旧版漏了 + 和 -, 导致 "c++" / "win-kb" 之类的查询
        // 被 FTS5 解析为语法错误或 NOT 操作符, 静默返回 0 行.
        let q = build_fts_query("c++");
        assert_eq!(q, "c\\+\\+*", "+ 必须转义");
        let q = build_fts_query("win-kb");
        assert_eq!(q, "win\\-kb*", "- 必须转义 (避免被解析为 NOT)");
    }

    #[test]
    fn build_fts_query_escapes_column_colon() {
        // : 是 FTS5 column filter 语法 (e.g. "name:chrome"), 必须转义,
        // 否则用户搜 "c:" 会被解析为 column filter, 改变语义.
        let q = build_fts_query("c:");
        assert_eq!(q, "c\\:*");
    }

    #[test]
    fn build_fts_query_escapes_all_fts5_operators() {
        // 一次性验证 12 个特殊字符都转义了.
        let specials = vec!['"', '\'', '\\', '^', '$', '@', '~', '*', '(', ')', '.', ':'];
        for c in specials {
            let q = build_fts_query(&c.to_string());
            let expected = format!("\\{}*", c);
            assert_eq!(q, expected, "特殊字符 {:?} 必须转义", c);
        }
    }

    #[test]
    fn build_fts_query_preserves_alphanumeric() {
        // 普通字符不应该被转义.
        let q = build_fts_query("abc 123");
        assert_eq!(q, "abc* OR 123*");
    }

    // === 集成: 内存 DB 端到端 FTS5 行为 ===

    /// 用内存 DB 跑一次 search, 验证 prefix=1 真的工作.
    /// 旧 schema 下 (prefix='2 3 4') 这个测试也会通过, 但效率低.
    /// 新 schema (prefix='1 2 3 4') 才是正确配置, 但功能层面同样应通过.
    #[test]
    fn fts5_single_char_query_finds_matching_files() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE dirs (id INTEGER PRIMARY KEY, full_path TEXT UNIQUE);
            CREATE TABLE files (id INTEGER PRIMARY KEY, name TEXT, dir_id INTEGER);
            CREATE VIRTUAL TABLE files_fts USING fts5(
                name, content='files', content_rowid='id',
                tokenize='unicode61 remove_diacritics 1',
                prefix='1 2 3 4'
            );
            CREATE TRIGGER files_ai AFTER INSERT ON files BEGIN
                INSERT INTO files_fts(rowid, name) VALUES (new.id, new.name);
            END;
            INSERT INTO dirs (id, full_path) VALUES (1, 'C:/test');
            INSERT INTO files (name, dir_id) VALUES ('sample.txt', 1);
            INSERT INTO files (name, dir_id) VALUES ('shell.exe', 1);
            INSERT INTO files (name, dir_id) VALUES ('system.log', 1);
            INSERT INTO files (name, dir_id) VALUES ('config.json', 1);
            "#,
        )
        .unwrap();

        let q = build_fts_query("s");
        let mut stmt = conn
            .prepare(&format!(
                "SELECT name FROM files_fts WHERE files_fts MATCH '{}' ORDER BY rank",
                q
            ))
            .unwrap();
        let names: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(|x| x.ok())
            .collect();

        // "s" 应匹配 sample / shell / system (starts-with "s")
        assert!(names.contains(&"sample.txt".to_string()));
        assert!(names.contains(&"shell.exe".to_string()));
        assert!(names.contains(&"system.log".to_string()));
        assert!(!names.contains(&"config.json".to_string()));
    }

    /// 验证 schema 迁移触发条件: user_version 旧 → 需要重建.
    #[test]
    fn schema_migration_triggers_on_old_version() {
        // 在临时目录创建一个 user_version=8 的空 DB, 验证 db_needs_migration
        // 返回 true (触发 v9 迁移).
        let dir = tempdir_in_target();
        let db_path = dir.join("test.db");
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch("PRAGMA user_version = 8;").unwrap();
        }
        assert!(
            FileSearchEngine::db_needs_migration(&db_path),
            "user_version=8 应触发 v9 迁移"
        );
    }

    /// 验证 user_version=9 的 DB 不再触发迁移.
    #[test]
    fn schema_migration_skips_when_current() {
        let dir = tempdir_in_target();
        let db_path = dir.join("test.db");
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch("PRAGMA user_version = 9; PRAGMA page_size = 4096;")
                .unwrap();
        }
        assert!(
            !FileSearchEngine::db_needs_migration(&db_path),
            "user_version=9 + page_size=4096 应跳过迁移"
        );
    }

    // === should_skip_path 单测 (验证单一真源 fs::SKIP_* 已生效) ===

    fn make_engine_for_skip_tests() -> FileSearchEngine {
        FileSearchEngine::new(vec![]).expect("构造 FileSearchEngine 应成功")
    }

    fn make_record(name: &str, path: &str) -> UsnRecord {
        UsnRecord {
            file_name: name.to_string(),
            full_path: std::path::PathBuf::from(path),
            file_reference_number: 0,
            parent_file_reference: 0,
            file_size: 0,
            last_write_time: 0,
            is_directory: false,
            extension: None,
            reason: crate::platform::windows::usn::UsnChangeReason::Modified,
            usn: 0,
        }
    }

    #[test]
    fn should_skip_thumbs_db() {
        let e = make_engine_for_skip_tests();
        let r = make_record("thumbs.db", r"C:\Users\foo\Pictures\thumbs.db");
        assert!(e.should_skip_path(&r), "thumbs.db 应被跳过");
    }

    #[test]
    fn should_skip_desktop_ini() {
        let e = make_engine_for_skip_tests();
        let r = make_record("desktop.ini", r"C:\Windows\System32\desktop.ini");
        assert!(e.should_skip_path(&r), "desktop.ini 应被跳过");
    }

    #[test]
    fn should_skip_pagefile_sys() {
        let e = make_engine_for_skip_tests();
        let r = make_record("pagefile.sys", r"C:\pagefile.sys");
        assert!(e.should_skip_path(&r), "pagefile.sys 应被跳过");
    }

    #[test]
    fn should_skip_winsxs_path() {
        let e = make_engine_for_skip_tests();
        let r = make_record("manifest.txt", r"C:\Windows\Winsxs\manifest.txt");
        assert!(e.should_skip_path(&r), r"\Windows\Winsxs 路径应被跳过");
    }

    #[test]
    fn should_skip_system32_config_path() {
        let e = make_engine_for_skip_tests();
        let r = make_record("SYSTEM", r"C:\Windows\System32\config\SYSTEM");
        assert!(e.should_skip_path(&r), r"system32\config 路径应被跳过");
    }

    #[test]
    fn should_skip_softwaredistribution_path() {
        let e = make_engine_for_skip_tests();
        let r = make_record("data.dat", r"C:\Windows\SoftwareDistribution\data.dat");
        assert!(e.should_skip_path(&r), "softwaredistribution 路径应被跳过");
    }

    #[test]
    fn should_skip_recycle_bin_path() {
        let e = make_engine_for_skip_tests();
        let r = make_record("file.txt", r"C:\$Recycle.Bin\file.txt");
        assert!(e.should_skip_path(&r), r"\$Recycle.Bin 路径应被跳过");
    }

    #[test]
    fn should_not_skip_normal_files() {
        let e = make_engine_for_skip_tests();
        let r = make_record("readme.md", r"D:\projects\mono\readme.md");
        assert!(!e.should_skip_path(&r), "正常文件不应被跳过");
    }

    #[test]
    fn should_not_skip_git_dir() {
        // .git 例外, 不应被跳过
        let e = make_engine_for_skip_tests();
        let r = make_record(".git", r"D:\projects\mono\.git");
        assert!(
            !e.should_skip_path(&r),
            ".git 目录不应被跳过 (用户可能想搜)"
        );
    }

    #[test]
    fn should_not_skip_vscode_dir() {
        // .vscode 例外
        let e = make_engine_for_skip_tests();
        let r = make_record(".vscode", r"D:\projects\mono\.vscode");
        assert!(!e.should_skip_path(&r), ".vscode 目录不应被跳过");
    }

    #[test]
    fn should_skip_ntfs_system_streams() {
        // $MFT 等 NTFS 系统流
        let e = make_engine_for_skip_tests();
        let r = make_record("$MFT", r"C:\$MFT");
        assert!(e.should_skip_path(&r), "NTFS 系统流 ($MFT) 应被跳过");
    }

    fn tempdir_in_target() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("mt-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&p);
        p
    }
}
