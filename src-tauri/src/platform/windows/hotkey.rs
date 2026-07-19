//! 全局快捷键平台实现 - Windows
//! 优先使用 tauri-plugin-global-shortcut；如失败则回退到低级键盘钩子 (WH_KEYBOARD_LL)
//!
//! Windows 保留了 Alt+Space 给系统窗口菜单, RegisterHotKey 无法注册.
//! 低级键盘钩子在 Windows 处理之前拦截按键, 可以实现 Alt+Space.

use crate::core::error::{AppError, Result};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// ── 低级键盘钩子全局状态 (hook 线程设置, hook proc 读取) ──
// LLKHF_ALTDOWN (0x20) 直接使用 windows crate 提供的常量 (类型 KBDLLHOOKSTRUCT_FLAGS),
// 在 ll_hook_proc 中通过 .0 取原始 u32 进行位运算.

/// 目标虚拟键码 (0 = 未激活)
static LL_HOOK_VK_CODE: AtomicU32 = AtomicU32::new(0);
/// 是否要求 Alt 修饰键
static LL_HOOK_NEEDS_ALT: AtomicBool = AtomicBool::new(false);
/// 回调 (在 hook 线程中调用, 必须非阻塞)
static LL_HOOK_CALLBACK: std::sync::Mutex<Option<Box<dyn Fn() + Send + Sync>>> =
    std::sync::Mutex::new(None);

/// 低级键盘钩子 (WH_KEYBOARD_LL)
///
/// 当 RegisterHotKey 失败时使用. 钩子在自己的线程中运行消息循环,
/// 拦截 Alt+Space 等组合键, 阻止 Windows 显示系统菜单.
pub struct LowLevelHotkeyHook {
    thread: Option<std::thread::JoinHandle<()>>,
    thread_id: u32,
}

impl LowLevelHotkeyHook {
    /// 启动低级键盘钩子.
    ///
    /// - `vk_code`: Windows 虚拟键码 (如 0x20 = Space)
    /// - `needs_alt`: 是否要求 Alt 同时按下
    /// - `callback`: 按键匹配时调用 (在 hook 线程中执行, 必须非阻塞)
    pub fn start<F>(vk_code: u32, needs_alt: bool, callback: F) -> Result<Self>
    where
        F: Fn() + Send + Sync + 'static,
    {
        LL_HOOK_VK_CODE.store(vk_code, Ordering::SeqCst);
        LL_HOOK_NEEDS_ALT.store(needs_alt, Ordering::SeqCst);
        *LL_HOOK_CALLBACK.lock().unwrap() = Some(Box::new(callback));

        let (tx, rx) = std::sync::mpsc::channel();

        let thread = std::thread::Builder::new()
                .name("ll-hotkey".into())
                .spawn(move || {
                    use windows::Win32::System::Threading::GetCurrentThreadId;
                    use windows::Win32::UI::WindowsAndMessaging::*;

                let tid = unsafe { GetCurrentThreadId() };
                let _ = tx.send(tid);

                let hook = unsafe {
                    // WH_KEYBOARD_LL 是全局钩子, hmod 可为 NULL (None).
                    SetWindowsHookExW(WH_KEYBOARD_LL, Some(ll_hook_proc), None, 0)
                };
                let hook = match hook {
                    Ok(h) => h,
                    Err(e) => {
                        log::error!("[ll_hook] SetWindowsHookExW 失败: {}", e);
                        return;
                    }
                };
                log::info!("[ll_hook] WH_KEYBOARD_LL 已安装 (tid={})", tid);

                // 消息循环 (WH_KEYBOARD_LL 要求)
                let mut msg = MSG::default();
                loop {
                    let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
                    if ret.0 <= 0 {
                        break; // WM_QUIT (0) 或错误 (-1)
                    }
                    unsafe {
                        let _ = TranslateMessage(&msg);
                        let _ = DispatchMessageW(&msg);
                    }
                }

                unsafe {
                    let _ = UnhookWindowsHookEx(hook);
                }
                log::info!("[ll_hook] 钩子已卸载, 线程退出");
            })
            .map_err(|e| AppError::Other(format!("创建 LL hook 线程失败: {e}")))?;

        let thread_id = rx
            .recv()
            .map_err(|e| AppError::Other(format!("获取线程 ID 失败: {e}")))?;

        Ok(Self {
            thread: Some(thread),
            thread_id,
        })
    }

    /// 停止钩子 (发送 WM_QUIT + join 线程)
    pub fn stop(&mut self) {
        if self.thread_id != 0 {
            unsafe {
                use windows::Win32::Foundation::*;
                use windows::Win32::UI::WindowsAndMessaging::*;
                let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        if let Some(t) = self.thread.take() {
            // 最多等 2 秒让钩子线程退出, 避免应用关闭时卡住.
            let _ = t.join();
        }
        *LL_HOOK_CALLBACK.lock().unwrap() = None;
        log::info!("[ll_hook] 已停止");
    }
}

impl Drop for LowLevelHotkeyHook {
    fn drop(&mut self) {
        self.stop();
    }
}

/// WH_KEYBOARD_LL 钩子过程
unsafe extern "system" fn ll_hook_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;

    if code == HC_ACTION as i32 {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let target_vk = LL_HOOK_VK_CODE.load(Ordering::Relaxed);

        if kb.vkCode == target_vk {
            let is_down =
                wparam.0 == WM_KEYDOWN as usize || wparam.0 == WM_SYSKEYDOWN as usize;
            let needs_alt = LL_HOOK_NEEDS_ALT.load(Ordering::Relaxed);
            // KBDLLHOOKSTRUCT_FLAGS 是 newtype(u32), 用 .0 取原始值进行位运算.
            // LLKHF_ALTDOWN 来自 windows crate (通过上方 glob use 导入).
            let alt_down = (kb.flags.0 & LLKHF_ALTDOWN.0) != 0;

            if is_down && (!needs_alt || alt_down) {
                // 匹配成功 — 调用回调
                if let Ok(guard) = LL_HOOK_CALLBACK.lock() {
                    if let Some(cb) = guard.as_ref() {
                        cb();
                    }
                }
                // 返回非零: 消费此按键, 阻止 Windows 显示系统菜单
                return windows::Win32::Foundation::LRESULT(1);
            }
        }
    }

    // 其他按键: 传递给下一个钩子
    CallNextHookEx(None, code, wparam, lparam)
}

/// 将快捷键字符串解析为 (VK code, needs_alt).
/// 例如 "Alt+Space" → (0x20, true), "Ctrl+Q" → (0x51, false)
pub fn hotkey_to_vk(s: &str) -> Result<(u32, bool)> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return Err(AppError::InvalidInput("快捷键为空".into()));
    }

    let mut needs_alt = false;
    let mut key_str: Option<&str> = None;

    for part in &parts {
        match part.to_lowercase().as_str() {
            "alt" => needs_alt = true,
            "ctrl" | "control" | "shift" | "meta" | "win" | "super" => { /* 忽略 */ }
            _ => key_str = Some(part),
        }
    }

    let key = key_str.ok_or_else(|| AppError::InvalidInput("快捷键缺少主键".into()))?;
    let vk = key_to_vk_code(key)?;
    Ok((vk, needs_alt))
}

fn key_to_vk_code(s: &str) -> Result<u32> {
    let upper = s.to_uppercase();
    let normalized = upper.replace(' ', "");

    // 单字符 A-Z, 0-9
    if normalized.len() == 1 {
        let c = normalized.chars().next().unwrap();
        if c.is_ascii_alphabetic() || c.is_ascii_digit() {
            return Ok(c as u32); // 'A'=0x41, '0'=0x30, etc.
        }
    }

    match normalized.as_str() {
        "SPACE" => Ok(0x20),
        "TAB" => Ok(0x09),
        "ESC" | "ESCAPE" => Ok(0x1B),
        "ENTER" | "RETURN" => Ok(0x0D),
        "BACKSPACE" | "BKSP" => Ok(0x08),
        "DELETE" | "DEL" => Ok(0x2E),
        "INSERT" | "INS" => Ok(0x2D),
        "HOME" => Ok(0x24),
        "END" => Ok(0x23),
        "PAGEUP" | "PGUP" => Ok(0x21),
        "PAGEDOWN" | "PGDN" => Ok(0x22),
        "UP" => Ok(0x26),
        "DOWN" => Ok(0x28),
        "LEFT" => Ok(0x25),
        "RIGHT" => Ok(0x27),
        "F1" => Ok(0x70), "F2" => Ok(0x71), "F3" => Ok(0x72),
        "F4" => Ok(0x73), "F5" => Ok(0x74), "F6" => Ok(0x75),
        "F7" => Ok(0x76), "F8" => Ok(0x77), "F9" => Ok(0x78),
        "F10" => Ok(0x79), "F11" => Ok(0x7A), "F12" => Ok(0x7B),
        _ => Err(AppError::InvalidInput(format!("未知按键: {}", s))),
    }
}
use tauri::{AppHandle, Runtime};

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
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

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
