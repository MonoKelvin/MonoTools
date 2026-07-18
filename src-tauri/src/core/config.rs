//! 全局配置中心 —— 把 12+ 后端 magic number / string 集中到一处.
//!
//! 设计原则:
//! - **单一真源**: 任何被 ≥ 2 处引用的字面量都应在此定义.
//! - **跨前后端同步**: 修改常量时必须同步改前端 `src/config/*.ts`,
//!   详见 CLAUDE.md "编码规范" 1.3.
//! - **不依赖运行时**: 全部 `pub const`, 无 lazy / 无 state, 编译期固化.
//! - **分组清晰**: 按模块 (`icon` / `window` / `search` / `fs` / `paths` / `ipc_events`)
//!   组织, 跨模块复用通过子模块嵌套 (如 `icon::blank_detection::*`).
//!
//! 使用方式:
//! ```rust
//! // 系统路径前缀查表
//! let _prefixes = monotools_lib::core::config::paths::SYSTEM_PATH_PREFIXES;
//! // 事件名集中管理
//! let _evt = monotools_lib::core::config::ipc_events::INDEX_PROGRESS;
//! ```

pub mod icon {
    /// 图标像素尺寸. 与前端 `ICON_CONFIG.size` 对齐.
    /// 使用 256x256 获取高质量图标, 前端可按需缩放显示.
    pub const SIZE: i32 = 256;

    /// 后端 base64 最短长度 (字符数).
    /// 256x256 RGBA PNG 压缩后通常 1000-5000 chars, 256 是安全下限.
    pub const MIN_BASE64_LEN: usize = 256;

    /// PNG magic 89 50 4E 47 0D 0A 1A 0A 的 base64 编码.
    pub const PNG_MAGIC_BASE64: &str = "iVBORw0KGgo";

    /// 256x256 RGBA 图标的字节数 (4 通道).
    /// 用于空白检测时判断 buffer 是否完整.
    pub const BUF_SIZE: usize = 256 * 256 * 4;

    /// 空白/单色图标检测阈值 —— icon.rs::is_blank_icon 用.
    /// 用于过滤"图标被提取了但实际上是全黑/全白/纯灰, 不应展示"的场景.
    pub mod blank_detection {
        /// 最少有效像素数 (非透明像素).
        /// 低于此值说明图标几乎没有内容, 判为空白.
        /// 256x256 = 65536 像素, 16x16 = 256 像素, 设为 50 比较安全.
        pub const MIN_VALID_PIXELS: u64 = 50;

        /// "富色彩" 图标的最低独立颜色数.
        pub const COLOR_COUNT_RICH: usize = 16;
        /// "富色彩" 图标的最低 luma 标准差 (反映色彩丰富度).
        pub const LUMA_STD_RICH: f64 = 16.0;
        /// "中等" 图标的最低 luma 标准差.
        pub const LUMA_STD_MID: f64 = 4.0;
        /// "中等" 图标的最低独立颜色数.
        pub const COLOR_COUNT_MID: usize = 5;
    }

    /// 自动裁剪并居中配置 —— icon.rs::autocrop_and_center 用.
    ///
    /// 解决 "图标只有左上角一点点" 的问题:
    /// - 当有效内容占比低于 MIN_AREA_RATIO 时触发自动裁剪
    /// - 裁剪后放大到 MAX_SCALE_RATIO (相对于整张图的比例), 居中放置
    /// - 保持宽高比, 双线性插值
    pub mod autocrop {
        /// 触发自动裁剪的最小内容占比 (0.0 ~ 1.0).
        /// 内容占比低于此值 → 裁剪并放大.
        /// 降低到 0.35: 大多数 Windows 图标自带 padding, 内容通常占 40-70%
        /// 画布. 0.6 阈值过高导致很多图标不触发放大, 缩到 32px 后显得太小.
        /// 0.35 确保只有内容确实很小的图标才被放大, 避免过度裁剪正常图标.
        pub const MIN_AREA_RATIO: f32 = 0.35;

        /// 裁剪后内容相对于整张图的最大比例 (0.0 ~ 1.0).
        /// 0.92 意味着: 内容最大放大到 92% 的尺寸, 周围留一点 padding,
        /// 避免贴边显得太挤. 比旧值 0.88 略大, 让图标更饱满.
        pub const MAX_SCALE_RATIO: f32 = 0.92;
    }
}

pub mod window {
    /// 主窗口默认宽度 (像素).
    pub const DEFAULT_WIDTH: f64 = 720.0;
    /// 主窗口最小高度.
    pub const MIN_HEIGHT: u32 = 320;
    /// 主窗口最大高度.
    pub const MAX_HEIGHT: u32 = 680;
}

pub mod search {
    /// 文件搜索引擎空查询返回上限 (实际命中数, 防止索引极大时单帧 IPC 阻塞).
    pub const ALL_FILES_EMPTY_QUERY_CAP: u32 = 500;

    /// 默认搜索返回条数上限. 与前端 `SEARCH_LIMITS.defaultLimit` 同步.
    pub const DEFAULT_LIMIT: u32 = 200;
    /// 空查询时返回条数上限. 与前端 `SEARCH_LIMITS.emptyQueryLimit` 同步.
    pub const EMPTY_QUERY_LIMIT: u32 = 2000;
    /// 最大允许返回条数. 与前端 `SEARCH_LIMITS.maxLimit` 同步.
    pub const MAX_LIMIT: u32 = 2000;

    /// 应用索引扫描目录最大深度.
    pub const APP_SCAN_MAX_DEPTH: usize = 5;
    /// 空查询时应用的兜底分数.
    pub const APP_EMPTY_QUERY_SCORE: f32 = 0.5;
    /// launch_count 在最终分数中的权重.
    pub const APP_LAUNCH_COUNT_WEIGHT: f32 = 0.5;
    /// 应用搜索评分: 精确匹配得分.
    pub const APP_SCORE_EXACT: f32 = 100.0;
    /// 应用搜索评分: 前缀匹配得分.
    pub const APP_SCORE_PREFIX: f32 = 80.0;
    /// 应用搜索评分: 子串匹配得分.
    pub const APP_SCORE_SUBSTR: f32 = 50.0;
    /// 应用搜索评分: 模糊匹配得分.
    pub const APP_SCORE_FUZZY: f32 = 20.0;
    /// 应用搜索评分: token 命中得分.
    pub const APP_SCORE_TOKEN: f32 = 5.0;

    /// 拼音搜索评分: 首字母命中得分 (例: "wj" → "微信").
    /// 低于 SUBSTR (50) 但高于 FUZZY (20): 拼音是辅助匹配, 不应超过名称子串.
    pub const PINYIN_SCORE_INITIALS: f32 = 30.0;
    /// 拼音搜索评分: 完整拼音命中得分 (例: "weixin" → "微信").
    /// 与 SUBSTR 持平: 全拼和子串同等可信.
    pub const PINYIN_SCORE_FULL: f32 = 50.0;

    /// SearchCategory 权重 (在 search::apply_category_weight 中使用).
    pub const CATEGORY_WEIGHT_APPS: f32 = 0.8;
    pub const CATEGORY_WEIGHT_COMMANDS: f32 = 1.2;
    pub const CATEGORY_WEIGHT_FILES: f32 = 1.0;

    /// 模糊匹配 title 权重 (fuzzy score * 0.3 / 0.1).
    pub const FUZZY_TITLE_WEIGHT: f32 = 0.3;
    /// 模糊匹配 subtitle 权重.
    pub const FUZZY_SUBTITLE_WEIGHT: f32 = 0.1;
    /// 模糊匹配在 search 阶段对原 score 的折扣系数.
    pub const FUZZY_BASE_SCORE_KEEP: f32 = 0.6;
    /// 模糊匹配 title 归一化系数 (skimmer / 100).
    pub const FUZZY_TITLE_NORM: f32 = 100.0;
    /// 模糊匹配 subtitle 归一化系数 (skimmer / 200, 比 title 弱).
    pub const FUZZY_SUBTITLE_NORM: f32 = 200.0;
}

pub mod fs {
    /// 文件索引 SQLite DB 名.
    pub const DB_NAME: &str = "monotools_file_index.db";
    /// 当前 schema 版本 (DB 升级时 +1).
    pub const SCHEMA_VERSION: i64 = 9;
    /// SQLite PRAGMA page_size.
    pub const PAGE_SIZE: u32 = 4096;
    /// SQLite PRAGMA cache_size (负数 = KB). 256MB → -262144; 历史值 -65536 = 64MB.
    pub const CACHE_SIZE_KB: i64 = -262144;
    /// SQLite PRAGMA mmap_size (字节). 64 MB.
    pub const MMAP_SIZE_BYTES: i64 = 67_108_864;
    /// SQLite PRAGMA wal_autocheckpoint (页数).
    pub const WAL_AUTOCHECKPOINT: i32 = 5000;
    /// USN 扫描 / 索引批量插入 chunk 大小 (避免一次 insert 太多导致内存爆).
    pub const INDEX_CHUNK_SIZE: usize = 50_000;
    /// required_page_size (用于 schema 升级校验).
    pub const REQUIRED_PAGE_SIZE: i64 = 4096;

    /// 文件索引扫描时跳过的文件/目录名 (小写匹配).
    /// 注意: .git / .vscode **不在此列表**, 因为 should_skip_path 的隐藏文件规则
    /// 把它们作为例外保留, 用户经常想搜到这些目录下的文件.
    pub const SKIP_NAMES: &[&str] = &["thumbs.db", "desktop.ini", "pagefile.sys", "hiberfil.sys"];

    /// 文件索引扫描时跳过的路径片段 (小写子串匹配).
    pub const SKIP_PATH_FRAGMENTS: &[&str] = &[
        "\\windows\\winsxs",
        "\\windows\\system32\\config",
        "\\windows\\softwaredistribution",
        "\\$recycle.bin",
        "\\system volume information",
    ];
}

pub mod paths {
    /// ProgramData 下的开始菜单 Programs 路径片段.
    pub const COMMON_START_MENU: &str = "Microsoft\\Windows\\Start Menu\\Programs";
    /// APPDATA (roaming) 下的开始菜单 Programs 路径片段.
    pub const USER_START_MENU: &str = "Microsoft\\Windows\\Start Menu\\Programs";
    /// 用户桌面目录名.
    pub const USER_DESKTOP: &str = "Desktop";
    /// Windows 资源管理器.
    pub const EXPLORER_EXE: &str = "explorer.exe";
    /// `CreateProcess` CREATE_NO_WINDOW 标志位.
    pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    /// 系统应用路径前缀 (用于 app_type_of 判定).
    pub const SYSTEM_PATH_PREFIXES: &[&str] =
        &["c:\\windows\\", "c:\\program files\\windowsapps\\"];
}

pub mod ipc_events {
    /// 索引进度事件 (Tauri app.emit).
    pub const INDEX_PROGRESS: &str = "index_progress";
    /// 前端 ready 事件.
    pub const FRONTEND_READY: &str = "frontend_ready";
}
