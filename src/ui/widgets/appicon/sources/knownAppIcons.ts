/**
 * 已知应用图标 (静态资源) —— 离线命中, 无需调用后端 IPC.
 *
 * 策略变更 (2026-07):
 *   - **不再使用手绘 SVG** (不符合 MonoTools 整体视觉风格, 维护成本高).
 *   - 改为关键词 → Lucide 组件的 1:1 映射. Lucide 矢量 / 单色 / `currentColor`,
 *     与 AppResultItem 默认图标 (AppWindow) 风格一致, 加载零延迟.
 *   - 用户也可选用 LobeHub UI 彩色图标 (cdn) — 通过 `lobehubIconUrl()` 工具函数
 *     拼出可下载的 URL, 由 `useAppIcon` 异步加载. 当前默认仍走 Lucide.
 *
 * 命中策略 (`lookupKnownIcon`): 名称或路径中含小写 token 即命中.
 *   例: 'WeChat.exe' 包含 'wechat' → 返回 MessageCircle 组件.
 */

import type { Component } from 'vue'
import {
    Monitor, AppWindow, Package, Terminal, FileText, Folder, FolderOpen,
    Music as MusicIcon, Image as ImageIcon, Video, Settings, Mail, Globe,
    MessageCircle, MessagesSquare, MessageSquare,
    Code, CodeXml, GitBranch,
    Cloud, CloudDownload,
    BookOpen, NotebookPen, FileCode, FileArchive, FileImage, FileVideo, FileMusic, FileSpreadsheet, FilePieChart,
    Calculator, Calendar, Clock,
    Gamepad2, Joystick, Sword,
    ShoppingCart, CreditCard, Wallet,
    Wrench,
    Shield, Lock, Key,
    Database, Server, HardDrive,
    TerminalSquare,
    Sparkles, Bot, Brain, Wand2,
    Headphones, Music2, Disc3, Radio, Mic,
    Play, Film, Tv, Clapperboard, Cast,
    Camera, Brush, Palette, PenTool, Pen,
    Newspaper, Bookmark, Map, MapPin, Activity, Boxes,
    FileType, FileQuestion, Hash, StickyNote, File, Files,
} from '@lucide/vue'
import type { IconState } from './types'

// ============================================================================
// 关键词 → Lucide 组件 映射表
// ============================================================================

type IconRule = { keywords: string[]; icon: Component }

const ICON_RULES: IconRule[] = [
    // === 通讯 / IM ===
    { keywords: ['wechat', 'weixin', 'wxwork', '企业微信'], icon: MessageCircle },
    { keywords: ['qq', 'tencent'], icon: MessageCircle },
    { keywords: ['dingtalk', 'dingding', '钉钉'], icon: MessageCircle },
    { keywords: ['feishu', 'lark', '飞书'], icon: MessageCircle },
    { keywords: ['slack'], icon: MessageSquare },
    { keywords: ['telegram'], icon: MessageCircle },
    { keywords: ['discord'], icon: MessagesSquare },
    { keywords: ['whatsapp'], icon: MessageCircle },
    { keywords: ['zoom'], icon: Video },
    { keywords: ['teams', 'microsoft teams'], icon: MessagesSquare },
    { keywords: ['skype'], icon: MessageCircle },
    { keywords: ['line'], icon: MessageCircle },

    // === 开发 (Code/IDE) ===
    { keywords: ['vscode', 'code -', 'visual studio code', 'vs code'], icon: Code },
    {
        keywords: ['intellij', 'idea64', 'pycharm', 'webstorm', 'rider', 'clion',
            'goland', 'datagrip', 'phpstorm', 'rubymine', 'appcode', 'android studio'], icon: CodeXml
    },
    { keywords: ['git', 'github desktop', 'sourcetree', 'tortoisegit', 'gitkraken'], icon: GitBranch },
    { keywords: ['docker', 'docker desktop', 'kubernetes', 'k8s'], icon: Boxes },
    { keywords: ['postman', 'insomnia', 'paw'], icon: Cloud },
    { keywords: ['cursor', 'windsurf', 'tabnine', 'codeium'], icon: Sparkles },
    { keywords: ['sublime', 'notepad++', 'vim', 'emacs', 'nano', 'ultraedit', 'editplus'], icon: FileCode },
    { keywords: ['terminal', 'warp', 'iterm', 'tabby', 'alacritty', 'kitty'], icon: TerminalSquare },
    { keywords: ['wsl', 'ubuntu on windows'], icon: TerminalSquare },

    // === 浏览器 ===
    { keywords: ['chrome', 'chromium', 'google chrome'], icon: Globe },
    { keywords: ['msedge', 'edge', 'microsoft edge'], icon: Globe },
    { keywords: ['firefox', 'mozilla'], icon: Globe },
    { keywords: ['brave'], icon: Shield },
    { keywords: ['safari'], icon: Globe },
    { keywords: ['opera', 'opera gx'], icon: Globe },
    { keywords: ['vivaldi'], icon: Globe },
    { keywords: ['arc'], icon: Globe },
    { keywords: ['tor browser'], icon: Lock },

    // === 办公 ===
    { keywords: ['winword', 'word', 'office word', 'wps word', 'pages'], icon: FileText },
    { keywords: ['excel', 'office excel', 'wps excel', 'numbers'], icon: FileText },
    { keywords: ['powerpnt', 'ppt', 'office powerpoint', 'wps ppt', 'keynote'], icon: FileText },
    { keywords: ['outlook', 'office outlook'], icon: Mail },
    { keywords: ['onenote'], icon: NotebookPen },
    { keywords: ['acrobat', 'pdf reader', 'foxit', 'sumatra'], icon: FileText },
    { keywords: ['visio'], icon: Boxes },

    // === 媒体 - 音乐 ===
    { keywords: ['spotify'], icon: Headphones },
    { keywords: ['网易云', 'cloudmusic', 'orion'], icon: Music2 },
    { keywords: ['qqmusic', 'qq音乐'], icon: Music2 },
    { keywords: ['foobar2000', 'aimp', 'itunes', 'musicbee'], icon: Disc3 },
    { keywords: ['荔枝', 'lizhi', 'ximalaya', '喜马拉雅', 'podcast'], icon: Mic },
    { keywords: ['audacity', 'soundforge', 'fl studio', 'ableton'], icon: Radio },

    // === 媒体 - 视频 ===
    { keywords: ['vlc', 'potplayer', 'mpv', 'kmplayer', 'gom'], icon: Play },
    { keywords: ['youtube', 'bilibili', '哔哩哔哩', 'youku', '优酷', 'iqiyi', '爱奇艺', 'tencent video', '腾讯视频'], icon: Tv },
    { keywords: ['netflix', 'disney+', 'hulu', 'hbomax', 'prime video'], icon: Film },
    { keywords: ['obs', 'obs studio', 'streamlabs', 'xsplit'], icon: Cast },
    { keywords: ['handbrake', '格式工厂', 'format factory'], icon: Clapperboard },
    { keywords: ['shotcut', 'davinci', 'premiere', 'pr cc', 'final cut'], icon: Film },

    // === 设计 / 创作 ===
    {
        keywords: ['photoshop', 'photoshopcc', 'ps64', 'ps ', ' gimp', 'paint.net',
            'affinity photo', 'lightroom', 'lr '], icon: Palette
    },
    { keywords: ['illustrator', 'ai64', ' ai ', 'inkscape', 'affinity designer'], icon: Pen },
    { keywords: ['figma', 'sketch', 'xd', 'axure'], icon: PenTool },
    { keywords: ['canva', '创客贴'], icon: Brush },
    { keywords: ['blender', 'maya', '3ds max', 'cinema 4d', 'c4d', 'zbrush', 'substance'], icon: Boxes },
    { keywords: ['unity', 'unreal', 'godot'], icon: Gamepad2 },
    { keywords: ['autocad', 'sketchup', 'rhino', 'solidworks'], icon: PenTool },

    // === 系统自带 ===
    { keywords: ['powershell', 'pwsh'], icon: TerminalSquare },
    { keywords: ['cmd', 'command prompt', '命令提示符'], icon: Terminal },
    { keywords: ['windows terminal'], icon: TerminalSquare },
    { keywords: ['explorer', 'file explorer', '此电脑', '我的电脑', 'finder'], icon: FolderOpen },
    { keywords: ['设置', 'settings', 'control panel', 'sysdm.cpl'], icon: Settings },
    { keywords: ['task manager', 'taskmgr', '任务管理器'], icon: Activity },
    { keywords: ['registry', 'regedit'], icon: Wrench },
    { keywords: ['notepad', '记事本', 'notepad.exe'], icon: FileText },
    { keywords: ['calculator', '计算器', 'calc'], icon: Calculator },
    { keywords: ['clock', '闹钟', '时间', '时间日期'], icon: Clock },
    { keywords: ['calendar', '日历'], icon: Calendar },
    { keywords: ['mspaint', '画图', '画图3d'], icon: Palette },
    { keywords: ['snippingtool', '截图工具', 'snip', 'screenshot'], icon: Camera },
    { keywords: ['defragment', '磁盘清理', '磁盘碎片'], icon: HardDrive },
    { keywords: ['task scheduler'], icon: Calendar },

    // === AI / 笔记 ===
    { keywords: ['chatgpt', 'chat gpt', 'openai'], icon: Bot },
    { keywords: ['claude', 'anthropic'], icon: Sparkles },
    { keywords: ['gemini', 'bard'], icon: Brain },
    { keywords: ['copilot', 'github copilot', 'cody'], icon: Wand2 },
    { keywords: ['midjourney'], icon: Wand2 },
    { keywords: ['notion'], icon: NotebookPen },
    { keywords: ['obsidian'], icon: BookOpen },
    { keywords: ['evernote'], icon: Bookmark },
    { keywords: ['typora', 'marktext', 'zettlr'], icon: FileCode },
    { keywords: ['roam'], icon: Newspaper },
    { keywords: ['logseq'], icon: Boxes },
    { keywords: ['anki'], icon: Boxes },
    { keywords: ['grammarly'], icon: FileText },

    // === 云盘 / 下载 ===
    { keywords: ['onedrive'], icon: Cloud },
    { keywords: ['dropbox'], icon: Cloud },
    { keywords: ['baidunetdisk', 'baidu', '百度网盘', '坚果云', 'jianguoyun'], icon: CloudDownload },
    { keywords: ['googledrive', 'google drive', 'drive'], icon: Cloud },
    { keywords: ['阿里云盘', 'aliyun drive', 'aliyunpan'], icon: Cloud },
    { keywords: ['迅雷', 'thunder', 'xunlei'], icon: CloudDownload },
    { keywords: ['motrix', 'aria2', 'idm', 'internet download manager'], icon: CloudDownload },
    { keywords: ['utorrent', 'bittorrent', 'qbittorrent', 'transmission'], icon: CloudDownload },

    // === 游戏 ===
    { keywords: ['steam'], icon: Gamepad2 },
    { keywords: ['epic', 'epicgames'], icon: Gamepad2 },
    { keywords: ['wegame'], icon: Gamepad2 },
    { keywords: ['origin', 'ea app'], icon: Joystick },
    { keywords: ['battle.net', 'battlenet'], icon: Sword },
    { keywords: ['minecraft', '我的世界', 'launcher'], icon: Gamepad2 },
    { keywords: ['playstation', 'ps4', 'ps5'], icon: Gamepad2 },

    // === 压缩 ===
    { keywords: ['7-zip', '7zip', 'winrar', 'winzip', 'bandizip', 'peazip', '好压', '快压', '360zip'], icon: FileArchive },

    // === 邮件 ===
    { keywords: ['mail', 'thunderbird', '网易邮箱', 'qq邮箱', 'foxmail'], icon: Mail },

    // === 地图 ===
    { keywords: ['baidu map', '百度地图', 'amap', '高德地图', 'google map', 'google earth'], icon: Map },
    { keywords: ['maps', 'mapview'], icon: MapPin },

    // === 购物 / 支付 ===
    {
        keywords: ['taobao', '淘宝', 'jd', '京东', 'pinduoduo', '拼多多',
            'shopify', 'amazon', 'aliexpress', '速卖通'], icon: ShoppingCart
    },
    { keywords: ['alipay', '支付宝', 'wechat pay', 'paypal', '财付通'], icon: CreditCard },

    // === 加密 / 货币 ===
    { keywords: ['metamask', 'phantom', 'trust wallet', 'coinbase', 'binance', 'okx', '火币', 'bitcoin', 'eth'], icon: Wallet },

    // === 数据库 / 服务器 ===
    {
        keywords: ['mysql', 'postgres', 'postgresql', 'redis', 'mongodb', 'mongo', 'navicat',
            'dbeaver', 'tableplus', 'sequel pro', 'pgadmin'], icon: Database
    },
    { keywords: ['xampp', 'wamp', 'mamp', 'phpstudy'], icon: Server },

    // === 系统工具 ===
    { keywords: ['ccleaner', 'cleaner', 'disk cleanup'], icon: Brush },
    { keywords: ['malwarebytes', 'defender', 'avg', 'avast', 'norton', 'mcafee', '卡巴斯基', 'kaspersky', '360安全', '360杀毒'], icon: Shield },
    { keywords: ['teamviewer', 'anydesk', 'rustdesk', 'parsec', '向日葵'], icon: Cast },
    { keywords: ['virtualbox', 'vmware', 'hyper-v', 'parallels'], icon: Boxes },
    { keywords: ['wireshark', 'fiddler', 'charles'], icon: Activity },
]

/**
 * 关键词命中查找. 返回 `IconState | null`:
 * - 命中关键词: 返回 `{ kind: 'component', value: LucideIcon }`, 直接渲染.
 * - 都不命中: 返回 `null`, 调用方决定回退到 IPC / 通用图标.
 *
 * 改成组件而非 data URL 的好处:
 * - **零延迟** (组件已在 bundle 内, 不需加载/解析).
 * - **currentColor**: 自动跟随主题色, 不需为深色/浅色各做一版.
 * - **矢量**: 1x / 1.5x / 2x DPI 都清晰.
 * - **风格统一**: 与 AppResultItem 默认 Lucide 图标一致.
 */
export function lookupKnownIcon(name: string, path?: string): IconState | null {
    const haystack = `${name} ${path ?? ''}`.toLowerCase()
    if (!haystack.trim()) return null
    for (const rule of ICON_RULES) {
        if (rule.keywords.some((k) => haystack.includes(k.toLowerCase()))) {
            return { kind: 'component', value: rule.icon }
        }
    }
    return null
}

// ============================================================================
// 按"文件类型/扩展名"给图标 —— 替代 monogram 的精确映射
// ============================================================================

/**
 * 文件扩展名 → Lucide 组件 映射表.
 * 优先于 resultType (因为 resultType 是后端归一化过的"大类", 扩展名能区分
 * 同一类里的子类型, 比如 document 里的 .docx / .pdf / .xlsx).
 *
 * 命名: 大类图标 (FileText) 留作 default, 命中具体扩展名时用子类型图标
 * (FileSpreadsheet / FileCode / FileArchive 等).
 */
const FILE_EXT_ICON: Array<[RegExp, Component]> = [
    // === 目录 ===
    [/^$/, Folder],

    // === 代码 / 配置文件 ===
    [/\.(ts|tsx|js|jsx|mjs|cjs|vue|svelte|astro|html?|css|scss|sass|less)$/i, FileCode],
    [/\.(py|pyw|pyx|pyi|rpy)$/i, FileCode],
    [/\.(rs|rlib|toml|cargo|lock)$/i, FileCode],
    [/\.(go|mod|sum)$/i, FileCode],
    [/\.(c|cc|cpp|cxx|h|hpp|hxx|hh|m|mm)$/i, FileCode],
    [/\.(java|kt|ktm|scala|groovy|gradle|kts)$/i, FileCode],
    [/\.(cs|fs|vb|sln|csproj|fsproj|vbproj)$/i, FileCode],
    [/\.(php|php5|phtml|inc)$/i, FileCode],
    [/\.(rb|erb|gemspec|ru)$/i, FileCode],
    [/\.(swift|m|h|mm)$/i, FileCode],
    [/\.(sh|bash|zsh|ksh|csh|fish|ps1|psm1|psd1|bat|cmd|nsi|ahk)$/i, FileCode],
    [/\.(json|json5|jsonc|yaml|yml|toml|ini|cfg|conf|config|env|properties|xml|plist)$/i, FileCode],
    [/\.(sql|dql|hql)$/i, Database],
    [/\.(lua|vim|vimrc|nix|dockerfile|containerfile|makefile|cmake|gradle)$/i, FileCode],
    [/\.(md|markdown|rst|adoc|tex|wiki)$/i, FileText],
    [/\.(log|txt)$/i, FileText],

    // === Office / 文档 ===
    [/\.pdf$/i, FileText],
    [/\.(doc|docx|odt|pages|rtf)$/i, FileText],
    [/\.(xls|xlsx|xlsm|xlsb|ods|numbers|csv|tsv)$/i, FileSpreadsheet],
    [/\.(ppt|pptx|pps|ppsx|odp|key)$/i, FilePieChart],
    [/\.(note|notebook|one|onepkg)$/i, NotebookPen],

    // === 媒体 - 图片 ===
    [/\.(jpe?g|png|gif|webp|bmp|tiff?|heic|heif|ico|avif|jxl|raw|cr2|nef|orf|sr2|dng|arw|rw2|raf|svg|svgz)$/i, FileImage],

    // === 媒体 - 视频 ===
    [/\.(mp4|m4v|mkv|webm|mov|avi|wmv|flv|f4v|mpe?g|mp2|vob|ogv|3gp|3g2|mts|m2ts|ts|rm|rmvb|asf|amv)$/i, FileVideo],

    // === 媒体 - 音频 ===
    [/\.(mp3|m4a|aac|flac|wav|ogg|oga|opus|wma|alac|ape|aiff?|mka|caf|mid|midi|amr|awb)$/i, FileMusic],

    // === 压缩包 ===
    [/\.(zip|rar|7z|tar|gz|tgz|bz2|tbz|xz|txz|lz|lzma|lz4|zst|Z|cab|msp|msu|rpm|iso|img|dmg|wim|esd|deb|pkg|rpm|apk|ipa|appx|msix|jar|war|ear|apk|aar)$/i, FileArchive],

    // === 字体 ===
    [/\.(ttf|otf|woff2?|eot|ttc)$/i, FileType],

    // === 可执行 / 安装包 ===
    [/\.(exe|msi|bat|cmd|com|scr|pif|gadget|dll|sys|drv|bin|run|app|dmg|pkg|deb|rpm|apk|ipa|appx|msix|jar|jse|wsf|vbs)$/i, AppWindow],

    // === 数据库 ===
    [/\.(db|db3|sqlite|sqlite3|mdb|accdb|dbf)$/i, Database],
]

/**
 * 应用结果类 → Lucide 组件 兜底.
 * 用于 `category === 'apps'` 的项, 区分系统/UWP/普通应用.
 */
function appFallbackIcon(resultType: string | undefined, title: string, path: string): Component {
    const t = `${title} ${path}`.toLowerCase()
    // 系统自带
    if (/powershell|pwsh/.test(t)) return TerminalSquare
    if (/^cmd$|command prompt|命令提示符/.test(t)) return Terminal
    if (/windows terminal/.test(t)) return TerminalSquare
    if (/explorer|file explorer|此电脑|我的电脑|finder/.test(t)) return FolderOpen
    if (/设置|settings|control panel|sysdm\.cpl/.test(t)) return Settings
    if (/task manager|taskmgr|任务管理器/.test(t)) return Activity
    if (/registry|regedit/.test(t)) return Wrench
    if (/notepad|记事本/.test(t)) return FileText
    if (/calculator|计算器|calc/.test(t)) return Calculator
    if (/clock|闹钟|时间/.test(t)) return Clock
    if (/calendar|日历/.test(t)) return Calendar
    if (/mspaint|画图/.test(t)) return Palette
    if (/snippingtool|截图工具|snip|screenshot/.test(t)) return Camera
    if (/defragment|磁盘清理|磁盘碎片/.test(t)) return HardDrive
    if (/task scheduler/.test(t)) return Calendar

    switch (resultType) {
        case 'system-app':
            return Monitor
        case 'uwp-app':
            return Package
        case 'command':
            return Terminal
        default:
            return AppWindow
    }
}

/**
 * 按 resultType + 扩展名 给出最合适的 Lucide 文件类型图标.
 *
 * 设计要点:
 * - 后端粗粒度 (document / image / video) → 精确扩展名 (xlsx / png / mp4)
 * - 目录: FolderOpen (打开) 而非 Folder (静态)
 * - 应用: 用 appFallbackIcon 区分 系统/UWP/普通
 *
 * 失败: 返回 File (通用占位, 不再返回 monogram 字母).
 */
export function iconForFileKind(result: { title?: string; subtitle?: string; category?: string; resultType?: string }): Component {
    const title = result.title || ''
    const subtitle = result.subtitle || ''
    const path = subtitle || title
    const resultType = result.resultType || ''

    // === 应用类: 系统/UWP/普通 exe 用 appFallbackIcon ===
    if (result.category === 'apps') {
        return appFallbackIcon(resultType, title, path)
    }

    // === 命令类 ===
    if (result.category === 'commands' || resultType === 'command') {
        return Terminal
    }

    // === 目录: 用 FolderOpen (语义: 可点击进入) ===
    if (resultType === 'directory') {
        return FolderOpen
    }

    // === 取扩展名 (从 path 最后一段) ===
    const lastSeg = path.split(/[\\/]/).pop() || ''
    const dot = lastSeg.lastIndexOf('.')
    // 没有扩展名 或扩展名为空 (例如 .gitignore) → 走 resultType 大类
    let ext = ''
    if (dot > 0 && dot < lastSeg.length - 1) {
        ext = lastSeg.slice(dot + 1).toLowerCase()
    }

    // === 按扩展名查表 ===
    if (ext) {
        for (const [pattern, comp] of FILE_EXT_ICON) {
            if (pattern.test('.' + ext) || (pattern.test(ext) && pattern.source.startsWith('\\.'))) {
                return comp
            }
        }
    }

    // === resultType 大类兜底 ===
    switch (resultType) {
        case 'image': return FileImage
        case 'video': return FileVideo
        case 'audio': return FileMusic
        case 'document': return FileText
        case 'archive': return FileArchive
        case 'executable': return AppWindow
        case 'code': return FileCode
        case 'pdf': return FileText
        case 'spreadsheet': return FileSpreadsheet
        case 'presentation': return FilePieChart
        case 'font': return FileType
        case 'design': return Palette
        case 'other-file': return File
        default:
            return File
    }
}


