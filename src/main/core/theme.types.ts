/**
 * 主题类型定义（插件化架构 - 简化版）
 *
 * 设计哲学：
 * - JSON 只保留元数据和特殊配置
 * - 颜色/字体/间距/圆角等全部由 style.css 定义
 * - style.css 是唯一样式来源
 */

/**
 * 主题配置接口
 */
export interface ThemeConfig {
  id: string
  name: string
  version: string
  description: string
  author: string
  homepage?: string
  license?: string
  type: 'theme' // 固定为 'theme'
  themeType: 'light' | 'dark' | 'system'
  tags?: string[]
  icon?: string
  screenshots?: string[]
  minAppVersion?: string
  config?: {
    fontDisplay?: string
    fontBody?: string
    fontMono?: string
    imports?: string[]
    [key: string]: any // 允许自定义配置
  }
}

/**
 * 主题信息（用于 UI 展示）
 */
export interface ThemeInfo {
  id: string
  name: string
  description: string
  author: string
  homepage?: string
  license?: string
  themeType: 'light' | 'dark' | 'system'
  version: string
  tags?: string[]
  isBuiltin: boolean
  isEnabled: boolean
  path: string // 主题根目录（用于复制/删除整个主题）
  publicPath: string // public 子目录（用于读取配置和样式）
  icon?: string
  screenshots?: string[]
  hasCustomCSS: boolean
}

/**
 * 主题应用选项
 */
export interface ThemeApplyOptions {
  injectCustomCSS?: boolean // 是否注入自定义 CSS（默认 true）
}

/**
 * 主题管理器接口
 */
export interface IThemeManager {
  // 主题生命周期
  loadThemes(): Promise<void>
  applyTheme(themeId: string, options?: ThemeApplyOptions): Promise<void>
  refreshTheme(): Promise<void>
  disableTheme(): Promise<void>

  // 主题信息
  getCurrentTheme(): ThemeConfig | null
  getAvailableThemes(): ThemeInfo[]
  getThemeById(themeId: string): ThemeConfig | null
  getThemePath(themeId: string): string | null
  getEnabledTheme(): ThemeInfo | null

  // 主题操作
  importTheme(sourcePath: string): Promise<ThemeInfo>
  exportTheme(themeId: string, targetPath: string): Promise<void>
  deleteTheme(themeId: string): Promise<boolean>
  enableTheme(themeId: string): Promise<boolean>
  validateTheme(configPath: string): Promise<{ valid: boolean; error?: string }>

  // 工具方法
  getThemeDir(): string
  getUserThemeDir(): string
  ensureThemeDirs(): Promise<void>
}
