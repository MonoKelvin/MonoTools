//! NTFS USN Journal 和 MFT 枚举 - 实现类似 Everything 的快速文件索引
//!
//! 核心架构：
//! 1. 全量索引：使用 FSCTL_ENUM_USN_DATA 批量读取 MFT，这是最快的全盘枚举方式
//! 2. 增量更新：使用 FSCTL_READ_USN_JOURNAL 读取变更记录
//! 3. 路径重建：通过父文件引用号构建完整路径（两遍扫描：先目录后文件）
//!
//! 参考：https://learn.microsoft.com/en-us/windows/win32/api/winioctl/

use crate::core::error::{AppError, Result};
use std::collections::HashMap;
use std::ffi::{c_void, OsString};
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::sync::Arc;

const FSCTL_QUERY_USN_JOURNAL: u32 = 0x000900F4;
const FSCTL_ENUM_USN_DATA: u32 = 0x000900B3;
const FSCTL_READ_USN_JOURNAL: u32 = 0x000900B8;
const FSCTL_CREATE_USN_JOURNAL: u32 = 0x000900E4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsnChangeReason {
    Created,
    Deleted,
    Modified,
    RenamedOldName,
    RenamedNewName,
}

#[derive(Debug, Clone)]
pub struct UsnRecord {
    pub file_reference_number: u64,
    pub parent_file_reference: u64,
    pub file_name: String,
    pub full_path: PathBuf,
    pub file_size: u64,
    pub last_write_time: i64,
    pub is_directory: bool,
    pub extension: Option<String>,
    pub reason: UsnChangeReason,
    pub usn: u64,
}

#[derive(Debug, Clone)]
pub struct UsnJournalState {
    pub usn: u64,
    pub next_usn: u64,
    pub first_usn: u64,
    pub journal_id: u64,
    pub max_size: u64,
    pub allocation_delta: u64,
}

fn extract_drive_letter(volume: &str) -> char {
    if volume.starts_with("\\\\.\\") && volume.len() >= 6 {
        volume.chars().nth(4).unwrap_or('C')
    } else if volume.starts_with("\\\\?\\Volume") {
        use windows_sys::Win32::Storage::FileSystem::{
            FindFirstVolumeMountPointW, FindVolumeMountPointClose,
        };

        let mut mount_point: [u16; 512] = [0; 512];
        let wide_path: Vec<u16> = volume.encode_utf16().chain(std::iter::once(0)).collect();
        let hfind = unsafe {
            FindFirstVolumeMountPointW(
                wide_path.as_ptr(),
                mount_point.as_mut_ptr(),
                mount_point.len() as u32,
            )
        };

        if hfind != windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            let mount_str = String::from_utf16_lossy(&mount_point).trim().to_string();
            unsafe { FindVolumeMountPointClose(hfind) };
            if let Some(c) = mount_str.chars().next() {
                if c.is_ascii_alphabetic() {
                    return c.to_ascii_uppercase();
                }
            }
        }
        'C'
    } else {
        'C'
    }
}

/// 从 volume 路径获取盘符字母和卷标名，格式化为 "卷标 (C:)" 形式。
/// 若无法获取卷标名，则回退到 "本地磁盘 (C:)"。
/// 当 volume 字符串无法解析时回退到 `"?"`。
///
/// 使用 `GetVolumeInformationW` 获取卷标，这是 Windows 标准 API，
/// 开销极小（微秒级），适合在索引进度回调中频繁调用。
pub fn drive_label(volume: &str) -> String {
    let c = extract_drive_letter(volume).to_ascii_uppercase();
    let drive_path = format!("{}:\\", c);

    // 尝试获取卷标名
    let wide_path: Vec<u16> = drive_path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut volume_name: [u16; 256] = [0; 256];
    let success = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW(
            wide_path.as_ptr(),
            volume_name.as_mut_ptr(),
            volume_name.len() as u32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        )
    };

    if success != 0 {
        let name_len = volume_name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(volume_name.len());
        let name = String::from_utf16_lossy(&volume_name[..name_len])
            .trim()
            .to_string();
        if !name.is_empty() {
            return format!("{} ({})", name, c);
        }
    }

    // 回退: 本地磁盘 + 盘符
    format!("本地磁盘 ({})", c)
}

pub struct NtfsIndexer {
    /// 已枚举到的盘符列表。初始为 `lazy_initial`，第一次访问时才真正调用 `enumerate_ntfs_volumes`。
    /// - 用 `std::sync::Once` + `parking_lot::Mutex` 保证只枚举一次。
    volumes: parking_lot::Mutex<Vec<String>>,
    /// 已经"懒初始化"过的标记; 配合 `initialize_now()` 调用一次后即就绪.
    initialized: std::sync::atomic::AtomicBool,
    last_usn: parking_lot::RwLock<HashMap<String, u64>>,
    path_cache: Arc<parking_lot::RwLock<HashMap<u64, PathBuf>>>,
    /// 指定盘符(非懒加载路径): 启动时同步枚举.
    explicit_drives: Option<Vec<char>>,
}

impl NtfsIndexer {
    /// 不进行任何 I/O 的空构造 —— 真正的盘符枚举会在第一次访问 `get_volumes()` 时进行。
    pub fn new_lazy() -> Result<Self> {
        Ok(Self {
            volumes: parking_lot::Mutex::new(Vec::new()),
            initialized: std::sync::atomic::AtomicBool::new(false),
            last_usn: parking_lot::RwLock::new(HashMap::new()),
            path_cache: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            explicit_drives: None,
        })
    }

    /// 用户**显式指定**了盘符: 懒枚举(同步只在用户显式调用 `initialize_now()` 时).
    pub fn new_with_drives_lazy(drives: Vec<char>) -> Self {
        Self {
            volumes: parking_lot::Mutex::new(Vec::new()),
            initialized: std::sync::atomic::AtomicBool::new(false),
            last_usn: parking_lot::RwLock::new(HashMap::new()),
            path_cache: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            explicit_drives: Some(drives),
        }
    }

    /// 兼容旧路径: 同步枚举所有 NTFS 卷。
    pub fn new() -> Result<Self> {
        log::info!("[usn] NtfsIndexer::new(): 同步枚举 NTFS 卷（仅用于显式 legacy 路径）");
        let vols = Self::enumerate_ntfs_volumes(None)?;
        log::info!("[usn] NTFS索引器已创建，检测到 {} 个NTFS卷", vols.len());
        Ok(Self {
            volumes: parking_lot::Mutex::new(vols),
            initialized: std::sync::atomic::AtomicBool::new(true),
            last_usn: parking_lot::RwLock::new(HashMap::new()),
            path_cache: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            explicit_drives: None,
        })
    }

    pub fn new_with_drives(drives: Vec<char>) -> Result<Self> {
        log::info!("[usn] NtfsIndexer::new_with_drives(): 同步枚举指定盘符");
        let vols = Self::enumerate_ntfs_volumes(Some(drives.clone()))?;
        Ok(Self {
            volumes: parking_lot::Mutex::new(vols),
            initialized: std::sync::atomic::AtomicBool::new(true),
            last_usn: parking_lot::RwLock::new(HashMap::new()),
            path_cache: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            explicit_drives: Some(drives),
        })
    }

    /// 主动触发**且仅触发一次**的真正 NTFS 卷枚举。
    /// 必须在后台线程中调用 —— `enumerate_ntfs_volumes` 是同步阻塞 API。
    /// 之后的 `get_volumes()` 会直接返回缓存，不会重复枚举。
    pub fn ensure_enumerated(&self) {
        use std::sync::atomic::Ordering;
        if self.initialized.swap(true, Ordering::SeqCst) {
            return;
        }
        log::info!("[usn] ensure_enumerated: 第一次访问, 同步枚举 NTFS 卷");

        let result = if let Some(drives) = self.explicit_drives.as_ref() {
            Self::enumerate_ntfs_volumes(Some(drives.clone()))
        } else {
            Self::enumerate_ntfs_volumes(None)
        };
        match result {
            Ok(v) => {
                log::info!("[usn] ensure_enumerated 完成: {} 个卷", v.len());
                *self.volumes.lock() = v;
            }
            Err(e) => {
                log::error!("[usn] ensure_enumerated 失败: {}, 后续将无法构建索引", e);
                // 失败也标记为 initialized=true 避免反复触发; volume 列表保持空.
            }
        }
    }

    fn enumerate_ntfs_volumes(drives: Option<Vec<char>>) -> Result<Vec<String>> {
        use windows_sys::Win32::Storage::FileSystem::{
            FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, GetVolumeInformationW,
        };

        let mut volumes = Vec::new();

        if let Some(specified_drives) = drives {
            for drive_char in specified_drives {
                let drive_letter = drive_char.to_ascii_uppercase();
                let drive_path = format!("{}:\\", drive_letter);
                let mut fs_name: [u16; 256] = [0; 256];
                let wide_path: Vec<u16> = drive_path
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();

                let success = unsafe {
                    GetVolumeInformationW(
                        wide_path.as_ptr(),
                        std::ptr::null_mut(),
                        0,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        fs_name.as_mut_ptr(),
                        fs_name.len() as u32,
                    )
                };

                if success != 0 {
                    let fs_len = fs_name
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(fs_name.len());
                    let fs = String::from_utf16_lossy(&fs_name[..fs_len])
                        .trim()
                        .to_string();
                    log::info!("{} 文件系统: {}", drive_path, fs);
                    if fs.eq_ignore_ascii_case("NTFS") {
                        let volume = format!("\\\\.\\{}:", drive_letter);
                        log::info!("检测到NTFS卷: {}", volume);
                        volumes.push(volume);
                    } else {
                        log::warn!("{} 文件系统不是NTFS: {}", drive_path, fs);
                        volumes.push(format!("\\\\.\\{}:", drive_letter));
                    }
                } else {
                    let last_error = unsafe { windows_sys::Win32::Foundation::GetLastError() };
                    log::warn!("无法获取 {} 驱动器信息，错误码: {}", drive_path, last_error);
                    volumes.push(format!("\\\\.\\{}:", drive_letter));
                }
            }
        } else {
            let mut volume_name: [u16; 512] = [0; 512];
            let hfind =
                unsafe { FindFirstVolumeW(volume_name.as_mut_ptr(), volume_name.len() as u32) };

            if hfind == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
                log::warn!("无法枚举系统卷");
                return Ok(volumes);
            }

            loop {
                let vol_name = String::from_utf16_lossy(&volume_name).trim().to_string();

                let mut mount_point: [u16; 512] = [0; 512];
                let hfind_mount = unsafe {
                    windows_sys::Win32::Storage::FileSystem::FindFirstVolumeMountPointW(
                        volume_name.as_ptr(),
                        mount_point.as_mut_ptr(),
                        mount_point.len() as u32,
                    )
                };

                let has_drive_letter =
                    hfind_mount != windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
                if hfind_mount != windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
                    unsafe {
                        windows_sys::Win32::Storage::FileSystem::FindVolumeMountPointClose(
                            hfind_mount,
                        )
                    };
                }

                let wide_path: Vec<u16> =
                    vol_name.encode_utf16().chain(std::iter::once(0)).collect();
                let mut fs_name: [u16; 256] = [0; 256];
                let success = unsafe {
                    GetVolumeInformationW(
                        wide_path.as_ptr(),
                        std::ptr::null_mut(),
                        0,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        fs_name.as_mut_ptr(),
                        fs_name.len() as u32,
                    )
                };

                if success != 0 {
                    let fs = String::from_utf16_lossy(&fs_name).trim().to_string();
                    if fs.eq_ignore_ascii_case("NTFS") && has_drive_letter {
                        log::info!("检测到NTFS卷: {}", vol_name);
                        volumes.push(vol_name);
                    }
                }

                let success = unsafe {
                    FindNextVolumeW(hfind, volume_name.as_mut_ptr(), volume_name.len() as u32)
                };
                if success == 0 {
                    break;
                }
            }

            unsafe { FindVolumeClose(hfind) };

            // Fallback：直接枚举所有逻辑盘符（动态发现，不依赖盘符前缀白名单）。
            // 之所以保留 fallback，是因为在某些虚拟机 / 容器 / 安全软件拦截下
            // `FindFirstVolumeW` 可能返回 0 个 NTFS 卷，但 `GetLogicalDriveStringsW` 仍可工作。
            // Fallback 策略：尝试**所有**卷(不强制 NTFS), 因为:
            //   1. 不依赖 FS 类型, 系统其它格式(exFAT/FAT32 等)虽然不能用 USN, 但其它可枚举;
            //   2. GetVolumeInformationW 失败 / 权限不足的盘符也加入列表,
            //      由 FileSearchEngine::enumerate_volume 自行跳过无权访问的卷。
            //
            // 注: 缓冲区 512 字符在我们机器上够用, 但 Windows API 在多盘 + 长 mount path
            // 场景下需要大得多的 buffer. 我们**总是**走"多次重试直到装下"策略, 而不是
            // 只用 512 字符固定大小。这样能稳健拿到 C/D/E/F/...
            if volumes.is_empty() {
                Self::enumerate_logical_drives(&mut volumes);
            }
        }

        log::info!(
            "NTFS卷检测完成，共发现 {} 个卷: {:?}",
            volumes.len(),
            volumes
        );
        Ok(volumes)
    }

    /**
     * 通过 `GetLogicalDriveStringsW` 枚举所有逻辑盘符（C:、D:、E: ...），
     * 自适应缓冲区大小 — 直到能装下所有路径为止。
     */
    fn enumerate_logical_drives(out: &mut Vec<String>) {
        use windows_sys::Win32::Storage::FileSystem::GetLogicalDriveStringsW;

        let mut buf_len: u32 = 512;
        let mut buf: Vec<u16> = vec![0u16; buf_len as usize];
        loop {
            let len = unsafe { GetLogicalDriveStringsW(buf_len, buf.as_mut_ptr()) };
            if len == 0 {
                return;
            }
            if len <= buf_len {
                // len 不含结尾 0; 解析路径列表.
                let mut i = 0usize;
                while i < len as usize {
                    let start = i;
                    while i < len as usize && buf[i] != 0 {
                        i += 1;
                    }
                    if i > start {
                        let name = std::ffi::OsString::from_wide(&buf[start..i]);
                        let s = name.to_string_lossy().to_string();
                        if s.len() >= 3
                            && s.as_bytes()[0].is_ascii_alphabetic()
                            && s.as_bytes()[1] == b':'
                            && s.as_bytes()[2] == b'\\'
                        {
                            let drive_letter = s.chars().next().unwrap();
                            // 该卷是否 NTFS?
                            let mut fs_name: [u16; 256] = [0; 256];
                            let wide_path: Vec<u16> =
                                s.encode_utf16().chain(std::iter::once(0)).collect();
                            let success = unsafe {
                                windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW(
                                    wide_path.as_ptr(),
                                    std::ptr::null_mut(),
                                    0,
                                    std::ptr::null_mut(),
                                    std::ptr::null_mut(),
                                    std::ptr::null_mut(),
                                    fs_name.as_mut_ptr(),
                                    fs_name.len() as u32,
                                )
                            };
                            if success != 0 {
                                let fs = String::from_utf16_lossy(&fs_name).trim().to_string();
                                let volume = format!("\\\\.\\{}:", drive_letter);
                                log::info!(
                                    "fallback 检测到卷 {} (FS: {})",
                                    volume,
                                    fs
                                );
                                out.push(volume);
                            }
                        }
                    }
                    if i < len as usize && buf[i] == 0 {
                        i += 1;
                    }
                }
                return;
            }
            // 不够装: 翻倍扩容再试 (Windows 文档: 0 = error, > buffer_size = 重新分配).
            buf_len = (buf_len as usize * 2).min(16384) as u32;
            if buf_len == 0 {
                return;
            }
            buf = vec![0u16; buf_len as usize];
        }
    }

    pub fn get_journal_state(&self, volume: &str) -> Option<UsnJournalState> {
        use windows_sys::Win32::Foundation::GENERIC_READ;
        use windows_sys::Win32::Storage::FileSystem::{
            CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_READ, FILE_SHARE_WRITE,
            OPEN_EXISTING,
        };
        use windows_sys::Win32::System::IO::DeviceIoControl;

        let wide_path: Vec<u16> = volume.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                std::ptr::null_mut(),
            )
        };

        if handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            let last_error = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            log::warn!(
                "无法打开卷 {} 查询 USN Journal 状态，错误码: {}",
                volume,
                last_error
            );
            return None;
        }

        let mut state = USN_JOURNAL_DATA_V2::default();
        let mut bytes_returned: u32 = 0;

        let success = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_QUERY_USN_JOURNAL,
                std::ptr::null_mut(),
                0,
                &mut state as *mut _ as *mut c_void,
                std::mem::size_of::<USN_JOURNAL_DATA_V2>() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }

        if success == 0 {
            return None;
        }

        Some(UsnJournalState {
            usn: state.NextUsn as u64,
            next_usn: state.NextUsn as u64,
            first_usn: state.FirstUsn as u64,
            journal_id: state.UsnJournalID,
            max_size: state.MaximumSize,
            allocation_delta: state.AllocationDelta,
        })
    }

    pub fn enumerate_volume_files(
        &self,
        volume: &str,
        mut callback: impl FnMut(UsnRecord),
    ) -> Result<()> {
        use windows_sys::Win32::Foundation::{GENERIC_READ, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_READ,
            FILE_SHARE_WRITE, OPEN_EXISTING,
        };
        use windows_sys::Win32::System::IO::DeviceIoControl;

        log::info!("开始枚举卷文件 (MFT枚举 - 两遍扫描): {}", volume);

        let journal_state = self.get_usn_journal_state(volume);
        let max_usn: i64 = if let Some(js) = &journal_state {
            log::info!(
                "USN Journal 状态: journal_id={}, next_usn={}",
                js.journal_id,
                js.next_usn
            );
            js.next_usn as i64
        } else {
            log::warn!("无法获取 USN Journal 状态，使用 0 作为 HighUsn");
            0
        };

        let wide_path: Vec<u16> = volume.encode_utf16().chain(std::iter::once(0)).collect();

        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            let last_error = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            if last_error == 5 {
                return Err(AppError::PermissionDenied);
            }
            return Err(AppError::Other(format!(
                "无法打开卷 {}，错误码: {}",
                volume, last_error
            )));
        }

        let drive_letter = extract_drive_letter(volume);
        let root_path = PathBuf::from(format!("{}:\\", drive_letter));

        let mut path_cache = self.path_cache.write();
        path_cache.clear();
        path_cache.insert(0, root_path.clone());

        let mut dir_records: Vec<(u64, u64, String, i64)> = Vec::new();
        let mut file_records: Vec<(u64, u64, String, i64)> = Vec::new();

        let mut buffer: Vec<u8> = vec![0; 1024 * 1024];
        let mut current_file_ref: u64 = 0;

        // 第一遍扫描：收集所有 MFT 记录（分离目录和文件）
        log::debug!("第一遍扫描：收集 MFT 记录...");
        loop {
            let query = MFT_ENUM_DATA {
                StartFileReferenceNumber: current_file_ref,
                LowUsn: 0,
                HighUsn: max_usn,
            };

            let mut bytes_returned: u32 = 0;
            let success = unsafe {
                DeviceIoControl(
                    handle,
                    FSCTL_ENUM_USN_DATA,
                    &query as *const _ as *mut c_void,
                    std::mem::size_of::<MFT_ENUM_DATA>() as u32,
                    buffer.as_mut_ptr() as *mut c_void,
                    buffer.len() as u32,
                    &mut bytes_returned,
                    std::ptr::null_mut(),
                )
            };

            if success == 0 || bytes_returned == 0 {
                let last_error = if success == 0 {
                    unsafe { windows_sys::Win32::Foundation::GetLastError() }
                } else {
                    0
                };
                log::debug!(
                    "FSCTL_ENUM_USN_DATA 结束 - success={}, bytes={}, error={}, dirs={}, files={}",
                    success,
                    bytes_returned,
                    last_error,
                    dir_records.len(),
                    file_records.len()
                );
                break;
            }

            let next_file_ref = unsafe { *(buffer.as_ptr() as *const u64) };

            let mut offset = std::mem::size_of::<u64>();

            while offset < bytes_returned as usize {
                let record = unsafe { &*(buffer.as_ptr().add(offset) as *const USN_RECORD_V2) };

                if record.RecordLength == 0 {
                    break;
                }

                let name_len = (record.FileNameLength / 2) as usize;
                let name_slice =
                    unsafe { std::slice::from_raw_parts(record.FileName.as_ptr(), name_len) };
                let name = OsString::from_wide(name_slice);
                let file_name = name.to_string_lossy().trim().to_string();

                if !file_name.starts_with('$') {
                    let is_directory = record.FileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0;
                    let entry = (
                        record.FileReferenceNumber,
                        record.ParentFileReferenceNumber,
                        file_name,
                        record.TimeStamp,
                    );

                    if is_directory {
                        dir_records.push(entry);
                    } else {
                        file_records.push(entry);
                    }
                }

                offset += record.RecordLength as usize;
            }

            if next_file_ref == 0 || next_file_ref == current_file_ref {
                break;
            }
            current_file_ref = next_file_ref;
        }

        log::info!(
            "第一遍扫描完成：{} 个目录，{} 个文件",
            dir_records.len(),
            file_records.len()
        );

        // 动态检测根目录的 FRN
        // 根目录的特点：
        // 1. ParentFileReferenceNumber 不指向任何已知目录
        // 2. ParentFileReferenceNumber 等于它自己的 FRN（根目录的父就是自己）
        // 3. 常见值是 5 或 0
        let all_dir_frns: std::collections::HashSet<u64> =
            dir_records.iter().map(|(frn, _, _, _)| *frn).collect();

        let mut root_frns: std::collections::HashSet<u64> = std::collections::HashSet::new();

        for (file_ref, parent_ref, _, _) in &dir_records {
            if !all_dir_frns.contains(parent_ref) || file_ref == parent_ref {
                root_frns.insert(*parent_ref);
            }
        }

        // 如果上面的方法没找到，试试常见值（NTFS 根目录通常是 5）
        if root_frns.is_empty() {
            for candidate in [5u64, 0u64] {
                if dir_records
                    .iter()
                    .any(|(frn, parent, _, _)| *frn == candidate || *parent == candidate)
                {
                    root_frns.insert(candidate);
                }
            }
        }

        log::info!("检测到根目录 FRN: {:?}", root_frns);

        // 清除并重新初始化 path_cache，使用所有检测到的根目录 FRN
        path_cache.clear();
        for frn in &root_frns {
            path_cache.insert(*frn, root_path.clone());
        }

        if root_frns.is_empty() {
            // fallback：尝试用 0 和 5
            path_cache.insert(0, root_path.clone());
            path_cache.insert(5, root_path.clone());
            log::warn!("无法确定根目录 FRN，使用 fallback (0 和 5)");
        }

        // 按层级排序目录（BFS顺序构建路径缓存）
        dir_records.sort_by_key(|(_, parent_ref, _, _)| *parent_ref);

        // 第二遍：构建目录路径缓存（迭代直到稳定）
        log::debug!("第二遍扫描：构建目录路径缓存...");
        let mut changed = true;
        let mut iterations = 0;
        while changed && iterations < 100 {
            changed = false;
            iterations += 1;

            for (file_ref, parent_ref, file_name, _) in &dir_records {
                if path_cache.contains_key(file_ref) {
                    continue;
                }
                if let Some(parent_path) = path_cache.get(parent_ref) {
                    let full_path = parent_path.join(file_name);
                    path_cache.insert(*file_ref, full_path);
                    changed = true;
                }
            }
        }

        let dirs_cached = path_cache.len() - 1; // 减去根目录
        log::debug!(
            "目录路径缓存构建完成：{} 个目录已缓存（迭代 {} 次）",
            dirs_cached,
            iterations
        );

        // 第三遍：处理文件并调用 callback
        log::debug!("第三遍扫描：处理文件记录...");
        let mut total_count = 0u64;
        let mut skipped_no_parent = 0u64;

        for (file_ref, parent_ref, file_name, timestamp) in &dir_records {
            if let Some(full_path) = path_cache.get(file_ref) {
                let usn_record = UsnRecord {
                    file_reference_number: *file_ref,
                    parent_file_reference: *parent_ref,
                    file_name: file_name.clone(),
                    full_path: full_path.clone(),
                    file_size: 0,
                    last_write_time: *timestamp / 10000000 - 11644473600,
                    is_directory: true,
                    extension: None,
                    reason: UsnChangeReason::Created,
                    usn: 0,
                };
                callback(usn_record);
                total_count += 1;
            }
        }

        for (file_ref, parent_ref, file_name, timestamp) in &file_records {
            if let Some(parent_path) = path_cache.get(parent_ref) {
                let full_path = parent_path.join(file_name);
                let ext = full_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase());

                let usn_record = UsnRecord {
                    file_reference_number: *file_ref,
                    parent_file_reference: *parent_ref,
                    file_name: file_name.clone(),
                    full_path,
                    file_size: 0,
                    last_write_time: *timestamp / 10000000 - 11644473600,
                    is_directory: false,
                    extension: ext,
                    reason: UsnChangeReason::Created,
                    usn: 0,
                };
                callback(usn_record);
                total_count += 1;
            } else {
                skipped_no_parent += 1;
            }
        }

        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }

        let file_count = total_count.saturating_sub(dirs_cached as u64);
        log::info!(
            "MFT 枚举完成: {} 个条目（目录{} + 文件{}），跳过无父路径: {}",
            total_count,
            dirs_cached,
            file_count,
            skipped_no_parent
        );

        if total_count == 0 {
            log::warn!("USN Journal 返回空结果");
        }

        Ok(())
    }

    pub fn read_usn_changes(&self, volume: &str, start_usn: u64) -> Result<Vec<UsnRecord>> {
        use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE};
        use windows_sys::Win32::Storage::FileSystem::{
            CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        };
        use windows_sys::Win32::System::IO::DeviceIoControl;

        let wide_path: Vec<u16> = volume.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };

        if handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            let last_error = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            log::warn!(
                "无法打开卷 {} 读取 USN 变化，错误码: {}",
                volume,
                last_error
            );
            return Err(AppError::Other(format!(
                "无法打开卷 {} (错误码: {})",
                volume, last_error
            )));
        }

        let mut journal_state = USN_JOURNAL_DATA_V2::default();
        let mut bytes_returned: u32 = 0;

        let query_success = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_QUERY_USN_JOURNAL,
                std::ptr::null_mut(),
                0,
                &mut journal_state as *mut _ as *mut c_void,
                std::mem::size_of::<USN_JOURNAL_DATA_V2>() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        if query_success == 0 {
            let last_error = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            log::warn!(
                "[usn] FSCTL_QUERY_USN_JOURNAL 失败 (卷={}, 错误码={})",
                volume,
                last_error
            );
            unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
            return Err(AppError::Other(format!(
                "查询 USN Journal 失败 (卷={}, 错误码={})",
                volume, last_error
            )));
        }

        let journal_id = journal_state.UsnJournalID;

        let mut buffer: Vec<u8> = vec![0; 1024 * 1024];
        bytes_returned = 0;

        let drive_letter = extract_drive_letter(volume);
        let root_path = PathBuf::from(format!("{}:\\", drive_letter));

        let mut changes = Vec::new();

        let query = READ_USN_JOURNAL_DATA_V0 {
            StartUsn: start_usn as i64,
            ReasonMask: 0xFFFFFFFF,
            UsnJournalID: journal_id,
        };

        let mut success = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_READ_USN_JOURNAL,
                &query as *const _ as *mut c_void,
                std::mem::size_of::<READ_USN_JOURNAL_DATA_V0>() as u32,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        if success == 0 {
            let mut enum_data = MFT_ENUM_DATA {
                StartFileReferenceNumber: 0,
                LowUsn: start_usn as i64,
                HighUsn: i64::MAX,
            };

            bytes_returned = 0;
            success = unsafe {
                DeviceIoControl(
                    handle,
                    FSCTL_ENUM_USN_DATA,
                    &mut enum_data as *mut _ as *mut c_void,
                    std::mem::size_of::<MFT_ENUM_DATA>() as u32,
                    buffer.as_mut_ptr() as *mut c_void,
                    buffer.len() as u32,
                    &mut bytes_returned,
                    std::ptr::null_mut(),
                )
            };
        }

        if success == 0 {
            let last_error = unsafe { windows_sys::Win32::Foundation::GetLastError() };

            // Journal Wrap 检测: 错误码 1117 = ERROR_JOURNAL_WRAP_DELETED
            // 表示 USN Journal 已被删除或回绕, 需要全量重新索引.
            if last_error == 1117 {
                log::warn!(
                    "[usn] Journal Wrap 检测到 (卷={}, 错误码=1117), 触发全量重新索引",
                    volume
                );
                unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
                return Err(AppError::Other(format!(
                    "USN Journal Wrap 检测到, 需要全量重新索引 (卷={})",
                    volume
                )));
            }

            log::warn!(
                "[usn] FSCTL_READ_USN_JOURNAL + FSCTL_ENUM_USN_DATA fallback 均失败 (卷={}, 错误码={})",
                volume,
                last_error
            );
            unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
            return Err(AppError::Other(format!(
                "读取 USN 变更失败 (卷={}, 错误码={})",
                volume, last_error
            )));
        }

        log::debug!(
            "FSCTL_READ_USN_JOURNAL/FSCTL_ENUM_USN_DATA 成功，返回 {} 字节",
            bytes_returned
        );

        let mut offset = 8;
        let mut dir_records: Vec<(u64, u64, String, u64)> = Vec::new();
        let mut file_records: Vec<(u64, u64, String, u32, i64, u64)> = Vec::new();

        while offset < bytes_returned as usize {
            let record = unsafe { &*(buffer.as_ptr().add(offset) as *const USN_RECORD_V2) };

            if record.RecordLength == 0 {
                break;
            }

            let name_len = (record.FileNameLength / 2) as usize;
            let name_slice =
                unsafe { std::slice::from_raw_parts(record.FileName.as_ptr(), name_len) };
            let name = OsString::from_wide(name_slice);
            let file_name = name.to_string_lossy().trim().to_string();

            if file_name.starts_with('.') || file_name.starts_with('$') {
                offset += record.RecordLength as usize;
                continue;
            }

            let is_directory = record.FileAttributes
                & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY
                != 0;

            if is_directory {
                dir_records.push((
                    record.FileReferenceNumber,
                    record.ParentFileReferenceNumber,
                    file_name,
                    record.Usn as u64,
                ));
            } else {
                file_records.push((
                    record.FileReferenceNumber,
                    record.ParentFileReferenceNumber,
                    file_name,
                    record.Reason,
                    record.TimeStamp,
                    record.Usn as u64,
                ));
            }

            offset += record.RecordLength as usize;
        }

        // 关键: 增量更新时使用局部 path_cache 而非清空全局缓存.
        // 清空全局缓存会导致其他卷的路径信息丢失, 造成后续搜索失败.
        let mut local_path_cache: std::collections::HashMap<u64, PathBuf> = HashMap::new();
        local_path_cache.insert(0, root_path.clone());

        let all_dir_frns: std::collections::HashSet<u64> =
            dir_records.iter().map(|(frn, _, _, _)| *frn).collect();
        let mut root_frn: Option<u64> = None;
        for (_, parent_ref, _, _) in &dir_records {
            if !all_dir_frns.contains(parent_ref) {
                root_frn = Some(*parent_ref);
                break;
            }
        }

        if root_frn.is_none() {
            for candidate in [5u64, 0u64] {
                if dir_records
                    .iter()
                    .any(|(_, parent, _, _)| *parent == candidate)
                {
                    root_frn = Some(candidate);
                    break;
                }
            }
        }

        if let Some(frn) = root_frn {
            local_path_cache.insert(frn, root_path.clone());
        } else {
            local_path_cache.insert(0, root_path.clone());
            local_path_cache.insert(5, root_path.clone());
            log::warn!("无法确定根目录 FRN，使用 fallback (0 和 5)");
        }

        dir_records.sort_by_key(|(_, parent_ref, _, _)| *parent_ref);

        let mut changed = true;
        let mut iterations = 0;
        while changed && iterations < 100 {
            changed = false;
            iterations += 1;

            for (file_ref, parent_ref, file_name, _) in &dir_records {
                if local_path_cache.contains_key(file_ref) {
                    continue;
                }
                if let Some(parent_path) = local_path_cache.get(parent_ref) {
                    let full_path = parent_path.join(file_name);
                    local_path_cache.insert(*file_ref, full_path);
                    changed = true;
                }
            }
        }

        // 将本地构建的路径映射合并到全局缓存 (仅更新本卷条目)
        {
            let mut global_cache = self.path_cache.write();
            for (frn, path) in &local_path_cache {
                global_cache.insert(*frn, path.clone());
            }
        }

        for (file_ref, parent_ref, file_name, usn) in &dir_records {
            let parent_path = local_path_cache
                .get(parent_ref)
                .cloned()
                .unwrap_or(root_path.clone());
            let full_path = parent_path.join(file_name);

            changes.push(UsnRecord {
                file_reference_number: *file_ref,
                parent_file_reference: *parent_ref,
                file_name: file_name.clone(),
                full_path,
                file_size: 0,
                last_write_time: 0,
                is_directory: true,
                extension: None,
                reason: UsnChangeReason::Modified,
                usn: *usn,
            });
        }

        for (file_ref, parent_ref, file_name, reason, timestamp, usn) in &file_records {
            let parent_path = local_path_cache
                .get(parent_ref)
                .cloned()
                .unwrap_or(root_path.clone());
            let full_path = parent_path.join(file_name);

            let reason_enum = match reason {
                // USN_REASON_FILE_CREATE = 0x00000100
                r if r & 0x00000100 != 0 => UsnChangeReason::Created,
                // USN_REASON_FILE_DELETE = 0x00001000
                r if r & 0x00001000 != 0 => UsnChangeReason::Deleted,
                // USN_REASON_DATA_EXTEND = 0x00000002
                r if r & 0x00000002 != 0 => UsnChangeReason::Modified,
                // USN_REASON_RENAME_OLD_NAME = 0x00008000
                r if r & 0x00008000 != 0 => UsnChangeReason::RenamedOldName,
                // USN_REASON_RENAME_NEW_NAME = 0x00004000
                r if r & 0x00004000 != 0 => UsnChangeReason::RenamedNewName,
                _ => UsnChangeReason::Modified,
            };

            let ext = full_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase());

            changes.push(UsnRecord {
                file_reference_number: *file_ref,
                parent_file_reference: *parent_ref,
                file_name: file_name.clone(),
                full_path,
                file_size: 0,
                last_write_time: *timestamp as i64 / 10000000 - 11644473600,
                is_directory: false,
                extension: ext,
                reason: reason_enum,
                usn: *usn,
            });
        }

        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }

        log::debug!("USN变化读取完成，共 {} 条记录", changes.len());
        Ok(changes)
    }

    pub fn get_all_changes(&self) -> Result<Vec<UsnRecord>> {
        let mut changes = Vec::new();
        let mut last_usn = self.last_usn.write();
        // 拷贝一份本地的 volumes,避免与 self.volumes 同时持锁.
        let volumes_snapshot = self.volumes.lock().clone();

        for volume in &volumes_snapshot {
            let start_usn = last_usn.get(volume).copied().unwrap_or(0);
            if let Ok(volume_changes) = self.read_usn_changes(volume, start_usn) {
                // 关键: 使用 volume_changes.last() 而非 changes.last(),
                // 避免跨卷 USN 追踪错误 (多卷场景下 changes 包含其他卷的记录).
                if let Some(record) = volume_changes.last() {
                    last_usn.insert(volume.to_string(), record.usn);
                }
                changes.extend(volume_changes);
            }
        }

        Ok(changes)
    }

    /// Returns the list of NTFS volumes currently known.
    /// - **调用前请确认这一函数是在 spawn_blocking 或后台线程中执行**, 因为它会触发一次同步盘符枚举.
    /// - 若盘符已被枚举过(`once` 已就绪), 直接返回缓存.
    /// Returns the list of NTFS volumes currently known.
    /// - 第一次访问会触发一次同步盘符枚举（即便在 spawn_blocking 中也请评估成本）。
    /// - 已枚举过时，直接返回缓存 `.to_vec()`。
    pub fn get_volumes(&self) -> Vec<String> {
        self.ensure_enumerated();
        self.volumes.lock().clone()
    }

    pub fn get_usn_journal_state(&self, volume: &str) -> Option<UsnJournalState> {
        self.get_journal_state(volume)
    }

    pub fn create_usn_journal(&self, volume: &str) -> Result<bool> {
        use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE};
        use windows_sys::Win32::Storage::FileSystem::CreateFileW;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        };
        use windows_sys::Win32::System::IO::DeviceIoControl;

        let wide_path: Vec<u16> = volume.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                std::ptr::null_mut(),
            )
        };

        if handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            let last_error = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            if last_error == 5 {
                return Err(AppError::PermissionDenied);
            }
            return Err(AppError::Other(format!(
                "无法打开卷 {}，错误码: {}",
                volume, last_error
            )));
        }

        let mut journal_data = CREATE_USN_JOURNAL_DATA {
            MaximumSize: 64 * 1024 * 1024,
            AllocationDelta: 16 * 1024 * 1024,
        };

        let success = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_CREATE_USN_JOURNAL,
                &mut journal_data as *mut _ as *mut c_void,
                std::mem::size_of::<CREATE_USN_JOURNAL_DATA>() as u32,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }

        if success == 0 {
            let last_error = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            if last_error == 5 {
                return Err(AppError::PermissionDenied);
            }
            return Err(AppError::Other(format!(
                "无法创建 USN Journal，错误码: {}",
                last_error
            )));
        }

        log::info!("USN Journal 创建成功: {}", volume);
        Ok(true)
    }
}

#[repr(C)]
#[derive(Debug, Default)]
#[allow(non_snake_case)]
struct USN_RECORD_V2 {
    RecordLength: u32,
    MajorVersion: u8,
    MinorVersion: u8,
    _unused: u16,
    FileReferenceNumber: u64,
    ParentFileReferenceNumber: u64,
    Usn: i64,
    TimeStamp: i64,
    Reason: u32,
    SourceInfo: u32,
    SecurityId: u32,
    FileAttributes: u32,
    FileNameLength: u16,
    FileNameOffset: u16,
    FileName: [u16; 1],
}

#[repr(C)]
#[derive(Debug, Default)]
#[allow(non_snake_case)]
struct USN_JOURNAL_DATA_V2 {
    UsnJournalID: u64,
    FirstUsn: i64,
    NextUsn: i64,
    LowestValidUsn: i64,
    MaxUsn: i64,
    MaximumSize: u64,
    AllocationDelta: u64,
    MinSupportedMajorVersion: u32,
    MaxSupportedMajorVersion: u32,
}

#[repr(C)]
#[derive(Debug, Default)]
#[allow(non_snake_case)]
struct MFT_ENUM_DATA {
    StartFileReferenceNumber: u64,
    LowUsn: i64,
    HighUsn: i64,
}

#[repr(C)]
#[derive(Debug, Default)]
#[allow(non_snake_case)]
struct READ_USN_JOURNAL_DATA_V0 {
    StartUsn: i64,
    ReasonMask: u32,
    UsnJournalID: u64,
}

#[repr(C)]
#[derive(Debug, Default)]
#[allow(non_snake_case)]
struct CREATE_USN_JOURNAL_DATA {
    MaximumSize: u64,
    AllocationDelta: u64,
}
