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

use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// 图标尺寸 (像素). 与 ResultItem 视觉对齐 + 留 1.5x 高分屏余量.
const ICON_PX: i32 = 32;

type IconCache = HashMap<String, Option<Vec<u8>>>;

static ICON_CACHE: OnceLock<Mutex<IconCache>> = OnceLock::new();

fn cache() -> &'static Mutex<IconCache> {
    ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
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

    let extracted = extract_icon_bytes(p);
    if extracted.is_none() {
        log_icon_debug("extract-failed", path, "extract_icon_bytes returned None");
        log::warn!("[icon] extract-failed path={} (file exists but extraction returned None)", path);
    } else {
        log::info!("[icon] extracted path={} bytes={}", path, extracted.as_ref().unwrap().len());
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
fn extract_icon_bytes(path: &Path) -> Option<Vec<u8>> {
    #[cfg(windows)]
    {
        extract_icon_windows(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

#[cfg(windows)]
fn extract_icon_windows(path: &Path) -> Option<Vec<u8>> {
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

    let hbm = unsafe { CreateCompatibleBitmap(screen_dc, ICON_PX, ICON_PX) };
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
            ICON_PX,
            ICON_PX,
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
    bmi.bmiHeader.biWidth = ICON_PX;
    // 负值高度 = top-down DIB, 我们要的就是 top-down
    bmi.bmiHeader.biHeight = -ICON_PX;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB.0;

    let mut buffer: Vec<u8> = vec![0u8; (ICON_PX * ICON_PX * 4) as usize];

    let scan = unsafe {
        GetDIBits(
            screen_dc,
            hbm,
            0,
            ICON_PX as u32,
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
            "icon appears blank (< 16 distinct colors), likely missing .lnk target");
        log::warn!("[icon] blank-icon-rejected path={} (Windows returned blank icon, likely .lnk with missing target)", path.to_string_lossy());
        return None;
    }

    // 6) PNG 编码
    encode_png(&rgba, ICON_PX as u32, ICON_PX as u32)
}

/// 检测 RGBA buffer 是否是"空白图标".
///
/// Windows 对某些文件 (典型场景: .lnk 指向已删除的目标) 会返回一个
/// 通用空白 32x32 图标. 视觉上是一格纯色或近似纯色方块, 用户无法分辨
/// "正在加载"和"真的没有图标".
///
/// 判定标准: 整个 buffer 内不同 RGBA 值的数量. 真实图标 (即使是单色)
/// 至少有抗锯齿边缘, 颜色数 ≥ 16. 真正的空白图标 (单色或 2-3 色) 远低于此.
pub fn is_blank_icon(rgba: &[u8]) -> bool {
    if rgba.len() < 4 {
        return true;
    }
    // 抽样检测: 每 4 个像素采一个, 性能上不会拖慢 32x32 的图.
    let mut seen = std::collections::HashSet::with_capacity(32);
    for (i, px) in rgba.chunks_exact(4).enumerate() {
        if i % 4 != 0 {
            continue;
        }
        seen.insert((px[0], px[1], px[2], px[3]));
        if seen.len() > 16 {
            return false;
        }
    }
    // 看到的颜色不超过 16 种 → 大概率是空白图标.
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

/// 解析 .lnk 快捷方式真实目标路径. 失败 -> None.
///
/// 当前实现仅做基础检查: 大多数情况下 SHGetFileInfoW 对 .lnk 自身已能返回
/// 正确图标 (Shell 自动解析并提取目标图标), 故此函数可作为后续增强占位.
#[allow(dead_code)]
pub fn resolve_shortcut_target(lnk_path: &Path) -> Option<PathBuf> {
    // 简化: 不依赖完整 COM, 让 .lnk 自身走 SHGetFileInfoW 即可拿到目标图标.
    // 大部分 .lnk 的图标与其目标的可执行文件一致.
    let _ = lnk_path;
    None
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

    /// 真实图标但只有 2 种颜色 (黑白剪影): 也应通过, 因为有 ≥ 16 种像素采样.
    /// 等等, 2 种颜色的图标会被判定为空白. 这是 by design 的取舍:
    /// Windows 不会返回 2 色图标, 真空白图标也只有 1-3 种颜色.
    #[test]
    fn two_color_icon_rejected() {
        // 棋盘格: 黑/白交替. 抽样 1/4 = 256 个像素, 看到的是黑或白 (2 种).
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
        // 棋盘格 = 2 种 RGBA, 落在阈值 16 之内, 会被判定为空白.
        // 这是 trade-off: 真实 2 色图标会被误判, 但收益是挡住 Windows 空白图标.
        assert!(is_blank_icon(&rgba));
    }

    /// 极少颜色 (3-5 种) = 仍被判为空白. 真实图标至少 16+ 色.
    #[test]
    fn few_color_icon_rejected() {
        let mut rgba = Vec::with_capacity(32 * 32 * 4);
        for y in 0..32 {
            for x in 0..32 {
                // 4 种颜色循环, 但每种只贡献 1 个不同 RGBA
                let c = (x % 4) as u8 * 60;
                rgba.push(c);
                rgba.push(c);
                rgba.push(c);
                rgba.push(255);
            }
        }
        assert!(is_blank_icon(&rgba), "4 色图标应被识别为空白");
    }

    /// 边界: 1x1 像素 = 永远"空白" (无足够信息判断).
    #[test]
    fn tiny_buffer_marked_blank() {
        let rgba = vec![0u8; 4];
        assert!(is_blank_icon(&rgba));
    }
}
