use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use std::collections::HashMap;

pub struct HotkeyService {
    registered: HashMap<String, u16>,
}

impl HotkeyService {
    pub fn new() -> Self {
        Self {
            registered: HashMap::new(),
        }
    }

    /// 注册全局热键
    pub fn register(
        &mut self,
        hwnd: HWND,
        id: &str,
        modifiers: u32,
        vk: u32,
    ) -> anyhow::Result<()> {
        unsafe {
            let atom = GlobalAddAtomW(&windows::core::HSTRING::from(id))?;
            RegisterHotKey(hwnd, atom as i32, modifiers, vk as u16)
                .map_err(|e| anyhow::anyhow!("Failed to register hotkey: {:?}", e))?;
            self.registered.insert(id.to_string(), atom as u16);
        }
        Ok(())
    }

    /// 注销热键
    pub fn unregister(&mut self, hwnd: HWND, id: &str) -> anyhow::Result<()> {
        if let Some(atom) = self.registered.remove(id) {
            unsafe {
                UnregisterHotKey(hwnd, atom as i32)?;
                GlobalDeleteAtom(atom)?;
            }
        }
        Ok(())
    }

    /// 处理热键消息
    pub fn handle_message(&self, wparam: WPARAM, lparam: LPARAM) -> Option<String> {
        let id = wparam.0 as u16;
        self.registered
            .iter()
            .find(|(_, v)| **v == id)
            .map(|(k, _)| k.clone())
    }
}

impl Default for HotkeyService {
    fn default() -> Self {
        Self::new()
    }
}

/// 热键修饰符常量
pub const MOD_ALT: u32 = MOD_ALT.0 as u32;
pub const MOD_CONTROL: u32 = MOD_CONTROL.0 as u32;
pub const MOD_SHIFT: u32 = MOD_SHIFT.0 as u32;
pub const MOD_WIN: u32 = MOD_WIN.0 as u32;

pub const VK_SPACE: u32 = VK_SPACE.0 as u32;
