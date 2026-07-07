// Re-export of common types for the lib root
pub use crate::models::app_entry::AppEntry;
pub use crate::models::custom_command::CustomCommand;
pub use crate::models::file_result::FileResult;
pub use crate::models::search_result::{SearchAction, SearchCategory, SearchOptions, SearchResult};
pub use crate::models::settings::{Settings, ThemeMode};
pub use crate::models::startup_item::{NewStartupItem, StartupItem, StartupSource};

// 兼容外层 "types.rs" 的命名
pub use crate::models::app_entry::AppEntry as _AppEntry;
pub use crate::models::startup_item::StartupItem as _StartupItem;
pub use crate::models::search_result::SearchResult as _SearchResult;
