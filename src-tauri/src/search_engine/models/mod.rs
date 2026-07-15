//! 搜索模块的数据模型

pub mod app_entry;
pub mod file_result;
pub mod search_result;

pub use app_entry::AppEntry;
pub use file_result::FileResult;
pub use search_result::{ResultType, SearchAction, SearchCategory, SearchOptions, SearchResult};
