/**
 * 排序配置 —— 集中管理所有排序相关的权重、映射和规则.
 *
 * 修改这里一处, 全工程排序行为同步生效.
 * 详见 CLAUDE.md "编码规范" 1.1 (不写魔法数).
 */

import type { Component } from 'vue'

// ============================================================================
// 排序模式
// ============================================================================

export type SortMode = 'smart' | 'name' | 'recent' | 'path'

export interface SortOption {
  key: SortMode
  label: string
  icon?: Component
}

export const SORT_OPTIONS: SortOption[] = [
  { key: 'smart', label: '智能排序' },
  { key: 'name', label: '名称' },
  { key: 'recent', label: '最近访问' },
  { key: 'path', label: '路径' },
]

// ============================================================================
// 智能排序权重
// ============================================================================

export interface SmartSortWeights {
  /** 访问次数权重: 每次访问贡献的分数 */
  launchCount: number
  /** 名称匹配权重: 搜索词命中名称时的额外加分 */
  nameMatch: number
  /** 目录访问时间权重: 最近访问过的目录中的文件额外加分 */
  dirAccess: number
  /** 推荐权重: 系统智能推荐的高额加分 (用于跨类别推荐) */
  recommendation: number
  /** 新鲜度衰减因子: 访问时间越久, 衰减越多 (每小时衰减比例) */
  freshnessDecayPerHour: number
}

/**
 * 智能排序权重配置.
 *
 * 权重设计原则:
 * - recommendation >> launchCount: 推荐项应该显著排在前面
 * - launchCount > nameMatch: 实际使用频率比名称匹配更重要
 * - dirAccess 适中: 目录访问时间作为辅助信号
 *
 * 当前权重经过实测调优, 如需调整请同步修改注释中的说明.
 */
export const SMART_WEIGHTS: SmartSortWeights = {
  launchCount: 10,       // 每次访问 +10 分
  nameMatch: 5,          // 名称命中 +5 分
  dirAccess: 8,          // 目录最近访问 +8 分
  recommendation: 50,    // 智能推荐 +50 分 (高权重, 确保推荐项靠前)
  freshnessDecayPerHour: 0.02, // 每小时衰减 2%
}

// ============================================================================
// 应用类别分组 (用于智能推荐)
// ============================================================================

/**
 * 应用类别分组映射.
 *
 * 用于智能推荐: 根据用户已打开的应用类别, 推荐同组或相关组的应用.
 * 分组设计参考了常见用户行为模式:
 * - 编程开发 → 推荐工具类 (编辑器、终端、版本控制)
 * - 通讯社交 → 推荐效率工具 (截图、浏览器、笔记)
 * - 媒体娱乐 → 推荐创作工具 (编辑器、转换器)
 * - 办公学习 → 推荐工具类 (计算器、词典、翻译)
 */
export const APP_CATEGORIES: Record<string, string[]> = {
  // 编程开发工具
  dev: [
    'code', 'vscode', 'visual studio', 'webstorm', 'pycharm', 'intellij',
    'clion', 'rider', 'goland', 'rustrover', 'fleet', 'android studio',
    'xcode', 'sublime', 'atom', 'notepad++', 'vim', 'emacs', 'neovim',
    'cursor', 'zed', 'lapce',
  ],
  // 终端/命令行
  terminal: [
    'terminal', 'powershell', 'cmd', 'wt', 'windows terminal', 'conemu',
    'cmder', 'alacritty', 'hyper', 'iterm', 'guake', 'kitty', 'tabby',
    'warp', 'ghostty', 'zsh', 'bash', 'fish',
  ],
  // 版本控制
  vcs: [
    'git', 'github', 'gitlab', 'sourcetree', 'fork', 'gitkraken',
    'tortoisegit', 'smartgit', 'sublime merge',
  ],
  // 通讯社交
  communication: [
    'wechat', 'qq', 'dingtalk', 'feishu', 'lark', 'slack', 'discord',
    'teams', 'zoom', 'skype', 'telegram', 'whatsapp', 'signal',
    'line', 'enterprise wechat', '企业微信', '钉钉', '飞书',
  ],
  // 浏览器
  browser: [
    'chrome', 'edge', 'firefox', 'safari', 'opera', 'brave', 'vivaldi',
    'chromium', 'arc', 'centbrowser', '360', 'qq browser', '搜狗浏览器',
  ],
  // 媒体播放
  media: [
    'vlc', 'potplayer', 'mpv', 'mplayer', 'wmplayer', 'itunes',
    'spotify', 'netease', 'qq music', '酷狗', '酷我', '网易云音乐',
    'bilibili', 'youtube', '爱奇艺', '腾讯视频',
  ],
  // 图像处理
  image: [
    'photoshop', 'ps', 'gimp', 'sai', 'clip studio', 'krita',
    'figma', 'sketch', 'affinity', 'canva', 'paint', '画图',
  ],
  // 办公文档
  office: [
    'word', 'excel', 'powerpoint', 'ppt', 'wps', 'pages', 'numbers',
    'keynote', 'notion', 'onenote', 'evernote', 'obsidian', 'logseq',
  ],
  // 文件管理
  filemanager: [
    'explorer', 'files', 'total commander', 'directory opus', 'xyplorer',
    'qtcoder', 'free commander', 'double commander', 'mucommander',
  ],
  // 下载工具
  download: [
    'thunder', 'bitcomet', 'utorrent', 'fdm', 'idm', 'aria2', 'motrix',
    'ndm', 'neat download',
  ],
  // 压缩工具
  archive: [
    '7zip', '7-zip', 'winrar', 'bandizip', 'haozip', '好压', '快压',
    'peazip', 'izarc',
  ],
}

/**
 * 推荐映射: 当用户打开了左侧类别的应用时, 推荐右侧类别的应用.
 *
 * 设计原则:
 * - 高权重: 推荐项在智能排序中获得 SMART_WEIGHTS.recommendation 加分
 * - 双向关联: A→B 时通常也 B→A (如 dev↔terminal)
 * - 跨类别: 通讯→效率工具, 开发→工具链
 */
export const RECOMMENDATION_MAP: Record<string, string[]> = {
  // 开发工具用户 → 推荐终端 + 版本控制 + 浏览器
  dev: ['terminal', 'vcs', 'browser', 'filemanager'],
  terminal: ['dev', 'vcs', 'filemanager'],
  vcs: ['dev', 'terminal'],

  // 通讯用户 → 推荐浏览器 + 截图 + 效率工具
  communication: ['browser', 'image', 'office'],
  browser: ['communication', 'download', 'office'],

  // 媒体用户 → 推荐图像处理 + 下载工具
  media: ['image', 'download'],
  image: ['media', 'office'],

  // 办公用户 → 推荐文件管理 + 浏览器
  office: ['filemanager', 'browser', 'communication'],
  filemanager: ['archive', 'download', 'office'],

  // 下载/压缩 → 文件管理
  download: ['archive', 'filemanager'],
  archive: ['filemanager', 'download'],
}

// ============================================================================
// 分组默认排序模式
// ============================================================================

/**
 * 每个分组类型的默认排序模式.
 *
 * 设计原则:
 * - pinned (固定项目): 智能排序, 让常用项靠前
 * - recent (最近访问): 按访问时间排序
 * - apps (应用程序): 智能排序, 结合使用频率和推荐
 * - system (系统应用): 按名称排序 (系统应用通常不需要智能排序)
 * - commands (命令): 按名称排序
 * - files (文件): 智能排序 (结合访问频率)
 */
export const DEFAULT_SORT_BY_GROUP: Record<string, SortMode> = {
  pinned: 'smart',
  recent: 'recent',
  apps: 'smart',
  system: 'name',
  commands: 'name',
  files: 'smart',
}

/**
 * 哪些分组支持排序功能.
 *
 * 所有分组都支持排序 (名称/最近访问/路径), 但仅 apps (所有程序) 和
 * commands (命令) 支持"智能排序"选项.
 */
export const SORTABLE_GROUPS: string[] = ['pinned', 'recent', 'apps', 'files', 'commands']

/**
 * 哪些分组支持"智能排序"选项.
 * 智能排序需要访问次数 + 推荐权重等信号, 仅 apps/commands 有意义.
 */
export const SMART_SORT_GROUPS: string[] = ['apps', 'commands']

// ============================================================================
// 分组默认布局模式
// ============================================================================

export const DEFAULT_LAYOUT_BY_GROUP: Record<string, string> = {
  pinned: 'grid-fixed',
  recent: 'grid-fixed',
  apps: 'icon',
  system: 'list',
  commands: 'list',
  files: 'list',
}
