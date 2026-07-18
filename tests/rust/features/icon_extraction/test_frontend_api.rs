use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use monotools_lib::platform::windows::{icon, shell};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

/// 根目录下的 `tests/output/icons/` — 与 `test.rs` 的 `get_output_dir` 用同样的
/// 显式绝对路径解析方式, 不依赖 cargo test 时的 CWD.
///
/// `env!("CARGO_MANIFEST_DIR")` = `src-tauri/`, `.parent()` = 仓库根,
/// `.join("tests/output/icons")` = 绝对路径, 不管从哪个目录跑测试都正确.
fn output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has a parent (workspace root)")
        .join("tests")
        .join("output")
        .join("icons")
}

fn collect_test_paths() -> Vec<(String, String)> {
    let mut paths: Vec<(String, String)> = Vec::new();

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

    let start_menu = Path::new("C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs");
    if let Ok(entries) = fs::read_dir(start_menu) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("lnk") {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    if paths.len() < 6 {
                        paths.push((
                            format!("startmenu_{}", name),
                            path.to_string_lossy().to_string(),
                        ));
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

pub async fn run_frontend_api_test() {
    let output_dir = output_dir();
    fs::create_dir_all(&output_dir).unwrap();

    let test_paths = collect_test_paths();
    println!("Testing {} paths with frontend API flow:", test_paths.len());

    let mut results = Vec::new();
    let mut success_count = 0;

    for (label, path) in &test_paths {
        println!("Processing: {} -> {}", label, path);

        let resolved_path = shell::resolve_shortcut(&std::path::PathBuf::from(path))
            .unwrap_or(std::path::PathBuf::from(path));
        let resolved_path_str = resolved_path.to_string_lossy().to_string();

        println!("  Resolved to: {}", resolved_path_str);

        let icon_result = icon::get_or_extract_cached(&resolved_path_str);

        match icon_result {
            Ok(Some(bytes)) => {
                let base64_str = BASE64.encode(&bytes);

                let sanitized_label = sanitize_filename(label);
                let output_path = output_dir.join(format!("{}.png", sanitized_label));

                if let Ok(mut file) = File::create(&output_path) {
                    if file.write_all(&bytes).is_ok() {
                        println!(
                            "  ✓ Extracted successfully, saved to: {}",
                            output_path.display()
                        );
                        println!(
                            "  Base64 length: {}, starts with: {}",
                            base64_str.len(),
                            base64_str.chars().take(20).collect::<String>()
                        );

                        let png_magic = "iVBORw0KGgo";
                        let starts_with_magic = base64_str.starts_with(png_magic);
                        println!("  Starts with PNG magic: {}", starts_with_magic);

                        let data_url = format!("data:image/png;base64,{}", base64_str);
                        println!("  Data URL length: {}", data_url.len());

                        results.push((
                            label.clone(),
                            path.clone(),
                            resolved_path_str.clone(),
                            "success".to_string(),
                            Some(base64_str.len()),
                            starts_with_magic,
                        ));
                        success_count += 1;
                    } else {
                        println!("  ✗ Failed to write file");
                        results.push((
                            label.clone(),
                            path.clone(),
                            resolved_path_str.clone(),
                            "write_failed".to_string(),
                            None,
                            false,
                        ));
                    }
                } else {
                    println!("  ✗ Failed to create output file");
                    results.push((
                        label.clone(),
                        path.clone(),
                        resolved_path_str.clone(),
                        "create_failed".to_string(),
                        None,
                        false,
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
                    None,
                    false,
                ));
            }
            Err(e) => {
                println!("  ✗ Error: {}", e);
                results.push((
                    label.clone(),
                    path.clone(),
                    resolved_path_str.clone(),
                    format!("error: {}", e),
                    None,
                    false,
                ));
            }
        }
    }

    let report_path = output_dir.join("report.txt");
    if let Ok(mut file) = File::create(&report_path) {
        writeln!(file, "Frontend API Icon Test Report").unwrap();
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

        writeln!(file, "Expected PNG magic: iVBORw0KGgo").unwrap();
        writeln!(file).unwrap();

        writeln!(file, "Detailed results:").unwrap();
        writeln!(file, "-----------------").unwrap();

        for (label, original_path, resolved_path, status, base64_len, has_magic) in results {
            writeln!(file, "{}:", label).unwrap();
            writeln!(file, "  Original: {}", original_path).unwrap();
            writeln!(file, "  Resolved: {}", resolved_path).unwrap();
            writeln!(file, "  Status: {}", status).unwrap();
            if let Some(len) = base64_len {
                writeln!(file, "  Base64 length: {}", len).unwrap();
                writeln!(file, "  Has PNG magic: {}", has_magic).unwrap();
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
