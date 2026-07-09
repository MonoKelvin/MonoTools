//! NTFS USN Journal 和 MFT 枚举 - 实现类似 Everything 的快速文件索引
//!
//! 核心架构：
//! 1. 全量索引：使用 FSCTL_ENUM_USN_DATA 批量读取 MFT，这是最快的全盘枚举方式
//! 2. 增量更新：使用 FSCTL_READ_USN_JOURNAL 读取变更记录
//! 3. 路径重建：通过父文件引用号构建完整路径
//!
//! 参考：https://learn.microsoft.com/en-us/windows/win32/api/winioctl/

use crate::error::{AppError, Result};
use crate::models::FileResult;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::ffi::{c_void, OsString};
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::sync::Arc;

const FSCTL_QUERY_USN_JOURNAL: u32 = 0x900F4;
const FSCTL_ENUM_USN_DATA: u32 = 0x900B8;
const FSCTL_READ_USN_JOURNAL: u32 = 0x900F8;

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
}

#[derive(Debug, Clone)]
pub struct UsnJournalState {
    pub usn: u64,
    pub next_usn: u64,
    pub journal_id: u64,
    pub max_size: u64,
    pub allocation_delta: u64,
}

pub struct NtfsIndexer {
    volumes: Vec<String>,
    last_usn: RwLock<HashMap<String, u64>>,
    path_cache: Arc<RwLock<HashMap<u64, PathBuf>>>,
}

impl NtfsIndexer {
    pub fn new() -> Result<Self> {
        let volumes = Self::enumerate_ntfs_volumes()?;
        Ok(Self {
            volumes,
            last_usn: RwLock::new(HashMap::new()),
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn enumerate_ntfs_volumes() -> Result<Vec<String>> {
        use windows_sys::Win32::Storage::FileSystem::{
            GetLogicalDriveStringsW, GetVolumeInformationW,
        };

        let mut buf: [u16; 512] = [0; 512];
        let len = unsafe { GetLogicalDriveStringsW(buf.len() as u32, buf.as_mut_ptr()) };
        if len == 0 {
            return Err(AppError::Other("Failed to enumerate volumes".into()));
        }

        let mut volumes = Vec::new();
        let mut i = 0;
        while i < len as usize {
            let start = i;
            while i < len as usize && buf[i] != 0 {
                i += 1;
            }
            if i > start {
                let name = OsString::from_wide(&buf[start..i]);
                let drive_path = name.to_string_lossy().to_string();

                let mut fs_name: [u16; 256] = [0; 256];
                let wide_path: Vec<u16> = drive_path.encode_utf16().chain(std::iter::once(0)).collect();

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
                        let volume = format!("\\\\.\\{}:", drive_path.chars().next().unwrap_or('C'));
                        volumes.push(volume);
                    }
                }
            }
            i += 1;
        }

        Ok(volumes)
    }

    pub fn get_journal_state(&self, volume: &str) -> Option<UsnJournalState> {
        use windows_sys::Win32::Storage::FileSystem::{
            CreateFileW,
            FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
        };
        use windows_sys::Win32::System::IO::DeviceIoControl;

        let wide_path: Vec<u16> = volume.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                std::ptr::null_mut(),
            )
        };

        if handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
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
            journal_id: state.UsnJournalID,
            max_size: state.MaximumSize,
            allocation_delta: state.AllocationDelta,
        })
    }

    pub fn enumerate_volume_files(&self, volume: &str, mut callback: impl FnMut(UsnRecord)) -> Result<()> {
        use windows_sys::Win32::Storage::FileSystem::{
            CreateFileW,
            FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
        };
        use windows_sys::Win32::System::IO::DeviceIoControl;

        let wide_path: Vec<u16> = volume.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                std::ptr::null_mut(),
            )
        };

        if handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            return Err(AppError::Other(format!("Failed to open volume: {}", volume)));
        }

        let mut start_usn: u64 = 0;
        let mut buffer: Vec<u8> = vec![0; 65536];
        let mut bytes_returned: u32 = 0;

        let drive_letter = volume.chars().nth(4).unwrap_or('C');
        let root_path = PathBuf::from(format!("{}:\\", drive_letter));

        let mut path_cache = self.path_cache.write();
        path_cache.insert(0, root_path.clone());

        loop {
            let mut query = USN_ENUM_DATA_V2 {
                StartUsn: start_usn,
                ReasonMask: 0xFFFFFFFF,
                ReturnOnlyOnClose: 0,
                Timeout: 0,
                BytesToWaitFor: 0,
                UsnJournalID: 0,
                NextUsn: 0,
            };

            let success = unsafe {
                DeviceIoControl(
                    handle,
                    FSCTL_ENUM_USN_DATA,
                    &mut query as *mut _ as *mut c_void,
                    std::mem::size_of::<USN_ENUM_DATA_V2>() as u32,
                    buffer.as_mut_ptr() as *mut c_void,
                    buffer.len() as u32,
                    &mut bytes_returned,
                    std::ptr::null_mut(),
                )
            };

            if success == 0 || bytes_returned == 0 {
                break;
            }

            let info = unsafe {
                &*(buffer.as_ptr() as *const USN_ENUM_DATA_V2)
            };

            if info.NextUsn == start_usn {
                break;
            }

            start_usn = info.NextUsn;

            let mut offset = std::mem::size_of::<USN_ENUM_DATA_V2>();
            while offset < bytes_returned as usize {
                let record = unsafe {
                    &*(buffer.as_ptr().add(offset) as *const USN_RECORD_V2)
                };

                let name_len = (record.FileNameLength / 2) as usize;
                let name_slice = unsafe {
                    std::slice::from_raw_parts(record.FileName.as_ptr(), name_len)
                };
                let name = OsString::from_wide(name_slice);
                let file_name = name.to_string_lossy().trim().to_string();

                if file_name.starts_with('.') {
                    if record.NextRecordOffset == 0 {
                        break;
                    }
                    offset += record.NextRecordOffset as usize;
                    continue;
                }

                let parent_ref = record.ParentFileReferenceNumber;
                let parent_path = path_cache.get(&parent_ref).cloned().unwrap_or(root_path.clone());
                let full_path = parent_path.join(&file_name);

                let is_directory = record.FileAttributes & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY != 0;

                if is_directory {
                    path_cache.insert(record.FileReferenceNumber, full_path.clone());
                }

                let ext = if !is_directory {
                    full_path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase())
                } else {
                    None
                };

                let next_offset = record.NextRecordOffset;
                let usn_record = UsnRecord {
                    file_reference_number: record.FileReferenceNumber,
                    parent_file_reference: parent_ref,
                    file_name,
                    full_path,
                    file_size: 0,
                    last_write_time: record.TimeStamp as i64 / 10000000 - 11644473600,
                    is_directory,
                    extension: ext,
                    reason: UsnChangeReason::Created,
                };

                callback(usn_record);

                if next_offset == 0 {
                    break;
                }
                offset += next_offset as usize;
            }
        }

        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }

        Ok(())
    }

    pub fn read_usn_changes(&self, volume: &str, start_usn: u64) -> Result<Vec<UsnRecord>> {
        use windows_sys::Win32::Storage::FileSystem::{
            CreateFileW,
            FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
        };
        use windows_sys::Win32::System::IO::DeviceIoControl;

        let wide_path: Vec<u16> = volume.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                std::ptr::null_mut(),
            )
        };

        if handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            return Ok(Vec::new());
        }

        let journal_id = match self.get_journal_state(volume) {
            Some(s) => s.journal_id,
            None => {
                unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
                return Ok(Vec::new());
            }
        };

        let mut buffer: Vec<u8> = vec![0; 65536];
        let mut bytes_returned: u32 = 0;

        let drive_letter = volume.chars().nth(4).unwrap_or('C');
        let root_path = PathBuf::from(format!("{}:\\", drive_letter));

        let mut changes = Vec::new();
        let path_cache = self.path_cache.read();

        let mut query = READ_USN_JOURNAL_DATA_V2 {
            StartUsn: start_usn,
            ReasonMask: 0xFFFFFFFF,
            ReturnOnlyOnClose: 0,
            Timeout: 0,
            BytesToWaitFor: 0,
            UsnJournalID: journal_id,
        };

        let success = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_READ_USN_JOURNAL,
                &mut query as *mut _ as *mut c_void,
                std::mem::size_of::<READ_USN_JOURNAL_DATA_V2>() as u32,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        if success == 0 || bytes_returned == 0 {
            unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
            return Ok(Vec::new());
        }

        let info = unsafe {
            &*(buffer.as_ptr() as *const READ_USN_JOURNAL_DATA_V2)
        };

        let mut offset = std::mem::size_of::<READ_USN_JOURNAL_DATA_V2>();
        while offset < bytes_returned as usize {
            let record = unsafe {
                &*(buffer.as_ptr().add(offset) as *const USN_RECORD_V2)
            };

            let name_len = (record.FileNameLength / 2) as usize;
            let name_slice = unsafe {
                std::slice::from_raw_parts(record.FileName.as_ptr(), name_len)
            };
            let name = OsString::from_wide(name_slice);
            let file_name = name.to_string_lossy().trim().to_string();

            if file_name.starts_with('.') {
                if record.NextRecordOffset == 0 {
                    break;
                }
                offset += record.NextRecordOffset as usize;
                continue;
            }

            let parent_ref = record.ParentFileReferenceNumber;
            let parent_path = path_cache.get(&parent_ref).cloned().unwrap_or(root_path.clone());
            let full_path = parent_path.join(&file_name);

            let reason = match record.Reason {
                r if r & 0x00000100 != 0 => UsnChangeReason::Created,
                r if r & 0x00000200 != 0 => UsnChangeReason::Deleted,
                r if r & 0x00000002 != 0 => UsnChangeReason::Modified,
                r if r & 0x00004000 != 0 => UsnChangeReason::RenamedOldName,
                r if r & 0x00008000 != 0 => UsnChangeReason::RenamedNewName,
                _ => UsnChangeReason::Modified,
            };

            let is_directory = record.FileAttributes & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY != 0;
            let ext = if !is_directory {
                full_path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase())
            } else {
                None
            };

            changes.push(UsnRecord {
                file_reference_number: record.FileReferenceNumber,
                parent_file_reference: parent_ref,
                file_name,
                full_path,
                file_size: 0,
                last_write_time: record.TimeStamp as i64 / 10000000 - 11644473600,
                is_directory,
                extension: ext,
                reason,
            });

            if record.NextRecordOffset == 0 {
                break;
            }
            offset += record.NextRecordOffset as usize;
        }

        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }

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
                    *last_usn.entry(volume.clone()).or_insert(0) = record.file_reference_number;
                }
            }
        }

        Ok(changes)
    }

    pub fn get_volumes(&self) -> &[String] {
        &self.volumes
    }
}

#[repr(C)]
#[derive(Debug, Default)]
struct USN_RECORD_V2 {
    RecordLength: u32,
    MajorVersion: u8,
    MinorVersion: u8,
    FileReferenceNumber: u64,
    ParentFileReferenceNumber: u64,
    Usn: i64,
    TimeStamp: i64,
    Reason: u32,
    SourceInfo: u32,
    SecurityId: u32,
    FileAttributes: u32,
    FileNameLength: u32,
    FileNameOffset: u32,
    NextRecordOffset: u32,
    FileName: [u16; 1],
}

#[repr(C)]
#[derive(Debug, Default)]
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
struct USN_ENUM_DATA_V2 {
    StartUsn: u64,
    ReasonMask: u32,
    ReturnOnlyOnClose: u32,
    Timeout: u32,
    BytesToWaitFor: u32,
    UsnJournalID: u64,
    NextUsn: u64,
}

#[repr(C)]
#[derive(Debug, Default)]
struct READ_USN_JOURNAL_DATA_V2 {
    StartUsn: u64,
    ReasonMask: u32,
    ReturnOnlyOnClose: u32,
    Timeout: u32,
    BytesToWaitFor: u32,
    UsnJournalID: u64,
}

pub trait FileEngine: Send + Sync {
    fn build_index(&self) -> Result<()>;
    fn update_index(&self) -> Result<()>;
    fn search(&self, query: &str, limit: u32) -> Vec<FileResult>;
    fn total(&self) -> usize;
}

pub struct WinUsnJournal {
    volumes: Vec<String>,
    last_usn: RwLock<HashMap<String, u64>>,
}

impl WinUsnJournal {
    pub fn new() -> Result<Self> {
        let volumes = Self::enumerate_volumes()?;
        Ok(Self {
            volumes,
            last_usn: RwLock::new(HashMap::new()),
        })
    }

    fn enumerate_volumes() -> Result<Vec<String>> {
        use windows_sys::Win32::Storage::FileSystem::GetLogicalDriveStringsW;

        let mut buf: [u16; 512] = [0; 512];
        let len = unsafe { GetLogicalDriveStringsW(buf.len() as u32, buf.as_mut_ptr()) };
        if len == 0 {
            return Err(AppError::Other("Failed to enumerate volumes".into()));
        }

        let mut volumes = Vec::new();
        let mut i = 0;
        while i < len as usize {
            let start = i;
            while i < len as usize && buf[i] != 0 {
                i += 1;
            }
            if i > start {
                let name = OsString::from_wide(&buf[start..i]);
                let s = name.to_string_lossy().to_string();
                if s.starts_with("C:\\") || s.starts_with("D:\\") || s.starts_with("E:\\") {
                    volumes.push(format!("\\\\.\\{}:", s.chars().next().unwrap()));
                }
            }
            i += 1;
        }

        Ok(volumes)
    }

    pub fn get_changes(&self) -> Result<Vec<UsnRecord>> {
        let mut changes = Vec::new();
        let mut last_usn = self.last_usn.write();

        for volume in &self.volumes {
            if let Ok(volume_changes) = self.get_volume_changes(volume, last_usn.get(volume).copied().unwrap_or(0)) {
                changes.extend(volume_changes);
                if let Some(record) = changes.last() {
                    *last_usn.entry(volume.clone()).or_insert(0) = record.file_reference_number;
                }
            }
        }

        Ok(changes)
    }

    fn get_volume_changes(&self, volume: &str, _starting_usn: u64) -> Result<Vec<UsnRecord>> {
        use windows_sys::Win32::Storage::FileSystem::{
            CreateFileW, ReadDirectoryChangesW,
            FILE_LIST_DIRECTORY, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
        };

        let wide_path: Vec<u16> = volume.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                FILE_LIST_DIRECTORY,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                std::ptr::null_mut(),
            )
        };

        if handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            return Ok(Vec::new());
        }

        let mut buffer: [u8; 65536] = [0; 65536];
        let mut bytes_returned: u32 = 0;

        let success = unsafe {
            ReadDirectoryChangesW(
                handle,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
                1,
                0x00000001 | 0x00000002 | 0x00000004 | 0x00000008 | 0x00000010,
                &mut bytes_returned,
                std::ptr::null_mut(),
                None,
            )
        };

        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }

        if success == 0 || bytes_returned == 0 {
            return Ok(Vec::new());
        }

        self.parse_changes(&buffer[..bytes_returned as usize], volume)
    }

    fn parse_changes(&self, buffer: &[u8], volume: &str) -> Result<Vec<UsnRecord>> {
        use windows_sys::Win32::Storage::FileSystem::FILE_NOTIFY_INFORMATION;

        let mut changes = Vec::new();
        let mut offset: usize = 0;

        while offset < buffer.len() {
            let info = unsafe {
                &*(buffer.as_ptr().add(offset) as *const FILE_NOTIFY_INFORMATION)
            };

            let name_len = (info.FileNameLength / 2) as usize;
            let name_slice = unsafe {
                std::slice::from_raw_parts(info.FileName.as_ptr(), name_len)
            };
            let name = OsString::from_wide(name_slice);
            let file_name = name.to_string_lossy().to_string();

            if file_name.starts_with('.') {
                if info.NextEntryOffset == 0 {
                    break;
                }
                offset += info.NextEntryOffset as usize;
                continue;
            }

            let drive_letter = volume.chars().nth(4).unwrap_or('C');
            let full_path = PathBuf::from(format!("{}:\\{}", drive_letter, file_name));

            let reason = match info.Action {
                0x00000001 => UsnChangeReason::Created,
                0x00000002 => UsnChangeReason::Deleted,
                0x00000003 => UsnChangeReason::Modified,
                0x00000004 => UsnChangeReason::RenamedOldName,
                0x00000005 => UsnChangeReason::RenamedNewName,
                _ => UsnChangeReason::Modified,
            };

            let record = UsnRecord {
                file_reference_number: offset as u64,
                parent_file_reference: 0,
                file_name,
                full_path,
                file_size: 0,
                last_write_time: chrono::Utc::now().timestamp(),
                is_directory: info.Action == 0x00000001 || info.Action == 0x00000002,
                extension: None,
                reason,
            };

            changes.push(record);

            if info.NextEntryOffset == 0 {
                break;
            }
            offset += info.NextEntryOffset as usize;
        }

        Ok(changes)
    }
}

pub struct FallbackFileEngine {
    index: RwLock<HashMap<String, Vec<UsnRecord>>>,
    roots: Vec<PathBuf>,
    last_update: RwLock<i64>,
}

impl FallbackFileEngine {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self {
            index: RwLock::new(HashMap::new()),
            roots,
            last_update: RwLock::new(0),
        }
    }

    fn record_filename(rec: &UsnRecord) -> String {
        rec.file_name.to_lowercase()
    }

    fn build_record(path: PathBuf) -> UsnRecord {
        let metadata = std::fs::metadata(&path).ok();
        let (size, is_dir) = metadata
            .as_ref()
            .map(|m| (m.len(), m.is_dir()))
            .unwrap_or((0, false));
        let modified = metadata
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        UsnRecord {
            file_reference_number: 0,
            parent_file_reference: 0,
            file_name: name,
            full_path: path,
            file_size: size,
            last_write_time: modified,
            is_directory: is_dir,
            extension: ext,
            reason: UsnChangeReason::Created,
        }
    }
}

impl FileEngine for FallbackFileEngine {
    fn build_index(&self) -> Result<()> {
        log::info!("开始构建文件索引(Fallback) - roots: {:?}", self.roots);
        let mut idx = self.index.write();
        idx.clear();

        let now = chrono::Utc::now().timestamp();
        for root in &self.roots {
            if !root.exists() {
                continue;
            }
            let walker = walkdir::WalkDir::new(root)
                .max_depth(8)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| !is_hidden(e.file_name()));
            for entry in walker.flatten() {
                if entry.depth() == 0 {
                    continue;
                }
                let rec = Self::build_record(entry.path().to_path_buf());
                if rec.file_name.is_empty() {
                    continue;
                }
                let key = Self::record_filename(&rec);
                idx.entry(key).or_insert_with(Vec::new).push(rec);
            }
        }
        *self.last_update.write() = now;
        log::info!("索引构建完成: {} 个文件名分组", idx.len());
        Ok(())
    }

    fn update_index(&self) -> Result<()> {
        let last = *self.last_update.read();
        let now = chrono::Utc::now().timestamp();
        if now - last < 60 {
            return Ok(());
        }
        self.build_index()
    }

    fn search(&self, query: &str, limit: u32) -> Vec<FileResult> {
        if query.is_empty() {
            return vec![];
        }
        let q = query.to_lowercase();
        let idx = self.index.read();
        let mut results = Vec::new();
        'outer: for (name, records) in idx.iter() {
            if !name.contains(&q) {
                continue;
            }
            for r in records.iter() {
                results.push(FileResult {
                    path: r.full_path.clone(),
                    name: r.file_name.clone(),
                    extension: r.extension.clone(),
                    size: r.file_size as i64,
                    modified_at: r.last_write_time,
                    is_directory: r.is_directory,
                });
                if results.len() >= limit as usize {
                    break 'outer;
                }
            }
        }
        results
    }

    fn total(&self) -> usize {
        self.index.read().values().map(|v| v.len()).sum()
    }
}

fn is_hidden(name: &std::ffi::OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}
