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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const DB_NAME: &str = "monotools_file_index.db";

/// 空查询时 (例如首屏未输入关键字), 后端一次性返回的最多文件数.
/// 限制: 防止索引极大 (几十万文件) 时单次 IPC 序列化阻塞 UI.
/// 经验值: 100k 索引时, 取 500 ≈ 200KB JSON, IPC < 30ms.
const ALL_FILES_EMPTY_QUERY_CAP: u32 = 500;

pub struct FileSearchEngine {
    db: Arc<Mutex<Connection>>,
    /// 磁盘 DB 路径。构造时只打开 in-memory 占位连接, 真正的磁盘 DB 在 `ensure_db()` 中懒打开。
    db_path: PathBuf,
    /// 是否已切换到磁盘 DB。false = 当前 `db` 仍是 in-memory 占位, 所有查询会优雅返回空。
    db_initialized: AtomicBool,
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
        log::info!("[boot] FileSearchEngine::new 入口 (路径: {:?}, in-memory 占位)", db_path);
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
        log::info!(
            "[boot] FileSearchEngine 构造完成(尚未枚举盘符; 懒枚举模式; 尚未打开磁盘 DB)"
        );

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            db_path,
            db_initialized: AtomicBool::new(false),
            ntfs_indexer,
            last_update: Mutex::new(0),
            indexed_paths: Arc::new(Mutex::new(HashSet::new())),
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
    fn db_needs_migration(path: &std::path::Path) -> bool {
        let conn = match Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let ver: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap_or(0);
        let psize: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0)).unwrap_or(0);
        ver < 8 || psize != 4096
    }

    fn init_db(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            -- 负值 = KB 数; -65536 = 64 MB (旧值 -262144 = 256 MB, 内存过高).
            PRAGMA cache_size=-65536;
            -- FILE: FTS5 重建时的排序临时表落盘, 避免在内存中堆积.
            PRAGMA temp_store=FILE;
            -- 4 KB 页 (旧值 65536/64KB 是 DB 体积膨胀到 2GB 的主因: 内部碎片 16x).
            PRAGMA page_size=4096;
            -- 64 MB mmap (旧值 256 MB, 过高).
            PRAGMA mmap_size=67108864;
            -- 自动 checkpoint WAL, 防止 WAL 无限膨胀 (旧配置无此设置, WAL 达 5.4 GB).
            PRAGMA wal_autocheckpoint=5000;
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
            CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
                name,
                content='files',
                content_rowid='id',
                tokenize='unicode61'
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

            PRAGMA user_version = 8;
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
        let cb_slot: std::sync::Arc<std::sync::Mutex<Box<dyn FnMut(&str, usize, usize, usize) + Send>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Box::new(move |v, i, c, t| {
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

        log::info!("文件索引构建完成: {} 个文件, 耗时 {:?}", total, now.elapsed());
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

        // === 1. 准备阶段: 清空表 + 开启事务 + 卸载 FTS5 触发器 ===
        {
            let mut conn = self.db.lock();
            conn.execute("DELETE FROM files", [])?;
            conn.execute("DELETE FROM dirs", [])?;
            self.begin_batch_insert(&mut conn)?;
            drop(conn);
            log::info!("[idx] 已清空 files/dirs, 事务已开启, FTS5 触发器已卸载");
        }

        const CHUNK_SIZE: usize = 50_000;
        let mut buffer: Vec<UsnRecord> = Vec::with_capacity(CHUNK_SIZE);
        let mut total_inserted: usize = 0;

        // === 2. 枚举 + 流式写入 ===
        if let Some(indexer) = &self.ntfs_indexer {
            // 注意: 这里是同步盘符枚举(可能耗时!). 这是已知路径, 已被 spawn_blocking 包裹.
            log::info!("[idx] 开始枚举盘符 (NtfsIndexer::get_volumes)");
            let volumes = indexer.get_volumes();
            let total_volumes = volumes.len();
            log::info!(
                "[idx] 盘符枚举完成，共 {} 个: {:?}",
                total_volumes,
                volumes
            );

            if total_volumes == 0 {
                log::warn!("[idx] 未发现可用卷(盘符列表为空)，跳过");
                // 即使 0 卷也要 finalize (COMMIT 事务), 否则连接卡在事务中.
                let mut conn = self.db.lock();
                self.finalize_batch_insert(&mut conn)?;
                return Ok(0);
            }

            for (idx, volume) in volumes.iter().enumerate() {
                let n = idx + 1;
                log::info!("[idx] 准备枚举第 {}/{} 个卷: {}", n, total_volumes, volume);
                on_volume(volume, n, total_inserted, total_volumes);

                // 内联原 enumerate_volume 逻辑: 回调内边枚举边分块 flush.
                let mut vol_count: usize = 0;
                let mut vol_skipped: usize = 0;
                let result = indexer.enumerate_volume_files(volume, |record| {
                    if self.should_skip_path(&record) {
                        vol_skipped += 1;
                        return;
                    }
                    buffer.push(record);
                    vol_count += 1;

                    // buffer 满 → flush 到 DB 并清空, 控制峰值内存.
                    if buffer.len() >= CHUNK_SIZE {
                        let inserted = self.flush_chunk_to_db(&buffer);
                        total_inserted += inserted;
                        buffer.clear();
                        log::debug!(
                            "[idx] 已插入 {} 条记录 (当前卷: {})",
                            total_inserted, volume
                        );
                    }
                });

                if let Err(e) = result {
                    log::warn!("[idx] 枚举卷 {} 失败: {}", volume, e);
                }

                log::info!(
                    "[idx] 卷 {} 枚举完成，有效: {}, 跳过: {}, 累计已写入: {}",
                    volume, vol_count, vol_skipped, total_inserted
                );
                on_volume(volume, n, total_inserted, total_volumes);
            }
        } else {
            log::warn!("[idx] NtfsIndexer 为 None, 无法构建索引");
            let mut conn = self.db.lock();
            self.finalize_batch_insert(&mut conn)?;
            return Ok(0);
        }

        // === 3. 刷入剩余记录 (< CHUNK_SIZE 的尾巴) ===
        if !buffer.is_empty() {
            let inserted = self.flush_chunk_to_db(&buffer);
            total_inserted += inserted;
            buffer.clear();
            log::info!("[idx] 刷入最后 {} 条, 总计 {}", inserted, total_inserted);
        }

        // === 4. 收尾: 重建 FTS5 触发器 + 全量 rebuild + COMMIT ===
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

        if name.starts_with('.') && !name.starts_with(".git") && !name.starts_with(".vscode") {
            return true;
        }

        if name.starts_with('$') {
            return true;
        }

        if name == "thumbs.db"
            || name == "desktop.ini"
            || name == "pagefile.sys"
            || name == "hiberfil.sys"
        {
            return true;
        }

        let path_str = record.full_path.to_string_lossy().to_lowercase();
        if path_str.contains("\\windows\\winsxs")
            || path_str.contains("\\windows\\system32\\config")
            || path_str.contains("\\windows\\softwaredistribution")
        {
            return true;
        }

        false
    }

    /// 批量写入阶段 1/3: 开启事务 + 卸载 FTS5 触发器。
    /// 在 `build_index_ntfs_internal` 开头调用一次, 与 `flush_chunk_to_db` / `finalize_batch_insert` 配合。
    fn begin_batch_insert(&self, conn: &mut Connection) -> Result<()> {
        conn.execute("PRAGMA synchronous=OFF", [])?;
        conn.execute("BEGIN TRANSACTION", [])?;
        conn.execute("DROP TRIGGER IF EXISTS files_ai", [])?;
        conn.execute("DROP TRIGGER IF EXISTS files_ad", [])?;
        conn.execute("DROP TRIGGER IF EXISTS files_au", [])?;
        Ok(())
    }

    /// 批量写入阶段 2/3: 将一个 chunk (<= CHUNK_SIZE) 写入 files_meta。
    /// **短时获取 DB lock** (lock → prepare → insert loop → drop), 不跨调用持有。
    /// 返回成功插入的条数 (INSERT OR IGNORE 可能跳过重复行, 但此处为全量重建, 一般全部成功)。
    fn flush_chunk_to_db(&self, buffer: &[UsnRecord]) -> usize {
        if buffer.is_empty() {
            return 0;
        }
        let conn = self.db.lock();
        let mut dir_stmt = match conn.prepare(
            "INSERT OR IGNORE INTO dirs(name, parent_id, full_path) VALUES (?1, ?2, ?3)",
        ) {
            Ok(s) => s,
            Err(e) => {
                log::error!("[idx] dir_stmt prepare 失败: {}", e);
                return 0;
            }
        };
        let mut file_stmt = match conn.prepare(
            "INSERT INTO files(name, dir_id) VALUES (?1, ?2)",
        ) {
            Ok(s) => s,
            Err(e) => {
                log::error!("[idx] file_stmt prepare 失败: {}", e);
                return 0;
            }
        };

        let mut inserted = 0usize;
        for record in buffer {
            let path_str = record.full_path.to_string_lossy().to_string();
            if record.is_directory {
                let _ = dir_stmt.execute(rusqlite::params![
                    record.file_name,
                    0i64,
                    path_str,
                ]);
            } else {
                let dir_path = if let Some(p) = record.full_path.parent() {
                    p.to_string_lossy().to_string()
                } else {
                    String::new()
                };
                let dir_id: i64 = conn.query_row(
                    "SELECT id FROM dirs WHERE full_path = ?1",
                    [dir_path],
                    |r| r.get(0),
                ).unwrap_or(0);
                if dir_id > 0 {
                    let _ = file_stmt.execute(rusqlite::params![
                        record.file_name,
                        dir_id,
                    ]);
                    inserted += 1;
                }
            }
        }
        drop(dir_stmt);
        drop(file_stmt);
        drop(conn);
        inserted
    }

    /// 批量写入阶段 3/3: 重建 FTS5 触发器 + 全量 rebuild + COMMIT + 恢复 synchronous。
    /// 在 `build_index_ntfs_internal` 末尾调用一次。FTS5 rebuild 会全量扫描 files_meta
    /// 重建倒排索引; 配合 `temp_store=FILE` 排序临时表落盘, 控制内存。
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
            COMMIT;
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

    fn update_index_usn(&self, indexer: &NtfsIndexer) -> Result<()> {
        Ok(())
    }

    pub fn search(&self, query: &str, limit: u32) -> Vec<SearchResult> {
        if query.is_empty() {
            return self
                .all_files(ALL_FILES_EMPTY_QUERY_CAP)
                .into_iter()
                .map(|f| file_result_to_search_result(f))
                .collect();
        }

        let fts_query = build_fts_query(query);
        let conn = self.db.lock();
        let mut results = Vec::new();

        let sql = r#"
            SELECT d.full_path, f.name
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
            .into_iter()
            .map(|f| file_result_to_search_result(f))
            .collect()
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

    /// 旧版"最近文件"实现: 无排序 `LIMIT N` 切片.
    /// 当前未使用, 保留以备未来"最近 N 个访问"等场景. (2026-07 改用 [`Self::all_files`])
    #[allow(dead_code)]
    fn recent_files(&self, limit: u32) -> Vec<FileResult> {
        let conn = self.db.lock();
        let mut results = Vec::new();

        let sql = r#"
            SELECT d.full_path, f.name
            FROM files f
            JOIN dirs d ON d.id = f.dir_id
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

    /// 空查询时的"全量文件"列表: 按文件名排序, 取前 N 个, 让首屏
    /// 默认展示可搜索/索引的全部文件 (而非 recent_files 那种无序片段).
    /// N 通过 [`ALL_FILES_EMPTY_QUERY_CAP`] 限制, 防止索引极大时单帧 IPC 阻塞.
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
                    '"' | '\'' | '\\' | '^' | '$' | '@' | '~' | '*' | '(' | ')' | '.' => {
                        format!("\\{}", c)
                    }
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
