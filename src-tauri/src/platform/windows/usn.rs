//! NTFS USN Journal 和 MFT 枚举 - 实现类似 Everything 的快速文件索引
//!
//! 核心架构：
//! 1. 全量索引：使用 FSCTL_ENUM_USN_DATA 批量读取 MFT，这是最快的全盘枚举方式
//! 2. 增量更新：使用 FSCTL_READ_USN_JOURNAL 读取变更记录
//! 3. 路径重建：通过父文件引用号构建完整路径（两遍扫描：先目录后文件）
//!
//! 参考：https://learn.microsoft.com/en-us/windows/win32/api/winioctl/

use crate::error::{AppError, Result};
use parking_lot::RwLock;
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

pub struct NtfsIndexer {
    volumes: Vec<String>,
    last_usn: RwLock<HashMap<String, u64>>,
    path_cache: Arc<RwLock<HashMap<u64, PathBuf>>>,
}

impl NtfsIndexer {
    pub fn new() -> Result<Self> {
        let volumes = Self::enumerate_ntfs_volumes(None)?;

        log::info!(
            "NTFS索引器已创建，检测到 {} 个NTFS卷。注意：完整的MFT索引需要管理员权限。",
            volumes.len()
        );

        Ok(Self {
            volumes,
            last_usn: RwLock::new(HashMap::new()),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn new_with_drives(drives: Vec<char>) -> Result<Self> {
        let volumes = Self::enumerate_ntfs_volumes(Some(drives))?;

        log::info!(
            "NTFS索引器已创建（指定盘符），检测到 {} 个NTFS卷: {:?}",
            volumes.len(),
            volumes
        );

        Ok(Self {
            volumes,
            last_usn: RwLock::new(HashMap::new()),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn enumerate_ntfs_volumes(drives: Option<Vec<char>>) -> Result<Vec<String>> {
        use windows_sys::Win32::Storage::FileSystem::{
            FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, GetLogicalDriveStringsW,
            GetVolumeInformationW,
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

            if volumes.is_empty() {
                let mut buf: [u16; 512] = [0; 512];
                let len = unsafe { GetLogicalDriveStringsW(buf.len() as u32, buf.as_mut_ptr()) };
                if len > 0 {
                    let mut i = 0;
                    while i < len as usize {
                        let start = i;
                        while i < len as usize && buf[i] != 0 {
                            i += 1;
                        }
                        if i > start {
                            let name = OsString::from_wide(&buf[start..i]);
                            let s = name.to_string_lossy().to_string();
                            if s.starts_with("C:\\")
                                || s.starts_with("D:\\")
                                || s.starts_with("E:\\")
                                || s.starts_with("F:\\")
                                || s.starts_with("G:\\")
                                || s.starts_with("H:\\")
                            {
                                let drive_letter = s.chars().next().unwrap();
                                let mut fs_name: [u16; 256] = [0; 256];
                                let wide_path: Vec<u16> =
                                    s.encode_utf16().chain(std::iter::once(0)).collect();

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
                                    if fs.eq_ignore_ascii_case("NTFS") {
                                        let volume = format!("\\\\.\\{}:", drive_letter);
                                        log::info!("检测到NTFS卷: {}", volume);
                                        volumes.push(volume);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        log::info!(
            "NTFS卷检测完成，共发现 {} 个卷: {:?}",
            volumes.len(),
            volumes
        );
        Ok(volumes)
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
            usn: state.UsnJournalID,
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
                    last_write_time: *timestamp as i64 / 10000000 - 11644473600,
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
                    last_write_time: *timestamp as i64 / 10000000 - 11644473600,
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

        let file_count = if total_count >= dirs_cached as u64 {
            total_count - dirs_cached as u64
        } else {
            0
        };
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
            return Ok(Vec::new());
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
            println!(
                "DEBUG: FSCTL_QUERY_USN_JOURNAL 失败，错误码: {}",
                last_error
            );
            unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
            return Ok(Vec::new());
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
            unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
            return Ok(Vec::new());
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

        let mut path_cache = self.path_cache.write();
        path_cache.clear();

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
            path_cache.insert(frn, root_path.clone());
        } else {
            path_cache.insert(0, root_path.clone());
            path_cache.insert(5, root_path.clone());
            log::warn!("无法确定根目录 FRN，使用 fallback (0 和 5)");
        }

        dir_records.sort_by_key(|(_, parent_ref, _, _)| *parent_ref);

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
        drop(path_cache);

        let path_cache = self.path_cache.read();

        for (file_ref, parent_ref, file_name, usn) in &dir_records {
            let parent_path = path_cache
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
            let parent_path = path_cache
                .get(parent_ref)
                .cloned()
                .unwrap_or(root_path.clone());
            let full_path = parent_path.join(file_name);

            let reason_enum = match reason {
                r if r & 0x00000100 != 0 => UsnChangeReason::Created,
                r if r & 0x00000200 != 0 => UsnChangeReason::Deleted,
                r if r & 0x00000002 != 0 => UsnChangeReason::Modified,
                r if r & 0x00004000 != 0 => UsnChangeReason::RenamedOldName,
                r if r & 0x00008000 != 0 => UsnChangeReason::RenamedNewName,
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

        for volume in &self.volumes {
            let start_usn = last_usn.get(volume).copied().unwrap_or(0);
            if let Ok(volume_changes) = self.read_usn_changes(volume, start_usn) {
                changes.extend(volume_changes);
                if let Some(record) = changes.last() {
                    *last_usn.entry(volume.clone()).or_insert(0) = record.usn;
                }
            }
        }

        Ok(changes)
    }

    pub fn get_volumes(&self) -> &[String] {
        &self.volumes
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
