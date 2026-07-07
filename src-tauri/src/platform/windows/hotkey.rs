//! 全局快捷键平台实现 - Windows
//! 优先使用 tauri-plugin-global-shortcut；如失败则回退到 Windows API

use crate::error::{AppError, Result};
use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};

/// 已注册快捷键的句柄
#[derive(Default)]
pub struct HotkeyState {
    pub current: Mutex<Option<String>>,
}

/// 通过 tauri GlobalShortcutManager 注册快捷键
pub fn register_via_tauri<R: Runtime>(
    app: &AppHandle<R>,
    hotkey: &str,
) -> Result<()> {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

    let manager = app.global_shortcut();
    let parsed = parse_hotkey_str(hotkey)?;

    // 先注销所有
    let _ = manager.unregister_all();

    manager
        .register(parsed)
        .map_err(|e| AppError::Other(format!("注册快捷键失败: {e}")))?;

    Ok(())
}

pub fn unregister_via_tauri<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let manager = app.global_shortcut();
    manager
        .unregister_all()
        .map_err(|e| AppError::Other(format!("注销快捷键失败: {e}")))?;
    Ok(())
}

/// 解析 "Alt+Space" 形式为 tauri Shortcut
pub fn parse_hotkey_str(s: &str) -> Result<tauri_plugin_global_shortcut::Shortcut> {
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return Err(AppError::InvalidInput("快捷键为空".into()));
    }

    let mut mods = Modifiers::empty();
    let mut key: Option<Code> = None;

    for part in &parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" => mods |= Modifiers::ALT,
            "meta" | "win" | "super" => mods |= Modifiers::META,
            _ => {
                key = Some(parse_key(part)?);
            }
        }
    }

    let key = key.ok_or_else(|| AppError::InvalidInput("快捷键缺少主键".into()))?;
    Ok(Shortcut::new(Some(mods), key))
}

fn parse_key(s: &str) -> Result<tauri_plugin_global_shortcut::Code> {
    use tauri_plugin_global_shortcut::Code;

    let upper = s.to_uppercase();
    let normalized = upper.replace(' ', "");

    // 字符 A-Z
    if normalized.len() == 1 {
        let c = normalized.chars().next().unwrap();
        if c.is_ascii_alphabetic() {
            return Ok(match c {
                'A' => Code::KeyA, 'B' => Code::KeyB, 'C' => Code::KeyC,
                'D' => Code::KeyD, 'E' => Code::KeyE, 'F' => Code::KeyF,
                'G' => Code::KeyG, 'H' => Code::KeyH, 'I' => Code::KeyI,
                'J' => Code::KeyJ, 'K' => Code::KeyK, 'L' => Code::KeyL,
                'M' => Code::KeyM, 'N' => Code::KeyN, 'O' => Code::KeyO,
                'P' => Code::KeyP, 'Q' => Code::KeyQ, 'R' => Code::KeyR,
                'S' => Code::KeyS, 'T' => Code::KeyT, 'U' => Code::KeyU,
                'V' => Code::KeyV, 'W' => Code::KeyW, 'X' => Code::KeyX,
                'Y' => Code::KeyY, 'Z' => Code::KeyZ,
                _ => unreachable!(),
            });
        }
        // 数字 0-9
        if c.is_ascii_digit() {
            return Ok(match c {
                '0' => Code::Digit0, '1' => Code::Digit1, '2' => Code::Digit2,
                '3' => Code::Digit3, '4' => Code::Digit4, '5' => Code::Digit5,
                '6' => Code::Digit6, '7' => Code::Digit7, '8' => Code::Digit8,
                '9' => Code::Digit9,
                _ => unreachable!(),
            });
        }
    }

    // 函数键与命名键
    match normalized.as_str() {
        "SPACE" => Ok(Code::Space),
        "TAB" => Ok(Code::Tab),
        "ESC" | "ESCAPE" => Ok(Code::Escape),
        "ENTER" | "RETURN" => Ok(Code::Enter),
        "F1" => Ok(Code::F1), "F2" => Ok(Code::F2), "F3" => Ok(Code::F3),
        "F4" => Ok(Code::F4), "F5" => Ok(Code::F5), "F6" => Ok(Code::F6),
        "F7" => Ok(Code::F7), "F8" => Ok(Code::F8), "F9" => Ok(Code::F9),
        "F10" => Ok(Code::F10), "F11" => Ok(Code::F11), "F12" => Ok(Code::F12),
        "F13" => Ok(Code::F13), "F14" => Ok(Code::F14), "F15" => Ok(Code::F15),
        "F16" => Ok(Code::F16), "F17" => Ok(Code::F17), "F18" => Ok(Code::F18),
        "F19" => Ok(Code::F19), "F20" => Ok(Code::F20),
        "F21" => Ok(Code::F21), "F22" => Ok(Code::F22),
        "F23" => Ok(Code::F23), "F24" => Ok(Code::F24),
        "BACKSPACE" | "BKSP" => Ok(Code::Backspace),
        "DELETE" | "DEL" => Ok(Code::Delete),
        "INSERT" | "INS" => Ok(Code::Insert),
        "HOME" => Ok(Code::Home),
        "END" => Ok(Code::End),
        "PAGEUP" | "PGUP" => Ok(Code::PageUp),
        "PAGEDOWN" | "PGDN" => Ok(Code::PageDown),
        "UP" => Ok(Code::ArrowUp),
        "DOWN" => Ok(Code::ArrowDown),
        "LEFT" => Ok(Code::ArrowLeft),
        "RIGHT" => Ok(Code::ArrowRight),
        _ => Err(AppError::InvalidInput(format!("未知按键: {}", s))),
    }
}
