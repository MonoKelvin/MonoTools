//! Windows 平台 IPC 命令
//!
//! 独立模块设计：平台相关的 IPC 命令定义在此模块中，
//! 通过 core_ipc_commands! 宏注册到全局。
//!
//! 包含：
//! - 图标提取 (get_app_icon, get_app_icons_batch)
//! - Shell 操作 (open_file_location, show_file_properties, delete_file_to_recycle_bin)
//! - 系统主题 (get_system_theme)

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use std::path::PathBuf;

/// 获取可执行文件图标 (base64 编码 PNG).
#[tauri::command]
pub async fn get_app_icon(path: String) -> Result<Option<String>, String> {
    // shell: 开头的是 shell 命名空间路径 (如 UWP 应用的 shell:AppsFolder\...),
    // resolve_shortcut 对此类路径无意义且会返回原路径. 直接跳过解析.
    let resolved_path = if path.starts_with("shell:") {
        PathBuf::from(&path)
    } else {
        crate::platform::windows::shell::resolve_shortcut(&PathBuf::from(&path))
            .unwrap_or(PathBuf::from(&path))
    };
    let resolved_path_str = resolved_path.to_string_lossy().to_string();

    {
        let cache = crate::platform::windows::icon::cache_snapshot();
        if let Some(cached) = cache.get(&resolved_path_str.to_lowercase()) {
            return Ok(cached.as_ref().map(|v| BASE64.encode(v)));
        }
    }

    let path_clone = resolved_path_str.clone();
    let bytes = tokio::task::spawn_blocking(move || {
        crate::platform::windows::icon::get_or_extract_cached(&path_clone).unwrap_or(None)
    })
    .await
    .map_err(|e| format!("icon join error: {}", e))?;

    if let Some(ref b) = bytes {
        Ok(Some(BASE64.encode(b)))
    } else {
        log::debug!(
            "[icon-ipc] extraction returned None for path={}",
            resolved_path_str
        );
        Ok(None)
    }
}

/// 批量获取图标 (base64 编码 PNG 数组).
#[tauri::command]
pub async fn get_app_icons_batch(paths: Vec<String>) -> Result<Vec<Option<String>>, String> {
    let total = paths.len();
    if total == 0 {
        return Ok(Vec::new());
    }

    let resolved: Vec<String> = paths
        .iter()
        .map(|p| {
            // shell: 开头的是 shell 命名空间路径 (如 UWP 应用的 shell:AppsFolder\...),
            // resolve_shortcut 对此类路径无意义, 直接跳过解析.
            if p.starts_with("shell:") {
                p.clone()
            } else {
                crate::platform::windows::shell::resolve_shortcut(&PathBuf::from(p))
                    .unwrap_or_else(|_| PathBuf::from(p))
                    .to_string_lossy()
                    .to_string()
            }
        })
        .collect();

    let mut out: Vec<Option<String>> = vec![None; total];
    let mut pending: Vec<(usize, String)> = Vec::new();
    {
        let cache = crate::platform::windows::icon::cache_snapshot();
        for (i, p) in resolved.iter().enumerate() {
            if let Some(cached) = cache.get(&p.to_lowercase()) {
                out[i] = cached.as_ref().map(|v| BASE64.encode(v));
            } else {
                pending.push((i, p.clone()));
            }
        }
    }

    let cache_hits = out.iter().filter(|x| x.is_some()).count();
    if pending.is_empty() {
        log::info!("[icon] batch fetched {} paths (all cache hit)", total);
        return Ok(out);
    }

    let concurrency = std::cmp::min(
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4),
        8,
    );
    let concurrency = concurrency.max(2);

    let chunk_size = pending.len().div_ceil(concurrency);
    let mut chunks: Vec<Vec<(usize, String)>> = Vec::with_capacity(concurrency);
    for chunk in pending.chunks(chunk_size) {
        chunks.push(chunk.to_vec());
    }

    let mut handles = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let handle = tokio::task::spawn_blocking(move || {
            let mut results = Vec::with_capacity(chunk.len());
            for (idx, p) in chunk {
                let bytes =
                    crate::platform::windows::icon::get_or_extract_cached(&p).unwrap_or(None);
                results.push((idx, bytes));
            }
            results
        });
        handles.push(handle);
    }

    let all_results = futures::future::join_all(handles).await;

    for result in all_results {
        match result {
            Ok(chunk_results) => {
                for (idx, bytes) in chunk_results {
                    out[idx] = bytes.as_ref().map(|v| BASE64.encode(v));
                }
            }
            Err(e) => {
                log::error!("[icon] batch chunk join error: {}", e);
            }
        }
    }

    log::info!(
        "[icon] batch fetched {} paths, cache_hits={}, extracted={}, success={}, concurrency={}",
        total,
        cache_hits,
        pending.len(),
        out.iter().filter(|x| x.is_some()).count(),
        concurrency
    );

    Ok(out)
}

/// 打开文件所在位置 (在资源管理器中选中文件).
#[tauri::command]
pub async fn open_file_location(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    crate::platform::windows::shell::open_path(&p).map_err(|e| e.to_string())
}

/// 显示文件属性对话框.
#[tauri::command]
pub async fn show_file_properties(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    crate::platform::windows::shell::show_file_properties(&p).map_err(|e| e.to_string())
}

/// 删除文件到回收站.
#[tauri::command]
pub async fn delete_file_to_recycle_bin(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    crate::platform::windows::shell::delete_to_recycle_bin(&p).map_err(|e| e.to_string())
}

/// 获取 Windows 系统当前主题模式 ("light" 或 "dark").
/// 读取注册表 HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme.
#[tauri::command]
pub fn get_system_theme() -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::process::Command;

        let output = Command::new("reg")
            .args([
                "query",
                r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize",
                "/v",
                "AppsUseLightTheme",
            ])
            .output()
            .map_err(|e| format!("无法执行 reg query: {}", e))?;

        if !output.status.success() {
            return Ok("dark".to_string());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("AppsUseLightTheme") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let value = parts[2];
                    if value == "0x0" || value == "0" {
                        return Ok("dark".to_string());
                    } else {
                        return Ok("light".to_string());
                    }
                }
            }
        }

        Ok("dark".to_string())
    }

    #[cfg(not(windows))]
    {
        Ok("dark".to_string())
    }
}

/// 注册 Windows 平台的 IPC 命令到 Tauri builder
pub fn register_ipc_commands(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.invoke_handler(tauri::generate_handler![
        get_app_icon,
        get_app_icons_batch,
        open_file_location,
        show_file_properties,
        delete_file_to_recycle_bin,
        get_system_theme,
    ])
}
