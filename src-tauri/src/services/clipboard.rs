use tauri::AppHandle;

pub struct ClipboardService {
    app_handle: AppHandle,
}

impl ClipboardService {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    pub async fn read_text(&self) -> Result<String, String> {
        // TODO: 实现剪贴板读取
        Ok(String::new())
    }

    pub async fn write_text(&self, text: &str) -> Result<(), String> {
        // TODO: 实现剪贴板写入
        Ok(())
    }

    pub async fn read_image(&self) -> Result<Vec<u8>, String> {
        // TODO: 实现剪贴板图片读取
        Ok(vec![])
    }
}
