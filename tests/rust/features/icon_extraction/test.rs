use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use monotools_lib::platform::windows::{icon, shell};

fn get_output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests")
        .join("output")
        .join("icons")
}

fn collect_test_paths() -> Vec<(String, String)> {
    let mut paths: Vec<(String, String)> = Vec::new();

    // System32 常见 exe (路径通用, 换电脑也能跑)
    paths.push((
        "cmd_exe".to_string(),
        "C:\\Windows\\System32\\cmd.exe".to_string(),
    ));
    paths.push((
        "notepad_exe".to_string(),
        "C:\\Windows\\System32\\notepad.exe".to_string(),
    ));
    paths.push((
        "calc_exe".to_string(),
        "C:\\Windows\\System32\\calc.exe".to_string(),
    ));

    // 动态扫描 System32 下的 exe (最多再补 3 个)
    let system32 = Path::new("C:\\Windows\\System32");
    if let Ok(entries) = fs::read_dir(system32) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("exe") {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    if !name.starts_with('.') && paths.len() < 6 {
                        paths.push((
                            format!("system32_{}", name),
                            path.to_string_lossy().to_string(),
                        ));
                    }
                }
            }
        }
    }

    // 公共 Start Menu (ProgramData, 所有用户可见)
    let start_menu = Path::new("C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs");
    if let Ok(entries) = fs::read_dir(start_menu) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("lnk") {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    if paths.len() < 10 {
                        paths.push((
                            format!("startmenu_{}", name),
                            path.to_string_lossy().to_string(),
                        ));
                    }
                }
            }
        }
    }

    // 当前用户的 Start Menu (通过 dirs::data_dir() 动态获取, 不写死用户名)
    if let Some(app_data) = dirs::data_dir() {
        let user_start_menu = app_data.join("Microsoft\\Windows\\Start Menu\\Programs");
        if let Ok(entries) = fs::read_dir(user_start_menu) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext.eq_ignore_ascii_case("lnk") {
                        let name = path.file_name().unwrap().to_string_lossy().to_string();
                        if paths.len() < 14 {
                            paths.push((
                                format!("user_startmenu_{}", name),
                                path.to_string_lossy().to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }

    // 桌面快捷方式 (动态获取桌面路径)
    if let Some(desktop) = dirs::desktop_dir() {
        if let Ok(entries) = fs::read_dir(desktop) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext.eq_ignore_ascii_case("lnk") {
                        let name = path.file_name().unwrap().to_string_lossy().to_string();
                        if paths.len() < 18 {
                            paths.push((
                                format!("desktop_{}", name),
                                path.to_string_lossy().to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }

    paths
        .into_iter()
        .filter(|(_, p)| Path::new(p).exists())
        .take(20)
        .collect()
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub async fn run_icon_extraction_tests() {
    let output_dir = get_output_dir();
    fs::create_dir_all(&output_dir).unwrap();
    println!("Output directory: {}", output_dir.display());

    let test_paths = collect_test_paths();
    println!("Testing {} paths:", test_paths.len());

    let mut results = Vec::new();
    let mut success_count = 0;

    for (label, path) in &test_paths {
        println!("Processing: {} -> {}", label, path);

        let resolved_path = shell::resolve_shortcut(&PathBuf::from(path))
            .unwrap_or(PathBuf::from(path));
        let resolved_path_str = resolved_path.to_string_lossy().to_string();

        println!("  Resolved to: {}", resolved_path_str);

        let result = icon::get_or_extract_cached(&resolved_path_str);

        match result {
            Ok(Some(bytes)) => {
                let sanitized_label = sanitize_filename(label);
                let output_path = output_dir.join(format!("{}.png", sanitized_label));

                if let Ok(mut file) = File::create(&output_path) {
                    if file.write_all(&bytes).is_ok() {
                        println!(
                            "  ✓ Extracted successfully, saved to: {}",
                            output_path.display()
                        );
                        results.push((
                            label.clone(),
                            path.clone(),
                            resolved_path_str.clone(),
                            "success".to_string(),
                            bytes.len(),
                        ));
                        success_count += 1;
                    } else {
                        println!("  ✗ Failed to write file");
                        results.push((
                            label.clone(),
                            path.clone(),
                            resolved_path_str.clone(),
                            "write_failed".to_string(),
                            0,
                        ));
                    }
                } else {
                    println!("  ✗ Failed to create output file");
                    results.push((
                        label.clone(),
                        path.clone(),
                        resolved_path_str.clone(),
                        "create_failed".to_string(),
                        0,
                    ));
                }
            }
            Ok(None) => {
                println!("  ✗ Extraction returned None");
                results.push((
                    label.clone(),
                    path.clone(),
                    resolved_path_str.clone(),
                    "extraction_none".to_string(),
                    0,
                ));
            }
            Err(e) => {
                println!("  ✗ Error: {}", e);
                results.push((
                    label.clone(),
                    path.clone(),
                    resolved_path_str.clone(),
                    format!("error: {}", e),
                    0,
                ));
            }
        }
    }

    let report_path = output_dir.join("report.txt");
    if let Ok(mut file) = File::create(&report_path) {
        writeln!(file, "Icon Extraction Test Report").unwrap();
        writeln!(file, "=================================").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "Total paths tested: {}", results.len()).unwrap();
        writeln!(file, "Successful extractions: {}", success_count).unwrap();
        writeln!(
            file,
            "Failed extractions: {}",
            results.len() - success_count
        )
        .unwrap();
        writeln!(file).unwrap();

        writeln!(file, "Detailed results:").unwrap();
        writeln!(file, "-----------------").unwrap();

        for (label, original_path, resolved_path, status, size) in results {
            writeln!(file, "{}:", label).unwrap();
            writeln!(file, "  Original: {}", original_path).unwrap();
            writeln!(file, "  Resolved: {}", resolved_path).unwrap();
            writeln!(file, "  Status: {}", status).unwrap();
            if size > 0 {
                writeln!(file, "  File size: {} bytes", size).unwrap();
            }
            writeln!(file).unwrap();
        }

        println!("Report saved to: {}", report_path.display());
    }

    assert!(
        success_count > 0,
        "At least some icons should be extracted successfully"
    );
}