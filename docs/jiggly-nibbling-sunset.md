# MonoTools 性能优化方案

## Context（背景）

MonoTools 目标是**轻量级** Windows 启动器,但当前存在五类性能问题:启动慢、软件图标获取慢且每次重启全部重提取、文件索引建立时 CPU 打满 + 内存高 + DB 膨胀导致界面卡死无法操作、UWP 图标获取失败。本方案定位到每个问题的根因并给出修复,核心目标是让索引期间界面保持可操作、降低内存/CPU 峰值、消除重复的图标提取工作。

调查结论(根因)已确认:
1. **文件索引 N+1 查询**:`build_index_ntfs_internal` 的 `flush_and_commit` 对**每个文件**执行一次 `SELECT id FROM dirs WHERE full_path=?`(百万文件=百万次单查询)——CPU 打满的元凶。
2. **SQLite 页缓存 256MB**:`config.rs::fs::CACHE_SIZE_KB = -262144`(注释声称已降但值未改)——内存高的直接原因。
3. **USN 枚举全量驻留内存**:`enumerate_volume_files` 把整卷 `dir_records`+`file_records` 全收进 Vec 后才回调,下游的"流式分块 flush"被上游整卷缓冲抵消;且路径构建用最多 100 轮定点迭代(O(轮次×目录数))——内存峰值 + 额外 CPU。
4. **图标无磁盘缓存**:`icon.rs::ICON_CACHE` 仅内存(`OnceLock<Mutex<HashMap>>`),每次重启全部重提取;提取尺寸 256×256(RGBA 256KB/张 + autocrop + PNG 编码,重)。前端 localStorage 缓存仅 2MB 太小。
5. **UWP 图标失败**:UWP 应用 action 为 `Run{command:"explorer.exe",args:["shell:AppsFolder\\{aumid}"]}`,而前端 `sources/known.ts::extractPath` 只处理 `launch`/`open`/`navigate`,对 `run` 返回空 → 前端**从不请求** UWP 图标 → 永远走兜底。后端 Tier-2 其实支持 `shell:` 路径。
6. **启动阻塞**:`app_search/engine.rs::refresh_index_incremental` 的重活(WalkDir、注册表扫描、`scan_uwp_apps` 通过 `powershell` 子进程枚举)在 tokio async 任务里**同步执行未用 spawn_blocking**,占用 runtime 工作线程。

**已确认决策**:图标降到 128×128 + 新增后端磁盘持久化缓存;五项一次做完。

---

## 实施方案

### Phase 1 — 文件索引 CPU/内存/DB(痛点 3,最高优先级)

**文件**:`src-tauri/src/search_engine/file_search.rs`、`src-tauri/src/core/config.rs`

1. **消除 N+1 查询**(`build_index_ntfs_internal` / `flush_and_commit`):
   - 在整个索引构建过程维护一个 `HashMap<String, i64>`(dir full_path → dir rowid),跨 chunk 存活。
   - dir 插入后用 `conn.last_insert_rowid()` 或对 `INSERT OR IGNORE` 后 `SELECT` 一次性回填(仅目录数量级,远小于文件数),写入 map。
   - 文件插入时直接从 map 查 `dir_id`,**不再每文件 SELECT**。找不到父目录则跳过并计数。
   - 预期:插入阶段 CPU 从"百万次查询"降到"一次哈希查找/文件",大幅缩短索引时间并释放 UI。

2. **降低 SQLite 页缓存**(`config.rs::fs::CACHE_SIZE_KB`):
   - 从 `-262144`(256MB)改为 `-49152`(48MB)。同步更新注释使其与实际值一致。

3. **USN 枚举降内存**(`platform/windows/usn.rs::enumerate_volume_files`):
   - 目录路径构建完成后立即 `drop(dir_records)` 释放目录中间态;`path_cache` 构建后遍历 `file_records` 时用 `std::mem::take` 逐条消费,避免克隆 `file_name`/`full_path` 两份。
   - 定点迭代改为按 parent 已解析优先的单次拓扑推进(保留 100 轮上限作为安全网),减少无效重扫。
   - 保持"整卷两遍扫描"结构(路径重建必需),但把峰值 Vec 尽早释放。

4. **索引期间保持 UI 可操作**:
   - 每 flush 一个 chunk 后 `std::thread::sleep(Duration::from_millis(2~5))` 轻微让渡(在 `spawn_blocking` 线程内,不占 async runtime),避免独占磁盘/CPU;或用 Windows `SetThreadPriority(THREAD_PRIORITY_BELOW_NORMAL)` 降低索引线程优先级(平台隔离在 `platform/windows`)。优先选 sleep 让渡(简单、跨平台安全)。

### Phase 2 — 图标磁盘缓存 + 128px(痛点 2)

**文件**:`src-tauri/src/platform/windows/icon.rs`、`src-tauri/src/core/config.rs`、`src/core/config/icon.ts`

5. **提取尺寸 256 → 128**:改 `config.rs::icon::SIZE = 128`,同步前端 `src/core/config/icon.ts::ICON_CONFIG.size`(遵循 CLAUDE.md §1.3 跨前后端同步约定)。`icon::BUF_SIZE` 相应改为 `128*128*4`。

6. **新增后端磁盘持久化缓存**:
   - 在 app cache 目录(`AppHandle::path().app_cache_dir()`,经 IPC 层传入或用已知路径)下建 `icons/` 目录,文件名用 `cache_key(path)` 的哈希 + `.png`。
   - `get_or_extract_cached` 流程改为:内存缓存 → **磁盘缓存**(命中则读入内存并返回) → 真正提取 → 写内存 + **异步写磁盘**。
   - 缓存键带上目标文件 mtime(或 size),源文件更新时自动失效。
   - 磁盘缓存无需清空策略(图标小、总量有限);可加一个上限(如 500 张)按 mtime 淘汰。
   - 保持现有 single-flight(in-flight + condvar)机制不变。

### Phase 3 — UWP 图标(痛点 4)

**文件**:`src/ui/widgets/appicon/sources/known.ts`(`extractPath`)、`src-tauri/src/platform/windows/ipc.rs`(`get_app_icon`)

7. **前端 `extractPath` 支持 `run` 动作的 shell 路径**:当 `action.type === 'run'` 且 `command === 'explorer.exe'`、`args[0]` 以 `shell:AppsFolder\\` 开头时,返回 `args[0]` 作为图标提取路径。这样 UWP 项会正常发起 batch/单条 IPC。
   - 同步检查 `useAppIcon.ts::loadIconsBatch` 中的 `extractPath` 引用一致(同一函数,自动生效)。

8. **后端 `get_app_icon` / `get_app_icons_batch` 处理 shell 路径**:对以 `shell:` 开头的 path **跳过 `resolve_shortcut`**(该函数对 shell 命名空间路径无意义),直接进入 `get_or_extract_cached` → `extract_icon_windows` 的 Tier-2(`extract_rgba_via_shell_item_factory` 已支持 `SHParseDisplayName` 解析 `shell:AppsFolder\\{aumid}`)。确认 Tier-1 SHIL_JUMBO 对 shell 路径失败时能正确回退到 Tier-2(已有 fallback,只需确保不被 resolve_shortcut 破坏路径)。

### Phase 4 — 启动阻塞(痛点 1)

**文件**:`src-tauri/src/search_engine/app_search/engine.rs`、`src-tauri/src/search_engine/init.rs`

9. **应用索引扫描移入 spawn_blocking**:`refresh_index_incremental` 内的 WalkDir/注册表/UWP 扫描是同步阻塞,当前直接跑在 async 任务上。改为在 `init.rs::start_app_index_refresh` 里用 `tokio::task::spawn_blocking` 包裹实际扫描逻辑(进度回调通过 channel 或 Arc 回传),避免占用 runtime 工作线程,消除与文件索引 spawn_blocking 争抢导致的卡顿。

10. **UWP PowerShell 扫描降低启动影响**:`scan_uwp_apps` 的 `powershell` 子进程(1-3s)当前在 apps 阶段串行阻塞。将其拆为**独立的、最后执行的一个阶段**(已经是最后阶段之一),并确保它在 spawn_blocking 内;其结果可写入磁盘缓存,下次启动先读缓存再后台刷新(可选增强,若时间允许)。

### Phase 5 — 零散清理(痛点 5)

- 检查 `dirs.parent_id` 始终为 0 且 `idx_dirs_parent_id` 未被查询使用 → 若确认无用则移除该列/索引以缩小 DB(需 `grep` 确认无引用,遵循 CLAUDE.md §4.4)。
- 清理索引热路径上多余的 `to_string_lossy().to_string()` 克隆。

---

## 关键复用点

- 图标 single-flight:`icon.rs::in_flight()` / `in_flight_condvar()` — 磁盘缓存改造时保持不变。
- 配置单一真源:所有阈值改 `src-tauri/src/core/config.rs::{fs,icon}` 与 `src/core/config/icon.ts`(CLAUDE.md §1.1/§1.3)。
- 批量图标并发:`ipc.rs::get_app_icons_batch` 已有 spawn_blocking 分片并发,磁盘缓存命中后自动跳过提取。
- 进度回调机制:`init.rs` 已有 `on_progress(count, phase)` / `on_volume` 模式,spawn_blocking 化时复用。

## 跨前后端同步声明(CLAUDE.md §1.3)

本方案将同步修改:`config.rs::icon::SIZE` (256→128) ↔ `src/core/config/icon.ts::ICON_CONFIG.size`。PR 描述需声明"已同步前后端"。

---

## 验证方式

1. **编译**:`pnpm test:rust` + `cargo build`(src-tauri)确保无破坏。
2. **索引性能**(痛点 3):`pnpm dev` 启动,观察日志 `[idx] 索引构建结束: N 条记录, 耗时 ?`;索引进行时拖动/滚动搜索窗口应流畅可操作;任务管理器观察 monotools 进程内存峰值应显著低于原 256MB+ 水平;索引完成后检查 `%APPDATA%` 下 `monotools_file_index.db` 体积。
3. **图标缓存**(痛点 2):首次启动搜索应用观察图标提取耗时(日志 `[icon] batch ...`);**完全退出后重启**,同样搜索,图标应从磁盘缓存瞬时命中(extracted=0 或极少)。
4. **UWP 图标**(痛点 4):搜索一个 Microsoft Store 应用(如"计算器"/"照片"),图标应正确显示而非兜底方块。开 DevTools `window.__iconDebug.dump()` 确认 UWP 项走 `ipc` 级而非 `fallback`。
5. **启动**(痛点 1):冷启动后按 Alt+Space,窗口应快速出现且输入即时响应(索引在后台进行不阻塞交互)。
6. **回归**:`pnpm test`(Vitest)+ `pnpm test:rust` 全绿;普通 exe/lnk 图标、文件搜索、命令搜索功能不回归。
