use anyhow::Result;

pub struct UsnJournal {
    volume: String,
    // TODO: 添加句柄等字段
}

impl UsnJournal {
    pub fn new(volume: String) -> Result<Self> {
        Ok(Self { volume })
    }

    /// 查询 USN Journal 状态
    pub fn query(&self) -> Result<JournalData> {
        // TODO: 实现 DeviceIoControl FSCTL_QUERY_USN_JOURNAL
        Err(anyhow::anyhow!("Not implemented"))
    }

    /// 读取 USN 记录
    pub fn read_records(&self, start_usn: u64) -> Result<Vec<UsnRecord>> {
        // TODO: 实现 DeviceIoControl FSCTL_READ_USN_JOURNAL
        Ok(vec![])
    }

    /// 创建 USN Journal
    pub fn create(&self) -> Result<()> {
        // TODO: 实现 DeviceIoControl FSCTL_CREATE_USN_JOURNAL
        Ok(())
    }
}

#[derive(Debug)]
pub struct JournalData {
    pub id: u64,
    pub first_usn: u64,
    pub next_usn: u64,
    pub oldest_usn: u64,
}

#[derive(Debug)]
pub struct UsnRecord {
    pub record_length: u32,
    pub major_version: u16,
    pub minor_version: u16,
    pub file_reference_number: u64,
    pub parent_file_reference_number: u64,
    pub usn: u64,
    pub timestamp: i64,
    pub reason: u32,
    pub source_info: u32,
    pub security_id: u32,
    pub file_attributes: u32,
    pub file_name_length: u16,
    pub file_name_offset: u16,
    pub file_name: String,
}
