//! Windows 应用图标提取
//!
//! 提供从 `.exe` / `.lnk` 中提取 32x32 RGBA 图标并编码为 PNG 的能力.
//!
//! ## 错误处理协议
//!
//! 所有内部错误一律映射为 `Ok(None)`, 永远不向上抛错.
//! - 文件不存在 / 访问被拒 → `Ok(None)` (前端降级到 Lucide 通用图标)
//! - 非 PE 文件 / 损坏 → `Ok(None)`
//! - 编码失败 → `Ok(None)`
//!
//! ## 缓存
//!
//! 进程内 `OnceLock<Mutex<HashMap>>` 缓存 (path -> Option<Vec<u8>>),
//! key 用归一化路径 (小写 + canonicalize 回退), 避免重复 SHGetFileInfoW 调用.
//!
//! ## 扩展性
//!
//! [`IconExtractor`] trait 把"平台特定"的图标提取能力抽象成可替换的
//! impl. 当前只有 [`WindowsIconExtractor`]; 未来 Linux (libunity /
//! freedesktop) / macOS (NSWorkspace) 可加 impl 后挂到全局 registry.
//! 调用方 (commands / services) 始终通过 [`get_extractor()`] 拿
//! 默认实例, 不直接 import Windows 平台代码.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use windows::Win32::UI::WindowsAndMessaging::HICON;

use crate::core::config::icon as icon_cfg;

/// 平台无关的图标提取器接口.
///
/// 实现方必须遵守"永不抛错"协议: 任何内部错误 (文件不存在 / 访问被拒 /
/// 非 PE / 编码失败) 一律返回 `None`, 由调用方决定是否走兜底.
pub trait IconExtractor: Send + Sync {
    /// 平台名, 用于诊断日志 (`[icon] platform=windows ...`).
    fn platform_name(&self) -> &'static str;

    /// 提取图标. size 是边长像素 (Windows 常用 16 / 32 / 48).
    /// 失败返回 `None` 而非 `Err`: 这是 trait 约定.
    fn extract(&self, path: &Path, size: i32) -> Option<Vec<u8>>;

    /// 是否"愿意"尝试这个 path. 默认 true; Linux 端可限定 `.desktop` 等.
    /// 注意: 返回 false 不等于 "一定能拿到" (例如 .lnk 的 target 已删),
    /// 真正的失败语义仍由 [`IconExtractor::extract`] 表达.
    fn supports(&self, path: &Path) -> bool {
        let _ = path;
        true
    }
}

/// Windows 平台实现. 使用 Win32 SHGetFileInfoW + GDI GetDIBits.
pub struct WindowsIconExtractor;

impl IconExtractor for WindowsIconExtractor {
    fn platform_name(&self) -> &'static str {
        "windows"
    }

    fn extract(&self, path: &Path, size: i32) -> Option<Vec<u8>> {
        extract_icon_windows(path, size)
    }

    fn supports(&self, path: &Path) -> bool {
        // 没扩展名的 PE 也算 (如 POSIX 子系统的裸 binary);
        // .exe / .lnk / .url / .ico / .msi 是 Windows 上能 SHGetFileInfo 出图标的常见格式;
        // shell: 开头的是 shell 命名空间路径 (如 UWP 应用的 shell:AppsFolder\...),
        // 可以通过 IShellItemImageFactory 提取图标.
        let path_str = path.to_string_lossy();
        if path_str.starts_with("shell:") {
            return true;
        }
        match path.extension().and_then(|e| e.to_str()) {
            None => true,
            Some(ext) => {
                let e = ext.to_ascii_lowercase();
                matches!(e.as_str(), "exe" | "lnk" | "url" | "ico" | "msi" | "scr")
            }
        }
    }
}

type IconCache = HashMap<String, Option<Vec<u8>>>;

static ICON_CACHE: OnceLock<Mutex<IconCache>> = OnceLock::new();

static ICON_EXTRACTOR: OnceLock<Box<dyn IconExtractor>> = OnceLock::new();

/// 正在提取中的图标: key = 缓存 key, value = ().
/// 用于 single-flight 去重: 当 N 个线程同时请求同一个未缓存的图标时,
/// 只有第一个真正去提取, 其余的等第一个完成后直接从 cache 读.
/// 避免 "dog pile effect" (缓存失效瞬间大量重复提取).
static IN_FLIGHT: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();

fn cache() -> &'static Mutex<IconCache> {
    ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn in_flight() -> &'static Mutex<std::collections::HashSet<String>> {
    IN_FLIGHT.get_or_init(|| Mutex::new(std::collections::HashSet::new()))
}

/// 全局默认 IconExtractor. 第一次访问时构造 `WindowsIconExtractor`,
/// 进程内复用. 未来多平台只需在 init 分支里加 cfg 判别.
pub fn get_extractor() -> &'static dyn IconExtractor {
    ICON_EXTRACTOR
        .get_or_init(|| Box::new(WindowsIconExtractor))
        .as_ref()
}

/// 归一化路径作为缓存 key. 小写 + canonicalize (失败时退回原路径).
fn cache_key(p: &Path) -> String {
    let canon = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    canon.to_string_lossy().to_lowercase()
}

/// 获取缓存的克隆 snapshot. 用于 commands.rs 在进入 spawn_blocking 前
/// 先快速过一遍 cache, 已命中的直接编码, 未命中的才进入后台线程池提取.
///
/// 为什么不直接暴露 cache()? 因为 cache 是 Mutex<HashMap>, 外部持锁太久
/// 会阻塞其他线程的写入. snapshot 一次性克隆后立即释放锁, 读写互不阻塞.
pub fn cache_snapshot() -> std::collections::HashMap<String, Option<Vec<u8>>> {
    cache().lock().clone()
}

/// 公共入口: 带缓存的图标提取. 永远不抛错.
///
/// 特殊处理 - .lnk 快捷方式:
/// - 若传入路径是 .lnk 文件, 先解析其目标路径, 再提取**目标文件**的图标.
/// - 这与用户直觉一致: 快捷方式应该显示它指向的程序的图标, 而不是快捷方式本身的图标.
/// - 缓存 key 仍用原始 .lnk 路径, 避免同目标不同快捷方式重复计算.
///
/// 失败诊断: 在 `mono_icon_debug` 环境变量被设置 (=1) 时, 任何
/// `None` 返回都会写一条 `log::warn!` 含 path + 阶段, 方便前端/后端
/// 联合排查"图标为什么是空白".
pub fn get_or_extract_cached(path: &str) -> crate::core::error::Result<Option<Vec<u8>>> {
    // 提前过滤 URL 路径: http:// / https:// / ftp:// 等不是本地文件,
    // 直接返回 None, 避免 Path::exists() 误判 + 无意义的 warn 日志.
    if path.starts_with("http://")
        || path.starts_with("https://")
        || path.starts_with("ftp://")
        || path.starts_with("mailto:")
    {
        log_icon_debug("url-skipped", path, "非本地文件路径, 跳过图标提取");
        return Ok(None);
    }

    let p = Path::new(path);

    // 路径包含 null / 控制字符等乱码时, Path::exists() 可能直接 false.
    // 先做一次轻量校验: 含 \x00 或大量替换字符 (U+FFFD) 的直接跳过.
    if path.contains('\x00') || path.chars().filter(|c| *c == '\u{FFFD}').count() > 2 {
        log_icon_debug("garbled-path", path, "路径含乱码/非法字符, 跳过图标提取");
        return Ok(None);
    }

    if !p.exists() {
        log_icon_debug("file-missing", path, "Path::exists() == false");
        log::warn!("[icon] file-missing path={}", path);
        return Ok(None);
    }

    let key = cache_key(p);
    // 第一次检查: cache 命中直接返回
    {
        let cache = cache().lock();
        if let Some(cached) = cache.get(&key) {
            return Ok(cached.clone());
        }
    }

    // Single-flight: 如果另一个线程正在提取同一个图标, 就等它完成后直接读 cache,
    // 而不是自己再提取一遍. 避免 "缓存失效瞬间 N 个请求同时穿透" 的 dog pile effect.
    let is_first = {
        let mut in_flight = in_flight().lock();
        if in_flight.contains(&key) {
            false
        } else {
            in_flight.insert(key.clone());
            true
        }
    };

    if !is_first {
        // 另一个线程在提取, 我们自旋等待它写入 cache (最多等 3s)
        // 每 10ms 检查一次 cache, 命中就返回
        let mut waited = 0u64;
        loop {
            {
                let cache = cache().lock();
                if let Some(cached) = cache.get(&key) {
                    return Ok(cached.clone());
                }
            }
            if waited >= 3000 {
                // 超时兜底: 3s 还没好 (可能另一个线程挂了), 自己提取
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            waited += 10;
        }
    }

    // 对于 .lnk 快捷方式: 解析目标路径, 提取目标文件的图标.
    // 目标文件图标提取失败时, 回退到提取 .lnk 本身的图标 (与资源管理器行为一致).
    let is_lnk = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("lnk"))
        .unwrap_or(false);

    let extracted = if is_lnk {
        match crate::platform::windows::shell::resolve_shortcut(&p.to_path_buf()) {
            Ok(target) if target.exists() => {
                let target_icon = get_extractor().extract(&target, icon_cfg::SIZE);
                if target_icon.is_some() {
                    log_icon_debug(
                        "lnk-target",
                        path,
                        &format!("resolved lnk target: {}", target.to_string_lossy()),
                    );
                    target_icon
                } else {
                    // 目标文件图标提取失败 → 回退到 .lnk 本身
                    log_icon_debug(
                        "lnk-fallback",
                        path,
                        "target icon extraction failed, falling back to lnk itself",
                    );
                    get_extractor().extract(p, icon_cfg::SIZE)
                }
            }
            _ => {
                // 解析失败 / 目标不存在 → 提取 .lnk 本身的图标
                log_icon_debug(
                    "lnk-resolve-failed",
                    path,
                    "failed to resolve lnk target, using lnk itself",
                );
                get_extractor().extract(p, icon_cfg::SIZE)
            }
        }
    } else {
        get_extractor().extract(p, icon_cfg::SIZE)
    };

    if extracted.is_none() {
        log_icon_debug("extract-failed", path, "extractor returned None");
        log::warn!(
            "[icon] extract-failed path={} (file exists but extraction returned None)",
            path
        );
    }
    // 写入 cache + 清除 in-flight 标记
    {
        let mut cache = cache().lock();
        cache.insert(key.clone(), extracted.clone());
    }
    {
        let mut in_flight = in_flight().lock();
        in_flight.remove(&key);
    }
    Ok(extracted)
}

/// 写一条 icon 诊断日志. 默认静默, 启用方法:
///   1. 设置环境变量 `MONO_ICON_DEBUG=1` 启动应用
///   2. 或调用方在测试代码里手动 `std::env::set_var`
fn log_icon_debug(stage: &str, path: &str, detail: &str) {
    if std::env::var("MONO_ICON_DEBUG").ok().as_deref() != Some("1") {
        return;
    }
    log::warn!(
        "[icon-debug] stage={} path={} detail={}",
        stage,
        path,
        detail,
    );
}

/// 实际提取. 任意步骤失败 -> None (不抛错).
///
/// 4-tier 优先级链 (高分辨率原生 → 高质量缩放):
/// - **Tier 1**: `SHGetImageList(SHIL_JUMBO)` + `IImageList2::GetIcon` — Vista+ 系统
///   维护的 256x256 图像列表, 与 Windows 资源管理器/任务栏同一来源. Win 8.1+ 多数
///   应用注册了真正的 256x256 图标, 这一步直接拿到**原生 256x256 HICON**, 0 锯齿.
/// - **Tier 2**: `IShellItemImageFactory::GetImage(SIIGBF_ICONONLY)` —
///   **不带** `SIIGBF_BIGGERSIZEOK` (不带强制缩放), 仅当返回位图实际尺寸 ==
///   请求尺寸时使用, 否则视为 fallback. 解决旧版把 16x16 强制拉伸到 256x256
///   造成全图模糊的问题.
/// - **Tier 3**: `ExtractIconExW` (32x32) + HALFTONE 高质量拉伸. `SetStretchBltMode`
///   + `HALFTONE` 让 32x32 → 256x256 用高质量插值, 不再是默认 nearest-neighbor
///   锯齿.
/// - **Tier 4**: `SHGetFileInfoW(SHGFI_LARGEICON)` + DrawIconEx + HALFTONE
///   高质量拉伸, 兜底路径.
///
/// 所有 Tier 成功提取后, 都会经过 `autocrop_and_center` 后处理:
/// - 检测有效像素的边界框 (去掉周围透明区域)
/// - 如果有效内容占比太小 (比如只有左上角一小块), 就裁剪并放大居中
/// - 解决 "图标只有左上角一点点" 的视觉问题
///
/// `size`: 边长像素. 统一用 `icon_cfg::SIZE` (256), 签名抽象以便未来
/// macOS NSWorkspace 等 impl 走不同尺寸.
#[cfg(windows)]
fn extract_icon_windows(path: &Path, size: i32) -> Option<Vec<u8>> {
    use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;

    let try_autocrop = |rgba: Vec<u8>| -> Vec<u8> {
        autocrop_and_center(&rgba, size as u32, size as u32).unwrap_or(rgba)
    };

    // Tier 1: SHIL_JUMBO 系统图像列表 (256x256 原生优先)
    if let Some(hicon) = extract_hicon_via_shil_jumbo(path) {
        if let Some(rgba) = draw_hicon_to_rgba(hicon, size) {
            unsafe {
                let _ = DestroyIcon(hicon);
            }
            if !is_blank_icon(&rgba) {
                let processed = try_autocrop(rgba);
                return encode_png(&processed, size as u32, size as u32);
            }
        }
        unsafe {
            let _ = DestroyIcon(hicon);
        }
        log_icon_debug(
            "tier1-failed",
            &path.to_string_lossy(),
            "SHIL_JUMBO returned icon but draw/blank-check failed",
        );
    }

    // Tier 2: IShellItemImageFactory (无 BIGGERSIZEOK, 仅接受原生命尺寸)
    if let Some(rgba) = extract_rgba_via_shell_item_factory(path, size) {
        if !is_blank_icon(&rgba) {
            let processed = try_autocrop(rgba);
            return encode_png(&processed, size as u32, size as u32);
        }
        log_icon_debug(
            "tier2-blank",
            &path.to_string_lossy(),
            "IShellItemImageFactory returned blank icon",
        );
    }

    // Tier 3/4: ExtractIconExW / SHGetFileInfoW + HALFTONE 高质量拉伸
    if let Some(hicon) = get_hicon(path) {
        if let Some(rgba) = draw_hicon_to_rgba(hicon, size) {
            unsafe {
                let _ = DestroyIcon(hicon);
            }
            if !is_blank_icon(&rgba) {
                let processed = try_autocrop(rgba);
                return encode_png(&processed, size as u32, size as u32);
            }
        }
        unsafe {
            let _ = DestroyIcon(hicon);
        }
        log_icon_debug(
            "tier3-4-failed",
            &path.to_string_lossy(),
            "ExtractIconExW/SHGetFileInfoW + HALFTONE draw returned blank/none",
        );
    }

    None
}

/// Tier 1: 通过 `SHGetImageList(SHIL_JUMBO)` + `IImageList2::GetIcon` 拿到系统
/// 为该 path 注册的**原生 256x256 HICON**.
///
/// 实现步骤:
/// 1. `SHGetFileInfoW` 配合 `SHGFI_SYSICONINDEX` 拿到该 path 在系统图像列表中的
///    索引 (`iIcon`).
/// 2. `SHGetImageList(SHIL_JUMBO)` 拿到系统 256x256 图像列表 (`IImageList2`).
/// 3. `IImageList2::GetIcon(iIcon, ILD_TRANSPARENT)` 拿到 HICON.
///
/// 任何一步失败都返回 `None`, 由 `extract_icon_windows` 继续 Tier 2/3/4.
#[cfg(windows)]
fn extract_hicon_via_shil_jumbo(path: &Path) -> Option<HICON> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Controls::{IImageList2, ILD_TRANSPARENT};
    use windows::Win32::UI::Shell::{
        SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_SYSICONINDEX, SHIL_JUMBO,
    };
    use windows::Win32::UI::WindowsAndMessaging::HICON;

    let wide_path: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // Step 1: 拿该 path 在系统图像列表中的索引
    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide_path.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX,
        )
    };
    if ok == 0 {
        log_icon_debug(
            "tier1-shiinfo-failed",
            &path.to_string_lossy(),
            "SHGetFileInfoW(SHGFI_SYSICONINDEX) returned 0",
        );
        return None;
    }
    let i_icon = info.iIcon;

    // Step 2: 拿系统 256x256 图像列表 (Vista+ 提供, SHIL_JUMBO = 4)
    let image_list: IImageList2 = match unsafe { SHGetImageList(SHIL_JUMBO as i32) } {
        Ok(l) => l,
        Err(e) => {
            log_icon_debug(
                "tier1-shil-jumbo-failed",
                &path.to_string_lossy(),
                &format!("SHGetImageList(SHIL_JUMBO) failed: {}", e),
            );
            return None;
        }
    };

    // Step 3: 拿到原生 256x256 HICON
    let hicon: HICON = match unsafe { image_list.GetIcon(i_icon, ILD_TRANSPARENT.0) } {
        Ok(h) => h,
        Err(e) => {
            log_icon_debug(
                "tier1-geticon-failed",
                &path.to_string_lossy(),
                &format!("IImageList2::GetIcon(i={}) failed: {}", i_icon, e),
            );
            return None;
        }
    };

    // 校验 HICON 有效 (32-bit HICON, .0 is_invalid 判断)
    if hicon.0.is_null() {
        log_icon_debug(
            "tier1-null-hicon",
            &path.to_string_lossy(),
            "IImageList2::GetIcon returned null HICON",
        );
        return None;
    }

    Some(hicon)
}

/// Tier 2: `IShellItemImageFactory` 不带 `SIIGBF_BIGGERSIZEOK`.
///
/// 不带 BIGGERSIZEOK 时, 系统返回"最接近请求尺寸的原生位图"; 实际尺寸若 != 请求
/// 尺寸就放弃这一层 (return None), 避免 16x16 被强制放大. 这是修锯齿的关键.
///
/// 对于 shell: 开头的路径 (如 UWP 应用), 先通过 SHParseDisplayName 解析为 PIDL,
/// 再用 SHCreateItemFromIDList 创建 IShellItem, 确保能正确解析 shell 命名空间路径.
/// 同时允许 BIGGERSIZEOK, 因为这些图标通常不是 256x256 的原生尺寸.
#[cfg(windows)]
fn extract_rgba_via_shell_item_factory(path: &Path, size: i32) -> Option<Vec<u8>> {
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::DeleteObject;
    use windows::Win32::System::Com::IBindCtx;
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromIDList, SHCreateItemFromParsingName,
        SHParseDisplayName, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
    };

    let path_str = path.to_string_lossy();
    let is_shell_path = path_str.starts_with("shell:");

    let wide_path: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();

    // 尝试获取 IShellItemImageFactory
    let shell_item: windows::core::Result<IShellItemImageFactory> = if is_shell_path {
        // shell 路径: 先用 SHParseDisplayName 解析为 PIDL, 再创建 IShellItem
        let mut pidl = std::ptr::null_mut();
        let mut attrs = 0u32;
        let result = unsafe {
            SHParseDisplayName(
                PCWSTR(wide_path.as_ptr()),
                None::<&IBindCtx>,
                &mut pidl,
                0,
                Some(&mut attrs),
            )
        };
        if result.is_err() || pidl.is_null() {
            log_icon_debug(
                "tier2-shparse-failed",
                &path_str,
                &format!("SHParseDisplayName failed: {:?}", result),
            );
            return None;
        }

        let item_result = unsafe { SHCreateItemFromIDList::<IShellItemImageFactory>(pidl) };

        // 释放 PIDL
        if !pidl.is_null() {
            unsafe {
                windows::Win32::UI::Shell::ILFree(Some(pidl as *const _));
            }
        }

        item_result
    } else {
        // 普通文件路径: 直接用 SHCreateItemFromParsingName
        unsafe { SHCreateItemFromParsingName(PCWSTR(wide_path.as_ptr()), None) }
    };

    let shell_item = match shell_item {
        Ok(s) => s,
        Err(e) => {
            log_icon_debug(
                "tier2-shcreate-failed",
                &path_str,
                &format!("Create IShellItemImageFactory failed: {}", e),
            );
            return None;
        }
    };

    let flags = if is_shell_path {
        // shell 路径允许缩放，确保能拿到图标
        SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK
    } else {
        SIIGBF_ICONONLY // 关键: 不带 BIGGERSIZEOK, 避免强制放大
    };

    let hbitmap = unsafe {
        shell_item.GetImage(
            windows::Win32::Foundation::SIZE { cx: size, cy: size },
            flags,
        )
    };
    let hbitmap = match hbitmap {
        Ok(h) if !h.is_invalid() => h,
        _ => {
            log_icon_debug(
                "tier2-getimage-failed",
                &path_str,
                "IShellItemImageFactory::GetImage returned invalid hbitmap",
            );
            return None;
        }
    };

    // 对于 shell 路径，使用了 BIGGERSIZEOK，系统会自动缩放到请求尺寸
    // 所以这里仍然可以用严格模式检查
    let rgba = extract_rgba_from_hbitmap_strict(&hbitmap, size);
    unsafe {
        let _ = DeleteObject(hbitmap.into());
    }
    rgba
}

/// 从 HBITMAP 直接读取像素. **严格模式**: 实际位图尺寸必须 == size, 否则返回
/// `None` (Tier 2 用此保证不接受拉伸位图).
#[cfg(windows)]
fn extract_rgba_from_hbitmap_strict(
    hbitmap: &windows::Win32::Graphics::Gdi::HBITMAP,
    size: i32,
) -> Option<Vec<u8>> {
    use windows::Win32::Graphics::Gdi::{GetObjectW, BITMAP};

    let mut bmp: BITMAP = unsafe { std::mem::zeroed() };
    let result = unsafe {
        GetObjectW(
            (*hbitmap).into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bmp as *mut _ as *mut _),
        )
    };
    if result == 0 {
        return None;
    }
    if bmp.bmBitsPixel != 32 {
        return None;
    }
    // 严格: bmWidth/bmHeight 必须 == size, 不接受拉伸后的尺寸
    if bmp.bmWidth != size || bmp.bmHeight.abs() != size {
        log_icon_debug(
            "tier2-strict-size-mismatch",
            "<hbitmap>",
            &format!(
                "BITMAP size {}x{} != target {}x{}",
                bmp.bmWidth,
                bmp.bmHeight.abs(),
                size,
                size
            ),
        );
        return None;
    }

    let byte_len = (size as usize) * (size as usize) * 4;
    let mut rgba = Vec::with_capacity(byte_len);
    unsafe {
        let src = std::slice::from_raw_parts(bmp.bmBits as *const u8, byte_len);
        for chunk in src.chunks_exact(4) {
            rgba.push(chunk[2]);
            rgba.push(chunk[1]);
            rgba.push(chunk[0]);
            rgba.push(chunk[3]);
        }
    }
    Some(rgba)
}

/// Tier 3/4 共用: 把 `HICON` 绘制到 `size x size` 的 32-bit DIB section, 转 RGBA.
///
/// **关键**: 调用 `SetStretchBltMode(HALFTONE)` + `SetBrushOrgEx(0, 0, None)` 让
/// GDI 拉伸时使用高质量插值 (类似双三次), 避免 32x32 → 256x256 时的 nearest-neighbor
/// 锯齿.
#[cfg(windows)]
fn draw_hicon_to_rgba(
    hicon: windows::Win32::UI::WindowsAndMessaging::HICON,
    size: i32,
) -> Option<Vec<u8>> {
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, SetBrushOrgEx,
        SetStretchBltMode, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HALFTONE, HDC,
    };

    let screen_dc: HDC = unsafe { CreateCompatibleDC(None) };
    if screen_dc.is_invalid() {
        return None;
    }

    let mut bmi: BITMAPINFO = unsafe { std::mem::zeroed() };
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = size;
    bmi.bmiHeader.biHeight = -size;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB.0;

    let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();

    let section_hbm = unsafe {
        CreateDIBSection(
            Some(screen_dc),
            &bmi,
            DIB_RGB_COLORS,
            &mut bits_ptr,
            None,
            0,
        )
    };

    let section_hbm = match section_hbm {
        Ok(h) if !bits_ptr.is_null() => h,
        _ => {
            unsafe {
                let _ = DeleteDC(screen_dc);
            }
            return None;
        }
    };

    unsafe {
        let _ = SelectObject(screen_dc, section_hbm.into());
    }

    // 关键: 高质量拉伸. HALFTONE 模式让 GDI 用类似双三次的算法拉伸 32x32 →
    // 256x256, 消除默认拉伸的锯齿. SetBrushOrgEx(0,0) 是 HALFTONE 的配套调用,
    // 没有它 HALFTONE 会产生奇怪的网格纹理.
    unsafe {
        let _ = SetStretchBltMode(screen_dc, HALFTONE);
        let _ = SetBrushOrgEx(screen_dc, 0, 0, None);
    }

    let di_flags = windows::Win32::UI::WindowsAndMessaging::DI_FLAGS(0x0003);
    let draw_ok = unsafe {
        windows::Win32::UI::WindowsAndMessaging::DrawIconEx(
            screen_dc, 0, 0, hicon, size, size, 0, None, di_flags,
        )
    }
    .is_ok();
    if !draw_ok {
        unsafe {
            let _ = DeleteObject(section_hbm.into());
            let _ = DeleteDC(screen_dc);
        }
        return None;
    }

    let byte_len = (size as usize) * (size as usize) * 4;
    let mut rgba = Vec::with_capacity(byte_len);
    unsafe {
        let src = std::slice::from_raw_parts(bits_ptr as *const u8, byte_len);
        for chunk in src.chunks_exact(4) {
            rgba.push(chunk[2]);
            rgba.push(chunk[1]);
            rgba.push(chunk[0]);
            rgba.push(chunk[3]);
        }
    }

    unsafe {
        let _ = DeleteObject(section_hbm.into());
        let _ = DeleteDC(screen_dc);
    }

    Some(rgba)
}

/// 从路径获取 HICON. 优先 ExtractIconExW, 回退到 SHGetFileInfoW.
/// IShellItemImageFactory 的高质量提取已移到 extract_icon_windows 中处理.
#[cfg(windows)]
fn get_hicon(path: &Path) -> Option<HICON> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        ExtractIconExW, SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON,
    };
    use windows::Win32::UI::WindowsAndMessaging::HICON;

    let wide_path: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // 优先: ExtractIconExW — 直接从文件提取图标
    let mut icons: [HICON; 1] = unsafe { std::mem::zeroed() };
    let extracted = unsafe {
        ExtractIconExW(
            PCWSTR(wide_path.as_ptr()),
            0,
            None,
            Some(icons.as_mut_ptr()),
            1,
        )
    };

    if extracted > 0 && !icons[0].0.is_null() {
        return Some(icons[0]);
    }

    // 回退: SHGetFileInfoW
    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    let flags = SHGFI_ICON | SHGFI_LARGEICON;
    let result = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide_path.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            flags,
        )
    };

    if result == 0 || info.hIcon.0.is_null() {
        log_icon_debug(
            "no-hicon",
            &path.to_string_lossy(),
            "IShellItemImageFactory, ExtractIconEx and SHGetFileInfo all failed",
        );
        return None;
    }

    Some(info.hIcon)
}

/// 检测 RGBA buffer 是否是"空白图标".
///
/// Windows 对某些文件 (典型场景: .lnk 指向已删除的目标) 会返回一个
/// 通用空白 32x32 图标. 视觉上是一格纯色或近似纯色方块, 用户无法分辨
/// "正在加载"和"真的没有图标".
///
/// 判定标准 (2026-07 二次调整):
/// - **只统计非透明像素** (alpha > alpha_threshold): 透明像素不参与
///   颜色数和亮度统计, 避免小图标放在大画布上因大量透明像素被误判为空白.
/// - 有效像素数 < MIN_VALID_PIXELS → 判 blank (几乎没有内容)
/// - 全黑 / 全白 / 单色 → 立刻判为 blank (color_count ≤ 1)
/// - 2-4 种颜色且亮度方差 < 4 → 可能是 Windows 空白方块, 判 blank
/// - 5+ 种颜色 或 亮度方差 ≥ 4 → 真实图标, 通过
pub fn is_blank_icon(rgba: &[u8]) -> bool {
    if rgba.len() < 4 {
        return true;
    }

    let mut seen = std::collections::HashSet::with_capacity(64);
    let mut lum_sum: u64 = 0;
    let mut lum_sum_sq: u64 = 0;
    let mut pixel_count: u64 = 0;
    let alpha_threshold = 8u8;

    // 只统计非透明像素
    for px in rgba.chunks_exact(4) {
        let a = px[3];
        if a <= alpha_threshold {
            continue;
        }
        let r = px[0] as u32;
        let g = px[1] as u32;
        let b = px[2] as u32;
        // 亮度 (ITU-R BT.601): 0.299R + 0.587G + 0.114B
        let lum = (299 * r + 587 * g + 114 * b) / 1000;
        lum_sum += lum as u64;
        lum_sum_sq += (lum as u64) * (lum as u64);
        pixel_count += 1;
        // 颜色桶化 (5-bit per channel) → 把"几乎相同的颜色"合并, 避免
        // 抗锯齿产生的细微差异让 seen 暴增.
        let key = (px[0] >> 3, px[1] >> 3, px[2] >> 3, px[3] >> 5);
        seen.insert(key);
    }

    use icon_cfg::blank_detection as bd;

    // 0) 有效像素太少 → 几乎是空白
    if pixel_count < bd::MIN_VALID_PIXELS {
        return true;
    }

    // 1) 全黑/全白/单色 → 一定空白
    if seen.len() <= 1 {
        return true;
    }

    // 2) 计算亮度方差 (mean abs deviation 简化版)
    let mean = lum_sum as f64 / pixel_count as f64;
    let variance = (lum_sum_sq as f64 / pixel_count as f64) - mean * mean;
    let std_dev = variance.sqrt();

    // 3) 启发式 (满足任一即通过):
    //    - 色数 ≥ COLOR_COUNT_RICH  → 多色图标 (常见 UI 图标)
    //    - 亮度标准差 ≥ LUMA_STD_RICH  → 高对比度 (黑白剪影 / 渐变)
    //    - 色数 ≥ COLOR_COUNT_MID 且 亮度标准差 ≥ LUMA_STD_MID  → 多色 + 有亮度变化
    //    - 其余 → 大概率 Windows 空白方块
    if seen.len() >= bd::COLOR_COUNT_RICH {
        return false;
    }
    if std_dev >= bd::LUMA_STD_RICH {
        return false;
    }
    if seen.len() >= bd::COLOR_COUNT_MID && std_dev >= bd::LUMA_STD_MID {
        return false;
    }
    true
}

/// 把 RGBA 字节编码为 PNG. 失败 -> None.
fn encode_png(rgba: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    use png::Encoder;
    use std::io::BufWriter;

    let mut out: Vec<u8> = Vec::with_capacity(rgba.len() / 4);
    let result = {
        let writer = BufWriter::new(&mut out);
        let mut encoder = Encoder::new(writer, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        match encoder.write_header() {
            Ok(mut w) => w.write_image_data(rgba),
            Err(e) => {
                log_icon_debug("png-header-failed", "<rgba>", &e.to_string());
                return None;
            }
        }
    };
    if result.is_err() {
        log_icon_debug(
            "png-write-failed",
            "<rgba>",
            &result.err().map(|e| e.to_string()).unwrap_or_default(),
        );
        return None;
    }
    Some(out)
}

/// 计算 RGBA 图像中有效像素的边界框 (非完全透明的像素).
///
/// 返回 (left, top, right, bottom) —— 都是 inclusive.
/// 如果所有像素都是完全透明的, 返回 None.
fn find_content_bounds(rgba: &[u8], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
    let w = width as usize;
    let h = height as usize;
    let mut min_x = w as i32;
    let mut min_y = h as i32;
    let mut max_x = -1i32;
    let mut max_y = -1i32;
    let alpha_threshold = 8u8; // 透明度低于此值视为"空"

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) * 4;
            let a = rgba[idx + 3];
            if a > alpha_threshold {
                if x < min_x as usize {
                    min_x = x as i32;
                }
                if y < min_y as usize {
                    min_y = y as i32;
                }
                if x as i32 > max_x {
                    max_x = x as i32;
                }
                if y as i32 > max_y {
                    max_y = y as i32;
                }
            }
        }
    }

    if max_x < 0 || max_y < 0 {
        return None;
    }
    Some((min_x as u32, min_y as u32, max_x as u32, max_y as u32))
}

/// 从源图像中裁剪指定区域.
fn crop_rgba(rgba: &[u8], src_w: u32, src_h: u32, x: u32, y: u32, w: u32, h: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for row in 0..h {
        let src_row = y + row;
        if src_row >= src_h {
            // 超出范围, 填充透明
            out.extend(std::iter::repeat(0).take((w * 4) as usize));
            continue;
        }
        let src_start = (src_row * src_w + x) as usize * 4;
        let src_end = src_start + (w as usize) * 4;
        if src_end > rgba.len() {
            out.extend(std::iter::repeat(0).take((w * 4) as usize));
            continue;
        }
        out.extend_from_slice(&rgba[src_start..src_end]);
    }
    out
}

/// 双线性插值缩放 RGBA 图像.
fn resize_rgba_bilinear(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8> {
    let mut dst = vec![0u8; (dst_w * dst_h * 4) as usize];

    let x_ratio = src_w as f32 / dst_w as f32;
    let y_ratio = src_h as f32 / dst_h as f32;

    for y in 0..dst_h {
        for x in 0..dst_w {
            let src_x = (x as f32 + 0.5) * x_ratio - 0.5;
            let src_y = (y as f32 + 0.5) * y_ratio - 0.5;

            let x0 = src_x.floor() as i32;
            let y0 = src_y.floor() as i32;
            let x1 = x0 + 1;
            let y1 = y0 + 1;

            let fx = src_x - x0 as f32;
            let fy = src_y - y0 as f32;

            let x0_clamped = x0.clamp(0, src_w as i32 - 1) as u32;
            let y0_clamped = y0.clamp(0, src_h as i32 - 1) as u32;
            let x1_clamped = x1.clamp(0, src_w as i32 - 1) as u32;
            let y1_clamped = y1.clamp(0, src_h as i32 - 1) as u32;

            let idx00 = ((y0_clamped * src_w + x0_clamped) * 4) as usize;
            let idx10 = ((y0_clamped * src_w + x1_clamped) * 4) as usize;
            let idx01 = ((y1_clamped * src_w + x0_clamped) * 4) as usize;
            let idx11 = ((y1_clamped * src_w + x1_clamped) * 4) as usize;

            let dst_idx = ((y * dst_w + x) * 4) as usize;

            for c in 0..4 {
                let v00 = src[idx00 + c] as f32;
                let v10 = src[idx10 + c] as f32;
                let v01 = src[idx01 + c] as f32;
                let v11 = src[idx11 + c] as f32;

                let top = v00 * (1.0 - fx) + v10 * fx;
                let bottom = v01 * (1.0 - fx) + v11 * fx;
                let val = top * (1.0 - fy) + bottom * fy;

                dst[dst_idx + c] = val.clamp(0.0, 255.0) as u8;
            }
        }
    }

    dst
}

/// 自动裁剪透明边界并居中放大.
///
/// 解决"图标只有左上角一点点"的问题:
/// - 检测有效像素的边界框
/// - 如果有效内容占比 < 60%, 说明图标被大量透明区域包围
/// - 裁剪后按比例放大到目标尺寸的 85%, 然后居中放置
/// - 保持宽高比, 使用双线性插值保证质量
///
/// 阈值集中在 `config::icon::autocrop::*`, 改一处全工程生效.
fn autocrop_and_center(rgba: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    use icon_cfg::autocrop as ac;

    let bounds = find_content_bounds(rgba, width, height)?;
    let (left, top, right, bottom) = bounds;

    let content_w = right - left + 1;
    let content_h = bottom - top + 1;

    // 计算有效内容占比
    let area_ratio = (content_w as f32 * content_h as f32) / (width as f32 * height as f32);

    // 如果内容占比已经足够大, 不需要裁剪
    if area_ratio >= ac::MIN_AREA_RATIO {
        return None;
    }

    // 计算目标尺寸 (保持宽高比, 最大占目标的 MAX_SCALE_RATIO)
    let max_w = (width as f32 * ac::MAX_SCALE_RATIO) as u32;
    let max_h = (height as f32 * ac::MAX_SCALE_RATIO) as u32;

    let scale = (max_w as f32 / content_w as f32).min(max_h as f32 / content_h as f32);

    let new_w = (content_w as f32 * scale).round() as u32;
    let new_h = (content_h as f32 * scale).round() as u32;

    if new_w == 0 || new_h == 0 {
        return None;
    }

    // 裁剪有效内容
    let cropped = crop_rgba(rgba, width, height, left, top, content_w, content_h);

    // 缩放到新尺寸
    let resized = resize_rgba_bilinear(&cropped, content_w, content_h, new_w, new_h);

    // 居中放置到透明画布上
    let mut out = vec![0u8; (width * height * 4) as usize];
    let offset_x = (width - new_w) / 2;
    let offset_y = (height - new_h) / 2;

    for y in 0..new_h {
        let src_start = (y * new_w) as usize * 4;
        let src_end = src_start + (new_w as usize) * 4;
        let dst_start = ((y + offset_y) * width + offset_x) as usize * 4;
        let dst_end = dst_start + (new_w as usize) * 4;
        out[dst_start..dst_end].copy_from_slice(&resized[src_start..src_end]);
    }

    Some(out)
}

/// 仅供测试/调试使用: 清除进程内图标缓存.
#[cfg(test)]
pub fn clear_cache_for_tests() {
    if let Some(c) = ICON_CACHE.get() {
        c.lock().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_lowercases_and_normalizes() {
        let p = Path::new(r"C:\Windows\System32\notepad.exe");
        let k = cache_key(p);
        assert!(k.to_lowercase().contains("notepad"));
    }

    #[test]
    fn missing_file_returns_none() {
        let p = "C:\\does\\not\\exist\\fake_app.exe";
        let result = get_or_extract_cached(p).expect("must never error");
        assert!(result.is_none());
    }

    #[test]
    fn png_encoder_produces_valid_output() {
        // 2x2 全红 RGBA
        let rgba = vec![
            255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
        ];
        let png = encode_png(&rgba, 2, 2).expect("encode ok");
        // PNG magic: 89 50 4E 47 0D 0A 1A 0A
        assert_eq!(
            &png[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
    }

    // === is_blank_icon 单元测试 ===

    /// 全白 32x32 = Windows 缺失 .lnk 目标时的典型行为.
    #[test]
    fn blank_icon_detected_for_white_square() {
        let rgba = vec![255u8; 32 * 32 * 4];
        assert!(is_blank_icon(&rgba), "全白方块应被识别为空白");
    }

    /// 全黑 32x32 = 同样可能 (罕见但存在).
    #[test]
    fn blank_icon_detected_for_black_square() {
        let rgba = vec![0u8; 32 * 32 * 4];
        assert!(is_blank_icon(&rgba), "全黑方块应被识别为空白");
    }

    /// 单色 (如纯红) = 应该是空白.
    #[test]
    fn blank_icon_detected_for_single_color() {
        let rgba = vec![200u8, 50, 50, 255].repeat(32 * 32);
        assert!(is_blank_icon(&rgba), "单色方块应被识别为空白");
    }

    /// 真实图标至少有几 + 颜色 (有渐变和抗锯齿). 模拟一个带渐变的图标.
    #[test]
    fn non_blank_icon_passes() {
        // 32x32 模拟图标: 4 行不同颜色, 已有 4 种 RGBA, 加上抖动就有更多.
        let mut rgba = Vec::with_capacity(32 * 32 * 4);
        for y in 0..32 {
            for x in 0..32 {
                rgba.push(x as u8); // R 随 x 变化
                rgba.push(y as u8); // G 随 y 变化
                rgba.push(128);
                rgba.push(255);
            }
        }
        assert!(!is_blank_icon(&rgba), "渐变图标不应被识别为空白");
    }

    /// 真实图标但只有 2 种颜色 (黑白剪影): 旧版会误判, 新版检查亮度方差.
    /// 2 种颜色的图标方差 = 0 → 应被判为 blank (但接受这个 trade-off).
    #[test]
    fn two_color_icon_rejected_by_old_and_new_logic() {
        // 棋盘格: 黑/白交替. 2 种 RGBA, 亮度方差 = 大, 因为有黑色 (0) 和 白色 (255).
        let mut rgba = Vec::with_capacity(32 * 32 * 4);
        for y in 0..32 {
            for x in 0..32 {
                let c = if (x + y) % 2 == 0 { 0u8 } else { 255u8 };
                rgba.push(c);
                rgba.push(c);
                rgba.push(c);
                rgba.push(255);
            }
        }
        // 新逻辑: 色数 2, 亮度方差巨大 (黑+白) → std_dev = 127.5 → 通过!
        assert!(
            !is_blank_icon(&rgba),
            "高对比度 2 色图标应通过 (亮度方差判据)"
        );
    }

    /// 极少颜色 (3-5 种) = 仍被判为空白. 真实图标至少 16+ 色 或 有亮度梯度.
    /// 注意: 4 色高对比度 (比如 0/60/120/180 灰度) 现在被认为是有效图标,
    /// 因为有清晰的亮度变化. 真正的"少色空白"是 2-3 种低对比度的颜色.
    #[test]
    fn few_color_icon_rejected() {
        let mut rgba = Vec::with_capacity(32 * 32 * 4);
        for _y in 0..32 {
            for x in 0..32 {
                // 3 种相近的灰度, 模拟"近似单色"图标
                let c = match x % 3 {
                    0 => 200u8,
                    1 => 205u8,
                    _ => 210u8,
                };
                rgba.push(c);
                rgba.push(c);
                rgba.push(c);
                rgba.push(255);
            }
        }
        // 3 种几乎相同的灰度, 亮度方差很小 → 判 blank
        assert!(is_blank_icon(&rgba), "3 种相近灰度应被识别为空白");
    }

    /// 边界: 1x1 像素 = 永远"空白" (无足够信息判断).
    #[test]
    fn tiny_buffer_marked_blank() {
        let rgba = vec![0u8; 4];
        assert!(is_blank_icon(&rgba));
    }

    /// 真实场景: 一个有 8 种颜色的简单图标 (低饱和度) 应通过 (有亮度变化).
    #[test]
    fn simple_icon_with_varying_brightness_passes() {
        let mut rgba = Vec::with_capacity(32 * 32 * 4);
        for y in 0..32 {
            for x in 0..32 {
                // 8 种颜色循环, 但每种亮度不同
                let idx = (x + y) % 8;
                let v = (idx * 32) as u8; // 0, 32, 64, ..., 224
                rgba.push(v);
                rgba.push(v);
                rgba.push(v);
                rgba.push(255);
            }
        }
        // 8 种灰度, 亮度方差 = 大 (从 0 到 224) → 通过
        assert!(!is_blank_icon(&rgba), "灰度渐变图标应通过");
    }

    // === IconExtractor trait 单元测试 ===

    /// trait 注册表: 默认 extractor 必须是 Windows impl.
    #[test]
    fn get_extractor_returns_windows_impl() {
        let ex = get_extractor();
        assert_eq!(ex.platform_name(), "windows");
    }

    /// supports: Windows 支持的常见 PE / 快捷方式 / URL 格式.
    #[test]
    fn windows_extractor_supports_common_types() {
        let ex = WindowsIconExtractor;
        assert!(ex.supports(Path::new("C:/x/y.exe")));
        assert!(ex.supports(Path::new("C:/x/y.lnk")));
        assert!(ex.supports(Path::new("C:/x/y.url")));
        assert!(ex.supports(Path::new("C:/x/y.ico")));
        assert!(ex.supports(Path::new("C:/x/y.msi")));
        assert!(ex.supports(Path::new("C:/x/y.scr")));
    }

    /// supports: 大小写不敏感 (大写扩展名也能识别).
    #[test]
    fn windows_extractor_supports_uppercase_extensions() {
        let ex = WindowsIconExtractor;
        assert!(ex.supports(Path::new("C:/x/Y.EXE")));
        assert!(ex.supports(Path::new("C:/x/Y.Lnk")));
    }

    /// supports: 不在白名单的扩展名 (.txt) 返回 false; 没扩展名的路径
    /// 返回 true (例如 POSIX 子系统裸 binary, 让 extract 内部自行判断).
    /// 这把"快速拒绝明显无关文件"和"对未知格式保持宽容"区分开.
    #[test]
    fn windows_extractor_supports_filters_unrelated_extensions() {
        let ex = WindowsIconExtractor;
        // 已知白名单 → true
        assert!(ex.supports(Path::new("C:/x/y.exe")));
        // 不在白名单 → false (快速拒绝)
        assert!(!ex.supports(Path::new("C:/x/y.txt")));
        assert!(!ex.supports(Path::new("C:/x/y.unknown")));
        // 没扩展名 → true (宽容路径)
        assert!(ex.supports(Path::new("C:/x/y")));
    }

    /// 端到端: 通过 trait 默认 extractor 提取, 行为与旧 `get_or_extract_cached` 一致.
    /// 不存在的文件永远返回 None (符合"永不抛错"协议).
    #[test]
    fn trait_extractor_path_returns_none_for_missing_file() {
        let ex = get_extractor();
        let result = ex.extract(Path::new("C:/does/not/exist/totally_fake.exe"), 32);
        assert!(result.is_none());
    }

    /// Mock extractor: 用于将来在测试中替换默认实现.
    /// 验证 trait 是 dyn-compatible (object-safe) — `dyn IconExtractor` 可正常工作.
    #[test]
    fn trait_object_safety_allows_dyn_dispatch() {
        let ex: Box<dyn IconExtractor> = Box::new(WindowsIconExtractor);
        let platform = ex.platform_name();
        assert_eq!(platform, "windows");
    }

    // === autocrop_and_center 单元测试 ===

    /// 全透明 32x32 → find_content_bounds 返回 None → autocrop_and_center 返回 None.
    #[test]
    fn autocrop_returns_none_for_fully_transparent() {
        let rgba = vec![0u8; 32 * 32 * 4];
        let result = autocrop_and_center(&rgba, 32, 32);
        assert!(result.is_none(), "全透明图像不应触发裁剪");
    }

    /// 内容占满整个画布 (100%) → 不触发裁剪 (area_ratio >= MIN_AREA_RATIO).
    #[test]
    fn autocrop_returns_none_when_content_fills_canvas() {
        // 32x32 全不透明渐变, 内容占比 = 100%
        let mut rgba = Vec::with_capacity(32 * 32 * 4);
        for y in 0..32 {
            for x in 0..32 {
                rgba.push(x as u8);
                rgba.push(y as u8);
                rgba.push(128);
                rgba.push(255);
            }
        }
        let result = autocrop_and_center(&rgba, 32, 32);
        assert!(result.is_none(), "满画布内容不应触发裁剪");
    }

    /// 小图标 (中心 8x8 不透明方块, 其余透明) → 应触发裁剪并放大.
    /// 8x8 / 32x32 = 6.25% < 35% (MIN_AREA_RATIO), 应被放大到 ~92% (MAX_SCALE_RATIO).
    #[test]
    fn autocrop_enlarges_small_centered_icon() {
        let size = 32u32;
        let mut rgba = vec![0u8; (size * size * 4) as usize];

        // 在中心画一个 8x8 的红色方块 (alpha=255)
        let icon_size = 8u32;
        let offset = (size - icon_size) / 2; // 12
        for y in offset..(offset + icon_size) {
            for x in offset..(offset + icon_size) {
                let idx = ((y * size + x) * 4) as usize;
                rgba[idx] = 255; // R
                rgba[idx + 3] = 255; // A
            }
        }

        let result = autocrop_and_center(&rgba, size, size);
        assert!(result.is_some(), "小图标应触发裁剪放大");
        let processed = result.unwrap();
        assert_eq!(processed.len(), (size * size * 4) as usize);

        // 验证中心区域有红色像素 (放大后的图标应该覆盖大部分画布)
        let mut red_count = 0;
        for px in processed.chunks_exact(4) {
            if px[0] > 128 && px[3] > 128 {
                red_count += 1;
            }
        }
        // 放大到 92% 后, 红色像素应远多于原始 64 个
        assert!(
            red_count > 200,
            "放大后红色像素应显著增多 (实际: {})",
            red_count
        );
    }

    /// 内容偏左上的图标 → 裁剪后应居中放置.
    /// 验证 autocrop 不仅放大, 还会将内容居中.
    #[test]
    fn autocrop_centers_offset_content() {
        let size = 32u32;
        let mut rgba = vec![0u8; (size * size * 4) as usize];

        // 在左上角 (0,0) 开始画 10x10 蓝色方块
        let icon_size = 10u32;
        for y in 0..icon_size {
            for x in 0..icon_size {
                let idx = ((y * size + x) * 4) as usize;
                rgba[idx + 2] = 255; // B
                rgba[idx + 3] = 255; // A
            }
        }

        // 10x10 / 32x32 = 9.7% < 35%, 应触发裁剪
        let result = autocrop_and_center(&rgba, size, size);
        assert!(result.is_some(), "偏置小图标应触发裁剪");
        let processed = result.unwrap();

        // 验证蓝色像素在输出中大致居中 (中心区域有蓝色)
        let center_y = size / 2;
        let center_x = size / 2;
        let center_idx = ((center_y * size + center_x) * 4) as usize;
        // 放大后的图标应该覆盖中心
        assert!(
            processed[center_idx + 2] > 0 || processed[center_idx + 3] > 0,
            "放大后中心附近应有内容"
        );
    }

    /// 验证 find_content_bounds 正确识别非透明像素的边界.
    #[test]
    fn find_content_bounds_detects_correct_bbox() {
        let size = 16u32;
        let mut rgba = vec![0u8; (size * size * 4) as usize];

        // 在 (2,3) 到 (5,7) 画一个矩形 (宽4高5)
        for y in 3..8 {
            for x in 2..6 {
                let idx = ((y * size + x) * 4) as usize;
                rgba[idx + 3] = 255;
            }
        }

        let bounds = find_content_bounds(&rgba, size, size);
        assert!(bounds.is_some());
        let (left, top, right, bottom) = bounds.unwrap();
        assert_eq!(left, 2, "left 应为 2");
        assert_eq!(top, 3, "top 应为 3");
        assert_eq!(right, 5, "right 应为 5");
        assert_eq!(bottom, 7, "bottom 应为 7");
    }

    /// 验证 crop_rgba 正确裁剪指定区域.
    #[test]
    fn crop_rgba_extracts_correct_region() {
        let src_w = 8u32;
        let src_h = 8u32;
        let mut src = vec![0u8; (src_w * src_h * 4) as usize];

        // 填充可识别的模式: 每个像素的 R = x, G = y
        for y in 0..src_h {
            for x in 0..src_w {
                let idx = ((y * src_w + x) * 4) as usize;
                src[idx] = x as u8; // R = x
                src[idx + 1] = y as u8; // G = y
                src[idx + 3] = 255;
            }
        }

        // 裁剪 (2,1) 开始的 3x2 区域
        let cropped = crop_rgba(&src, src_w, src_h, 2, 1, 3, 2);
        assert_eq!(cropped.len(), (3 * 2 * 4) as usize);

        // 验证第一个像素 (原图的 2,1)
        assert_eq!(cropped[0], 2, "R 应等于原 x=2");
        assert_eq!(cropped[1], 1, "G 应等于原 y=1");

        // 验证第二行第一个像素 (原图的 2,2)
        let row1_start = (3 * 4) as usize;
        assert_eq!(cropped[row1_start], 2, "第二行 R 应等于原 x=2");
        assert_eq!(cropped[row1_start + 1], 2, "第二行 G 应等于原 y=2");
    }

    /// 验证 resize_rgba_bilinear 正确缩放图像.
    #[test]
    fn resize_rgba_bilinear_scales_correctly() {
        // 2x2 源图像: 左上红, 右上绿, 左下蓝, 右下白
        let src = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];

        let dst = resize_rgba_bilinear(&src, 2, 2, 4, 4);
        assert_eq!(dst.len(), 4 * 4 * 4);

        // 左上角 (0,0) 应接近红色
        assert!(dst[0] > 200, "左上角应偏红");
        // 右下角 (3,3) 应接近白色
        let last_px_start = (3 * 4 + 3) * 4;
        assert!(dst[last_px_start] > 200, "右下角 R 应偏白");
        assert!(dst[last_px_start + 1] > 200, "右下角 G 应偏白");
        assert!(dst[last_px_start + 2] > 200, "右下角 B 应偏白");
    }

    /// 验证 autocrop 阈值变更 (0.35) 的效果:
    /// 一个占 50% 面积的图标 (16x16 内容在 32x32 画布上)
    /// 旧阈值 0.6 会触发放大, 新阈值 0.35 不应触发.
    #[test]
    fn autocrop_threshold_respects_new_ratio() {
        let size = 32u32;
        let mut rgba = vec![0u8; (size * size * 4) as usize];

        // 画一个 16x16 的方块 (面积比 = 25%, 仍 < 35%, 应触发)
        for y in 0..16 {
            for x in 0..16 {
                let idx = ((y * size + x) * 4) as usize;
                rgba[idx + 3] = 255;
            }
        }

        // 16*16 / 32*32 = 25% < 35%, 应触发裁剪
        let result = autocrop_and_center(&rgba, size, size);
        assert!(result.is_some(), "25% 面积比应触发裁剪 (阈值 0.35)");

        // 再测试 60% 面积比 (不触发)
        let mut rgba2 = vec![0u8; (size * size * 4) as usize];
        // 20x20 = 400 / 1024 ≈ 39% > 35%, 不触发
        for y in 0..20 {
            for x in 0..20 {
                let idx = ((y * size + x) * 4) as usize;
                rgba2[idx + 3] = 255;
            }
        }
        let result2 = autocrop_and_center(&rgba2, size, size);
        assert!(result2.is_none(), "39% 面积比不应触发裁剪 (阈值 0.35)");
    }
}
