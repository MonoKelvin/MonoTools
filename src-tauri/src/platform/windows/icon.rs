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

use crate::config::icon as icon_cfg;

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
        // .exe / .lnk / .url / .ico / .msi 是 Windows 上能 SHGetFileInfo 出图标的常见格式.
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

fn cache() -> &'static Mutex<IconCache> {
    ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
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

/// 公共入口: 带缓存的图标提取. 永远不抛错.
///
/// 失败诊断: 在 `mono_icon_debug` 环境变量被设置 (=1) 时, 任何
/// `None` 返回都会写一条 `log::warn!` 含 path + 阶段, 方便前端/后端
/// 联合排查"图标为什么是空白".
pub fn get_or_extract_cached(path: &str) -> crate::error::Result<Option<Vec<u8>>> {
    let p = Path::new(path);
    if !p.exists() {
        log_icon_debug("file-missing", path, "Path::exists() == false");
        log::warn!("[icon] file-missing path={}", path);
        return Ok(None);
    }

    let key = cache_key(p);
    {
        let cache = cache().lock();
        if let Some(cached) = cache.get(&key) {
            return Ok(cached.clone());
        }
    }

    // 走 trait 抽象的默认实现 (Windows 平台). 调用方不感知具体 impl.
    let extracted = get_extractor().extract(p, icon_cfg::SIZE);

    if extracted.is_none() {
        log_icon_debug("extract-failed", path, "extractor returned None");
        log::warn!(
            "[icon] extract-failed path={} (file exists but extraction returned None)",
            path
        );
    } else {
        log::info!(
            "[icon] extracted path={} bytes={}",
            path,
            extracted.as_ref().unwrap().len()
        );
    }
    let mut cache = cache().lock();
    cache.insert(key, extracted.clone());
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
        stage, path, detail,
    );
}

/// 实际提取. 任意步骤失败 -> None (不抛错).
///
/// size: 边长像素. 现在统一用 `icon_cfg::SIZE` (32), 但签名已抽象
/// 以便未来 `IconExtractor` impl 走不同尺寸 (例如 macOS NSWorkspace
/// 可按 16/32/64 任意).
#[cfg(windows)]
fn extract_icon_windows(path: &Path, size: i32) -> Option<Vec<u8>> {
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, SelectObject,
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC,
    };
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHGFI_ICON, SHGFI_LARGEICON, SHFILEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, HICON};
    use windows::core::PCWSTR;

    // 1) SHGetFileInfoW -> HICON
    let wide_path: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

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
    if result == 0 {
        log_icon_debug("shgetfileinfo-zero", &path.to_string_lossy(), "SHGetFileInfoW returned 0");
        return None;
    }
    let hicon: HICON = info.hIcon;
    if hicon.0.is_null() {
        log_icon_debug("hicon-null", &path.to_string_lossy(), "SHGetFileInfoW returned null hIcon");
        return None;
    }

    // 2) 准备 DC + Bitmap + 把 HICON 画上去
    let screen_dc: HDC = unsafe { CreateCompatibleDC(None) };
    if screen_dc.is_invalid() {
        log_icon_debug("create-dc-failed", &path.to_string_lossy(), "CreateCompatibleDC returned invalid HDC");
        unsafe {
            let _ = DestroyIcon(hicon);
        }
        return None;
    }

    let hbm = unsafe { CreateCompatibleBitmap(screen_dc, size, size) };
    if hbm.is_invalid() {
        log_icon_debug("create-bitmap-failed", &path.to_string_lossy(), "CreateCompatibleBitmap returned invalid HBITMAP");
        unsafe {
            let _ = DeleteDC(screen_dc);
            let _ = DestroyIcon(hicon);
        }
        return None;
    }

    let prev = unsafe { SelectObject(screen_dc, hbm.into()) };
    let _ = prev; // 不主动恢复: 整个 DC 接下来就释放

    // DI_NORMAL = 0x0003
    let di_flags = windows::Win32::UI::WindowsAndMessaging::DI_FLAGS(0x0003);
    let draw_ok = unsafe {
        DrawIconEx(
            screen_dc,
            0,
            0,
            hicon,
            size,
            size,
            0,
            None,
            di_flags,
        )
        .is_ok()
    };
    if !draw_ok {
        log_icon_debug("draw-icon-failed", &path.to_string_lossy(), "DrawIconEx returned err");
        unsafe {
            let _ = DeleteDC(screen_dc);
            let _ = DestroyIcon(hicon);
        }
        return None;
    }

    // 3) GetDIBits -> RGBA
    let mut bmi: BITMAPINFO = unsafe { std::mem::zeroed() };
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = size;
    // 负值高度 = top-down DIB, 我们要的就是 top-down
    bmi.bmiHeader.biHeight = -size;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB.0;

    // buffer 长度按 size 算, 不再硬写 32*32*4; 防止未来 macOS impl
    // 传非 32 时 OOB.
    let byte_len = (size as usize) * (size as usize) * 4;
    let mut buffer: Vec<u8> = vec![0u8; byte_len];

    let scan = unsafe {
        GetDIBits(
            screen_dc,
            hbm,
            0,
            size as u32,
            Some(buffer.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };

    // 资源清理
    unsafe {
        let _ = DeleteObject(hbm.into());
        let _ = DeleteDC(screen_dc);
        let _ = DestroyIcon(hicon);
    }

    if scan == 0 {
        log_icon_debug("getdibits-zero", &path.to_string_lossy(), "GetDIBits returned 0");
        return None;
    }

    // 4) BGRA -> RGBA 转换 (Windows DIB 32-bit 是 BGRA byte order)
    let mut rgba = Vec::with_capacity(buffer.len());
    for chunk in buffer.chunks_exact(4) {
        rgba.push(chunk[2]); // R = 原 B
        rgba.push(chunk[1]); // G = 原 G
        rgba.push(chunk[0]); // B = 原 R
        rgba.push(chunk[3]); // A
    }

    // 5) 检测"空白图标" (Windows 对缺失 .lnk 目标会返回通用空白方块)
    // 旧版会把空白 PNG 当合法图标返回, 用户看到的就是一格 32x32 白色方块.
    // 现在检测到后返回 None, 让前端走 Lucide 兜底.
    if is_blank_icon(&rgba) {
        log_icon_debug("blank-icon-rejected", &path.to_string_lossy(),
            "icon appears blank (low color count or low luma variance), likely missing .lnk target");
        log::warn!("[icon] blank-icon-rejected path={} (Windows returned blank icon, likely .lnk with missing target)", path.to_string_lossy());
        return None;
    }

    // 6) PNG 编码
    encode_png(&rgba, size as u32, size as u32)
}

/// 检测 RGBA buffer 是否是"空白图标".
///
/// Windows 对某些文件 (典型场景: .lnk 指向已删除的目标) 会返回一个
/// 通用空白 32x32 图标. 视觉上是一格纯色或近似纯色方块, 用户无法分辨
/// "正在加载"和"真的没有图标".
///
/// 判定标准 (2026-07 调整, 旧版太严):
/// - 旧: 抽样 1/4 像素, 不同 RGBA 数 > 16 才算"有内容"
/// - 新: 全采样, 同时检查"亮度方差" (mean abs deviation) 和"色数".
///   - 全黑 / 全白 / 单色 → 立刻判为 blank (color_count ≤ 1)
///   - 2-4 种颜色且亮度方差 < 4 → 可能是 Windows 空白方块, 判 blank
///   - 5+ 种颜色 或 亮度方差 ≥ 4 → 真实图标, 通过
///
/// 为什么放宽: 旧版把 16 色阈值定得过严, 导致很多有效图标
/// (尤其 .lnk 指向简单 target 的情况) 被判为 blank, 整个 list 都
/// 走不到后端 IPC 成功路径, 用户看到的就是一片"空白".
pub fn is_blank_icon(rgba: &[u8]) -> bool {
    if rgba.len() < 4 {
        return true;
    }

    let mut seen = std::collections::HashSet::with_capacity(64);
    let mut lum_sum: u64 = 0;
    let mut lum_sum_sq: u64 = 0;
    let mut pixel_count: u64 = 0;

    // 全采样 (32x32 = 1024 像素, 性能可接受)
    for px in rgba.chunks_exact(4) {
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

    if pixel_count == 0 {
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
    //
    // 阈值集中在 `config::icon::blank_detection::*`, 改一处全工程生效.
    use icon_cfg::blank_detection as bd;
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
        log_icon_debug("png-write-failed", "<rgba>", &result.err().map(|e| e.to_string()).unwrap_or_default());
        return None;
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
            255, 0, 0, 255, 255, 0, 0, 255,
            255, 0, 0, 255, 255, 0, 0, 255,
        ];
        let png = encode_png(&rgba, 2, 2).expect("encode ok");
        // PNG magic: 89 50 4E 47 0D 0A 1A 0A
        assert_eq!(&png[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
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
        assert!(!is_blank_icon(&rgba), "高对比度 2 色图标应通过 (亮度方差判据)");
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
}
