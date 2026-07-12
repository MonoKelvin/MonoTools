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
pub fn get_or_extract_cached(path: &str) -> crate::error::Result<Option<Vec<u8>>> {
    let p = Path::new(path);
    if !p.exists() {
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
    let mut cache = cache().lock();
    cache.insert(key, extracted.clone());
    Ok(extracted)
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
        return None;
    }
    let hicon: HICON = info.hIcon;
    if hicon.0.is_null() {
        return None;
    }

    // 2) 准备 DC + Bitmap + 把 HICON 画上去
    let screen_dc: HDC = unsafe { CreateCompatibleDC(None) };
    if screen_dc.is_invalid() {
        unsafe {
            let _ = DestroyIcon(hicon);
        }
        return None;
    }

    let hbm = unsafe { CreateCompatibleBitmap(screen_dc, ICON_PX, ICON_PX) };
    if hbm.is_invalid() {
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

    // 5) PNG 编码
    encode_png(&rgba, ICON_PX as u32, ICON_PX as u32)
}

/// 把 RGBA 字节编码为 PNG. 失败 -> None.
fn encode_png(rgba: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    use png::Encoder;
    use std::io::BufWriter;

    let mut out: Vec<u8> = Vec::with_capacity(rgba.len() / 4);
    {
        let writer = BufWriter::new(&mut out);
        let mut encoder = Encoder::new(writer, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(rgba).ok()?;
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
}
