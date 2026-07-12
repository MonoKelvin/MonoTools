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
  Monitor, AppWindow, Package, Terminal, FileText, Folder,
  Music as MusicIcon, Image as ImageIcon, Video, Settings, Mail, Globe,
  MessageCircle, MessagesSquare, MessageSquare,
  Code, CodeXml, GitBranch,
  Cloud, CloudDownload,
  BookOpen, NotebookPen, FileCode, FileArchive,
  Calculator, Calendar, Clock,
  Gamepad2, Joystick, Sword,
  ShoppingCart, CreditCard, Wallet,
  Wrench,
  Shield, Lock,
  Database, Server, HardDrive,
  TerminalSquare,
  Sparkles, Bot, Brain, Wand2,
  Headphones, Music2, Disc3, Radio, Mic,
  Play, Film, Tv, Clapperboard, Cast,
  Camera, Brush, Palette, PenTool, Pen,
  FolderOpen, Newspaper, Bookmark, Map, MapPin, Activity, Boxes,
} from '@lucide/vue'
import type { IconState } from '@/composables/useAppIcon'

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
  { keywords: ['intellij', 'idea64', 'pycharm', 'webstorm', 'rider', 'clion',
    'goland', 'datagrip', 'phpstorm', 'rubymine', 'appcode', 'android studio'], icon: CodeXml },
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
  { keywords: ['photoshop', 'photoshopcc', 'ps64', 'ps ', ' gimp', 'paint.net',
    'affinity photo', 'lightroom', 'lr '], icon: Palette },
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
  { keywords: ['taobao', '淘宝', 'jd', '京东', 'pinduoduo', '拼多多',
    'shopify', 'amazon', 'aliexpress', '速卖通'], icon: ShoppingCart },
  { keywords: ['alipay', '支付宝', 'wechat pay', 'paypal', '财付通'], icon: CreditCard },

  // === 加密 / 货币 ===
  { keywords: ['metamask', 'phantom', 'trust wallet', 'coinbase', 'binance', 'okx', '火币', 'bitcoin', 'eth'], icon: Wallet },

  // === 数据库 / 服务器 ===
  { keywords: ['mysql', 'postgres', 'postgresql', 'redis', 'mongodb', 'mongo', 'navicat',
    'dbeaver', 'tableplus', 'sequel pro', 'pgadmin'], icon: Database },
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
// 通用兜底: 根据 resultType 给出最匹配的 Lucide 组件
// ============================================================================

/**
 * 通用兜底图标: 在没命中任何关键词规则时使用, 风格统一.
 * 接受 resultType 给出最佳匹配, 默认 AppWindow.
 */
export function fallbackIconForResultType(type: string): Component {
  switch (type) {
    case 'system-app':
      return Monitor
    case 'uwp-app':
      return Package
    case 'command':
      return Terminal
    case 'directory':
      return Folder
    case 'image':
      return ImageIcon
    case 'video':
      return Video
    case 'audio':
      return MusicIcon
    case 'document':
    case 'other-file':
    case 'executable':
    case 'archive':
      return FileText
    default:
      return AppWindow
  }
}

// ============================================================================
// LobeHub UI 彩色图标 (可选方案)
// ============================================================================

/**
 * LobeHub 提供了大量高质量的开源彩色品牌图标, 适合需要"识别度"的应用图标.
 * 本函数返回可下载的 URL, 由调用方 (useAppIcon) 决定是否使用.
 *
 * URL 格式参考: https://github.com/lobehub/lobe-icons
 *   - 单色:   `https://lobehub.com/icon/{slug}`
 *   - 彩色:   `https://lobehub.com/icon-colorful/{slug}`
 *
 * 使用方式 (示例):
 *   const url = lobehubIconUrl('wechat', 'colorful')
 *   // → 'https://lobehub.com/icon-colorful/wechat'
 *
 * 注意: 需要外网访问. 失败时调用方应回退到 Lucide 通用图标.
 */
export function lobehubIconUrl(slug: string, variant: 'mono' | 'colorful' = 'colorful'): string {
  const safe = slug.toLowerCase().replace(/[^a-z0-9-]/g, '-')
  return variant === 'colorful'
    ? `https://lobehub.com/icon-colorful/${safe}`
    : `https://lobehub.com/icon/${safe}`
}

// ============================================================================
// 兼容导出: 给 AppResultItem / ResultItem 使用的兜底 Lucide 组件
// ============================================================================

export {
  Monitor, AppWindow, Package, Terminal, FileText, Folder,
  MusicIcon, ImageIcon, Video, Settings, Mail, Globe,
}
