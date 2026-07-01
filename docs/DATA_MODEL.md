# MonoTools 数据模型与存储设计

> 版本: v1.0  
> 日期: 2026-07-01

---

## 1. 存储架构

| 数据类型 | 存储方式 | 位置 | 说明 |
|---------|---------|------|------|
| 用户配置 | JSON | `config/settings.json` | 结构化配置，热重载 |
| 插件配置 | JSON | `config/plugins.json` | 各插件独立配置命名空间 |
| 文件索引 | SQLite | `data/index.db` | USN 索引，分表存储 |
| 应用索引 | SQLite | `data/index.db` (apps 表) | 已安装应用信息 |
| 工作区快照 | JSON | `data/workspaces/{id}.json` | 独立文件，便于导入导出 |
| 搜索历史 | SQLite | `data/history.db` | 查询记录，用于排序 |
| 插件状态 | JSON | `config/plugin-state.json` | 启用/禁用、加载顺序 |
| 日志 | 文本 | `logs/` | 按日期轮转 |

---

## 2. SQLite 数据库设计

### 2.1 文件索引数据库 (`data/index.db`)

```sql
-- ============================================
-- 文件索引表 (按 ASCII 和分表 0~40)
-- ============================================

-- list0 ~ list40
CREATE TABLE list0 (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL COLLATE NOCASE,
    path TEXT NOT NULL,
    parent_path TEXT NOT NULL,
    ext TEXT,                       -- 文件扩展名
    size INTEGER DEFAULT 0,
    created_at INTEGER DEFAULT 0,   -- Unix 时间戳
    modified_at INTEGER DEFAULT 0,
    accessed_at INTEGER DEFAULT 0,
    is_dir INTEGER DEFAULT 0,
    ascii_sum INTEGER DEFAULT 0,
    pinyin TEXT,                    -- 中文拼音索引
    priority INTEGER DEFAULT 0,     -- 手动设置的优先级
    volume TEXT                     -- 所在卷标
);

-- 为每个 listN 创建索引
CREATE INDEX idx_list0_name ON list0(name);
CREATE INDEX idx_list0_pinyin ON list0(pinyin);
CREATE INDEX idx_list0_parent ON list0(parent_path);
CREATE INDEX idx_list0_modified ON list0(modified_at);

-- ============================================
-- 应用索引表
-- ============================================
CREATE TABLE apps (
    id TEXT PRIMARY KEY,            -- 唯一ID (哈希或UUID)
    name TEXT NOT NULL COLLATE NOCASE,
    exe_path TEXT NOT NULL,
    icon_path TEXT,
    description TEXT,
    keywords TEXT,                  -- JSON 数组序列化
    pinyin TEXT,
    launch_count INTEGER DEFAULT 0,
    last_accessed INTEGER,
    source TEXT,                    -- start_menu / registry / desktop
    created_at INTEGER DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX idx_apps_name ON apps(name);
CREATE INDEX idx_apps_pinyin ON apps(pinyin);

-- ============================================
-- 搜索历史表
-- ============================================
CREATE TABLE search_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    query TEXT NOT NULL,
    result_type TEXT,               -- file / app / command / workspace
    result_id TEXT,
    selected_at INTEGER DEFAULT (strftime('%s', 'now')),
    execution_time_ms INTEGER
);

CREATE INDEX idx_history_query ON search_history(query);
CREATE INDEX idx_history_time ON search_history(selected_at);

-- ============================================
-- 元数据表
-- ============================================
CREATE TABLE meta (
    key TEXT PRIMARY KEY,
    value TEXT,
    updated_at INTEGER DEFAULT (strftime('%s', 'now'))
);

-- 存储索引版本、最后更新时间等
INSERT INTO meta (key, value) VALUES 
    ('index_version', '1'),
    ('last_full_index', '0'),
    ('total_indexed_files', '0');
```

### 2.2 数据库连接配置

```rust
use rusqlite::Connection;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;

pub struct Database {
    pool: Pool<SqliteConnectionManager>,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let manager = SqliteConnectionManager::file(db_path);
        let pool = Pool::builder()
            .max_size(10)
            .build(manager)?;
        
        let conn = pool.get()?;
        Self::optimize(&conn)?;
        
        Ok(Self { pool })
    }
    
    fn optimize(conn: &Connection) -> Result<()> {
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -262144;
            PRAGMA page_size = 65536;
            PRAGMA mmap_size = 268435456;
            PRAGMA temp_store = MEMORY;
            PRAGMA optimize;
        ")?;
        Ok(())
    }
    
    pub fn get_conn(&self) -> Result<PooledConnection<SqliteConnectionManager>> {
        Ok(self.pool.get()?)
    }
}
```

---

## 3. 配置数据模型

### 3.1 主配置 (`settings.json`)

```json
{
  "version": "1.0.0",
  "general": {
    "language": "zh-CN",
    "startup": true,
    "silentStart": true,
    "checkUpdates": true,
    "dataDirectory": "%LOCALAPPDATA%/MonoTools"
  },
  "search": {
    "windowWidth": 800,
    "windowHeight": 520,
    "position": "center",
    "blurOnLostFocus": true,
    "debounceMs": 80,
    "maxResults": 50,
    "showRecentOnEmpty": true,
    "rememberLastQuery": false
  },
  "hotkeys": {
    "search.toggle": {
      "key": "Space",
      "modifiers": ["Alt"],
      "enabled": true
    },
    "search.files": {
      "key": "F",
      "modifiers": ["Alt", "Shift"],
      "enabled": true
    },
    "workspace.quickSave": {
      "key": "S",
      "modifiers": ["Alt", "Shift"],
      "enabled": false
    }
  },
  "searchProviders": {
    "order": [
      "builtin:app-launcher",
      "builtin:file-search",
      "builtin:workspace-manager",
      "builtin:command-palette"
    ],
    "disabled": []
  },
  "fileSearch": {
    "indexedVolumes": ["C:", "D:"],
    "excludePaths": [
      "C:/Windows",
      "C:/Program Files/WindowsApps",
      "*/node_modules",
      "*/.git"
    ],
    "maxIndexSizeMB": 1024,
    "indexOnStartup": true,
    "realTimeUpdate": true
  },
  "workspace": {
    "autoSaveInterval": 0,
    "confirmBeforeRestore": true,
    "restoreDelayMs": 500
  },
  "theme": {
    "active": "builtin:default-theme",
    "mode": "dark",
    "accentColor": "#5e6ad2",
    "fontScale": 1.0
  },
  "plugin": {
    "autoUpdate": false,
    "developerMode": false,
    "trustedPublishers": []
  }
}
```

### 3.2 插件配置 (`plugins.json`)

```json
{
  "version": "1.0.0",
  "plugins": {
    "builtin:default-theme": {
      "enabled": true,
      "builtin": true,
      "config": {
        "mode": "dark",
        "accentColor": "#5e6ad2"
      }
    },
    "builtin:app-launcher": {
      "enabled": true,
      "builtin": true,
      "config": {}
    },
    "builtin:file-search": {
      "enabled": true,
      "builtin": true,
      "config": {
        "excludeHidden": true
      }
    },
    "com.example.calculator": {
      "enabled": true,
      "builtin": false,
      "installedAt": "2026-07-01T12:00:00Z",
      "config": {
        "precision": 2
      }
    }
  },
  "loadOrder": [
    "builtin:default-theme",
    "builtin:app-launcher",
    "builtin:file-search",
    "com.example.calculator"
  ]
}
```

---

## 4. 工作区快照模型

### 4.1 JSON Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["id", "name", "created_at", "apps"],
  "properties": {
    "id": { "type": "string", "format": "uuid" },
    "name": { "type": "string", "maxLength": 100 },
    "description": { "type": "string", "maxLength": 500 },
    "icon": { "type": "string" },
    "color": { "type": "string", "pattern": "^#[0-9a-fA-F]{6}$" },
    "created_at": { "type": "string", "format": "date-time" },
    "updated_at": { "type": "string", "format": "date-time" },
    "last_restored_at": { "type": "string", "format": "date-time" },
    "auto_start": { "type": "boolean", "default": false },
    "apps": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["id", "exe_path", "window_title", "window_rect"],
        "properties": {
          "id": { "type": "string" },
          "exe_path": { "type": "string" },
          "args": {
            "type": "array",
            "items": { "type": "string" }
          },
          "working_dir": { "type": "string" },
          "window_title": { "type": "string" },
          "window_rect": {
            "type": "object",
            "required": ["x", "y", "width", "height"],
            "properties": {
              "x": { "type": "integer" },
              "y": { "type": "integer" },
              "width": { "type": "integer", "minimum": 0 },
              "height": { "type": "integer", "minimum": 0 }
            }
          },
          "window_state": {
            "type": "string",
            "enum": ["Normal", "Minimized", "Maximized", "Fullscreen"]
          },
          "launch_order": { "type": "integer", "minimum": 0 },
          "launch_delay_ms": { "type": "integer", "minimum": 0 },
          "require_admin": { "type": "boolean", "default": false }
        }
      }
    }
  }
}
```

### 4.2 示例工作区文件

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "前端开发环境",
  "description": "VS Code + Chrome + 终端",
  "icon": "💻",
  "color": "#5e6ad2",
  "created_at": "2026-07-01T10:00:00Z",
  "updated_at": "2026-07-01T10:00:00Z",
  "last_restored_at": null,
  "auto_start": true,
  "apps": [
    {
      "id": "app-1",
      "exe_path": "C:/Users/xxx/AppData/Local/Programs/Microsoft VS Code/Code.exe",
      "args": ["C:/Projects/monotools"],
      "working_dir": "C:/Projects/monotools",
      "window_title": "monotools - Visual Studio Code",
      "window_rect": { "x": 0, "y": 0, "width": 1920, "height": 1080 },
      "window_state": "Maximized",
      "launch_order": 1,
      "launch_delay_ms": 0,
      "require_admin": false
    },
    {
      "id": "app-2",
      "exe_path": "C:/Program Files/Google/Chrome/Application/chrome.exe",
      "args": ["--profile-directory=Default"],
      "working_dir": null,
      "window_title": "Google Chrome",
      "window_rect": { "x": 1920, "y": 0, "width": 1920, "height": 1080 },
      "window_state": "Normal",
      "launch_order": 2,
      "launch_delay_ms": 1000,
      "require_admin": false
    }
  ]
}
```

---

## 5. 插件元数据模型

### 5.1 `plugin.json` Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["id", "name", "version", "type"],
  "properties": {
    "id": {
      "type": "string",
      "pattern": "^[a-z0-9-_.]+$",
      "description": "反向域名格式，如 com.example.plugin"
    },
    "name": { "type": "string", "maxLength": 100 },
    "version": { "type": "string", "pattern": "^\\d+\\.\\d+\\.\\d+.*$" },
    "description": { "type": "string", "maxLength": 500 },
    "author": { "type": "string" },
    "license": { "type": "string" },
    "type": {
      "type": "string",
      "enum": ["theme", "provider", "command", "view", "integration", "hybrid"]
    },
    "entry": {
      "type": "object",
      "properties": {
        "frontend": { "type": "string" },
        "backend": { "type": "string" }
      }
    },
    "permissions": {
      "type": "array",
      "items": {
        "type": "string",
        "pattern": "^[a-z]+:[a-z.]+$"
      }
    },
    "capabilities": {
      "type": "object",
      "properties": {
        "search": {
          "type": "object",
          "properties": {
            "enabled": { "type": "boolean" },
            "trigger": {
              "type": "array",
              "items": { "type": "string" }
            },
            "priority": { "type": "integer", "minimum": 0, "maximum": 1000 }
          }
        },
        "commands": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["id", "title"],
            "properties": {
              "id": { "type": "string" },
              "title": { "type": "string" },
              "description": { "type": "string" },
              "shortcut": { "type": "string" }
            }
          }
        }
      }
    },
    "hooks": {
      "type": "object",
      "properties": {
        "activate": { "type": "string" },
        "deactivate": { "type": "string" },
        "search": { "type": "string" },
        "execute": { "type": "string" },
        "configChange": { "type": "string" }
      }
    },
    "config": {
      "type": "object",
      "properties": {
        "schema": {
          "type": "object",
          "additionalProperties": {
            "type": "object",
            "properties": {
              "type": { "type": "string" },
              "default": {},
              "description": { "type": "string" }
            }
          }
        }
      }
    },
    "dependencies": {
      "type": "object",
      "properties": {
        "monotools": { "type": "string" },
        "plugins": {
          "type": "array",
          "items": { "type": "string" }
        }
      }
    },
    "resources": {
      "type": "object",
      "properties": {
        "memory_limit": { "type": "string" },
        "cpu_limit": { "type": "string" }
      }
    }
  }
}
```

---

## 6. 缓存设计

### 6.1 图标缓存

```rust
pub struct IconCache {
    dir: PathBuf,
    max_size: usize, // MB
}

impl IconCache {
    pub fn get(&self, exe_path: &str) -> Option<PathBuf> {
        let hash = blake3::hash(exe_path.as_bytes());
        let path = self.dir.join(format!("{}.png", hash));
        if path.exists() { Some(path) } else { None }
    }
    
    pub async fn extract(&self, exe_path: &str) -> Result<PathBuf> {
        // 从 EXE 提取图标，缓存为 PNG
        let hash = blake3::hash(exe_path.as_bytes());
        let out_path = self.dir.join(format!("{}.png", hash));
        
        // 使用 windows-rs 提取图标资源
        extract_exe_icon(exe_path, &out_path).await?;
        
        self.cleanup_if_needed().await?;
        Ok(out_path)
    }
}
```

### 6.2 搜索结果缓存

```rust
use lru::LruCache;
use std::sync::Mutex;

pub struct SearchCache {
    cache: Mutex<LruCache<String, Vec<SearchResult>>>,
}

impl SearchCache {
    pub fn get(&self, query: &str) -> Option<Vec<SearchResult>> {
        self.cache.lock().ok()?.get(query).cloned()
    }
    
    pub fn put(&self, query: String, results: Vec<SearchResult>) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.put(query, results);
        }
    }
}
```

---

## 7. 数据迁移策略

### 7.1 配置迁移

```rust
pub struct ConfigMigration;

impl ConfigMigration {
    pub fn migrate(current_version: &str, config: &mut Value) -> Result<()> {
        match current_version {
            "0.9.0" => Self::v090_to_v100(config)?,
            "1.0.0" => {} // 当前版本
            _ => warn!("Unknown config version: {}", current_version),
        }
        Ok(())
    }
    
    fn v090_to_v100(config: &mut Value) -> Result<()> {
        // 重命名字段
        if let Some(search) = config.get_mut("search") {
            if let Some(val) = search.get("windowSize").cloned() {
                search.as_object_mut().unwrap().remove("windowSize");
                search["windowWidth"] = val["width"].clone();
                search["windowHeight"] = val["height"].clone();
            }
        }
        Ok(())
    }
}
```

### 7.2 数据库迁移

使用 `rusqlite_migration` 管理数据库版本：

```rust
use rusqlite_migration::{Migrations, M};

const MIGRATIONS: Migrations = Migrations::new(vec![
    M::up("CREATE TABLE list0 (...)"),
    M::up("CREATE TABLE apps (...)"),
    M::up("ALTER TABLE apps ADD COLUMN pinyin TEXT"),
    // ...
]);
```

---

## 8. 备份与恢复

### 8.1 自动备份

```rust
pub struct BackupService;

impl BackupService {
    pub async fn auto_backup(&self) -> Result<()> {
        let backup_dir = get_data_dir().join("backups");
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = backup_dir.join(format!("backup_{}.zip", timestamp));
        
        // 打包 config/ 和 data/workspaces/
        create_zip_backup(&backup_path).await?;
        
        // 保留最近 10 个备份
        cleanup_old_backups(&backup_dir, 10).await?;
        
        Ok(())
    }
}
```

### 8.2 导出格式

用户可通过命令导出全部数据：

```bash
monotools config:export --output ./monotools-backup.zip
```

包含：
- `settings.json`
- `plugins.json`
- `workspaces/` 目录
- `plugin-data/` 目录