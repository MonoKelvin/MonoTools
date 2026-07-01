/**
 * 主题管理器核心实现（插件化架构）
 *
 * 职责：
 * 1. 扫描并加载所有主题（默认 + 用户自定义）
 * 2. 将主题 CSS 注入到页面
 * 3. 支持热切换主题
 * 4. 支持导入/导出/删除主题
 * 5. 支持启用/禁用主题（复用插件机制）
 *
 * 主题即插件架构：
 * - 主题作为特殊插件类型，复用插件的安装、启用/禁用机制
 * - 主题存储在 plugins 数据库中，type 为 'theme'
 * - 主题启用时自动应用，禁用时恢复默认
 */

import { app } from 'electron'
import { promises as fs } from 'fs'
import path from 'path'
import { ThemeConfig, ThemeInfo, ThemeApplyOptions, IThemeManager } from './theme.types.js'

/**
 * 主题管理器实现类
 */
export class ThemeManager implements IThemeManager {
  private themes: Map<string, ThemeConfig> = new Map()
  private themeInfos: Map<string, ThemeInfo> = new Map()
  private currentThemeId: string | null = null
  private themeDir: string
  private userThemeDir: string
  private mainWindow?: Electron.BrowserWindow

  // 默认主题 ID
  private static readonly DEFAULT_THEME_ID = 'linear-dark'

  constructor() {
    // 内置主题目录（在 internal-plugins/themes 中）
    const isDev = !app.isPackaged
    if (isDev) {
      // 开发环境：使用源码目录
      this.themeDir = path.resolve(process.cwd(), 'internal-plugins', 'themes')
    } else {
      // 生产环境：从 app.asar.unpacked 加载
      this.themeDir = path.join(
        process.resourcesPath,
        'app.asar.unpacked',
        'internal-plugins',
        'themes'
      )
    }
    // 用户主题目录（用户数据目录下的 themes）
    this.userThemeDir = path.join(app.getPath('userData'), 'themes')
  }

  /**
   * 设置主窗口引用（用于广播主题切换事件）
   */
  setMainWindow(window: Electron.BrowserWindow): void {
    this.mainWindow = window
  }

  /**
   * 获取主题根目录
   */
  getThemeDir(): string {
    return this.themeDir
  }

  /**
   * 获取用户主题目录
   */
  getUserThemeDir(): string {
    return this.userThemeDir
  }

  /**
   * 确保主题目录存在
   */
  async ensureThemeDirs(): Promise<void> {
    await fs.mkdir(this.userThemeDir, { recursive: true })
  }

  /**
   * 扫描并加载所有主题
   */
  async loadThemes(): Promise<void> {
    this.themes.clear()
    this.themeInfos.clear()

    // 确保用户主题目录存在
    await this.ensureThemeDirs()

    // 加载内置主题
    await this.loadThemesFromDir(this.themeDir, true)

    // 加载用户自定义主题
    await this.loadThemesFromDir(this.userThemeDir, false)
  }

  /**
   * 从目录加载主题
   * @param dir 主题目录
   * @param isBuiltin 是否为内置主题
   */
  private async loadThemesFromDir(dir: string, isBuiltin: boolean): Promise<void> {
    // 检查目录是否存在，不存在则跳过
    try {
      await fs.access(dir)
    } catch {
      // 目录不存在，跳过
      return
    }

    try {
      const entries = await fs.readdir(dir, { withFileTypes: true })

      for (const entry of entries) {
        if (!entry.isDirectory()) continue

        const themePath = path.join(dir, entry.name)

        // 主题插件采用与内置插件相同的结构：文件在 public 子目录中
        const effectiveThemePath = path.join(themePath, 'public')
        const configPath = path.join(effectiveThemePath, 'plugin.json')
        const stylesPath = path.join(effectiveThemePath, 'style.css')

        try {
          // 读取并解析 plugin.json
          const configContent = await fs.readFile(configPath, 'utf-8')
          const config: ThemeConfig = JSON.parse(configContent)

          // 验证必要字段
          if (!config.id || !config.type || config.type !== 'theme') {
            console.warn(`[ThemeManager] ${entry.name} 不是有效的主题配置，跳过`)
            continue
          }

          // 检查自定义 CSS 是否存在
          let hasCustomCSS = false
          try {
            await fs.access(stylesPath)
            hasCustomCSS = true
          } catch {
            // 无自定义 CSS，忽略
          }

          // 从插件数据库读取启用状态（复用插件机制）
          const isEnabled = await this.isThemeEnabled(config.id)

          // 存储主题配置
          this.themes.set(config.id, config)

          // 存储主题信息
          this.themeInfos.set(config.id, {
            id: config.id,
            name: config.name,
            description: config.description,
            author: config.author,
            homepage: config.homepage,
            license: config.license,
            themeType: config.themeType,
            version: config.version,
            tags: config.tags,
            isBuiltin,
            isEnabled,
            path: themePath, // 主题根目录（用于复制/删除整个主题）
            publicPath: effectiveThemePath, // public 子目录（用于读取配置和样式）
            icon: config.icon ? path.join(effectiveThemePath, config.icon) : undefined,
            screenshots: config.screenshots?.map((s) => path.join(effectiveThemePath, s)),
            hasCustomCSS
          })
        } catch (error) {
          console.error(`[ThemeManager] 加载主题 ${entry.name} 失败:`, error)
        }
      }
    } catch (error) {
      console.error(`[ThemeManager] 读取主题目录失败: ${dir}`, error)
    }
  }

  /**
   * 检查主题是否已启用（从插件数据库读取）
   * @param themeId 主题 ID
   */
  private async isThemeEnabled(themeId: string): Promise<boolean> {
    try {
      // 通过 IPC 调用渲染进程的 dbGet
      // 这里使用动态 import 避免循环依赖
      const result = await this.dbGet<{ enabled: boolean }>(`theme-${themeId}`)
      return result?.enabled ?? false
    } catch {
      return false
    }
  }

  /**
   * 从数据库读取数据
   * @param key 键
   */
  private async dbGet<T>(key: string): Promise<T | null> {
    // 通过 IPC 调用
    if (this.mainWindow) {
      try {
        return await this.mainWindow.webContents.executeJavaScript(
          `window.monotools.dbGet('${key}')`
        )
      } catch {
        return null
      }
    }
    return null
  }

  /**
   * 保存数据到数据库
   * @param key 键
   * @param value 值
   */
  private async dbPut(key: string, value: any): Promise<void> {
    if (this.mainWindow) {
      try {
        await this.mainWindow.webContents.executeJavaScript(
          `window.monotools.dbPut('${key}', ${JSON.stringify(value)})`
        )
      } catch (error) {
        console.error('[ThemeManager] 保存数据失败:', error)
      }
    }
  }

  /**
   * 应用主题（热切换）
   * @param themeId 主题 ID
   * @param options 应用选项
   */
  async applyTheme(themeId: string, options: ThemeApplyOptions = {}): Promise<void> {
    const theme = this.themes.get(themeId)
    if (!theme) {
      throw new Error(`主题不存在: ${themeId}`)
    }

    const { injectCustomCSS = true } = options

    this.currentThemeId = themeId

    // 注入自定义 CSS
    if (injectCustomCSS) {
      const publicPath = this.themeInfos.get(themeId)?.publicPath
      if (publicPath) {
        await this.injectCustomStyles(publicPath)
      }
    }

    // 保存当前主题到用户数据
    await this.saveCurrentTheme(themeId)

    // 更新主题启用状态
    await this.enableTheme(themeId)

    console.log(`[ThemeManager] 主题已应用: ${theme.name} (${themeId})`)
  }

  /**
   * 注入自定义 CSS
   * @param themePath 主题路径（public 目录）
   */
  private async injectCustomStyles(themePath: string): Promise<void> {
    const stylesPath = path.join(themePath, 'style.css')

    try {
      const cssContent = await fs.readFile(stylesPath, 'utf-8')

      // 使用 webContents.executeJavaScript 在渲染进程中注入样式
      if (!this.mainWindow) {
        console.warn('[ThemeManager] 主窗口未设置，无法注入主题 CSS')
        return
      }

      await this.mainWindow.webContents.executeJavaScript(`
        (function() {
          // 移除旧的主题样式
          const oldStyle = document.getElementById('theme-custom-styles')
          if (oldStyle) {
            oldStyle.remove()
          }

          // 创建新的样式标签
          const style = document.createElement('style')
          style.id = 'theme-custom-styles'
          style.textContent = ${JSON.stringify(cssContent)}
          document.head.appendChild(style)

          // 设置 data-theme 属性
          const appEl = document.getElementById('app')
          if (appEl) {
            appEl.setAttribute('data-theme', 'active')
          }

          console.log('[Theme] 主题样式已注入')
        })()
      `)

      console.log(`[ThemeManager] 主题 CSS 已注入: ${stylesPath}`)
    } catch (error) {
      console.error(`[ThemeManager] 注入主题 CSS 失败: ${stylesPath}`, error)
    }
  }

  /**
   * 禁用当前主题（恢复默认样式）
   */
  async disableTheme(): Promise<void> {
    // 移除自定义样式
    if (this.mainWindow) {
      try {
        await this.mainWindow.webContents.executeJavaScript(`
          (function() {
            const style = document.getElementById('theme-custom-styles')
            if (style) {
              style.remove()
              console.log('[Theme] 主题样式已移除')
            }

            // 移除 data-theme 属性
            const appEl = document.getElementById('app')
            if (appEl) {
              appEl.removeAttribute('data-theme')
            }
          })()
        `)
      } catch (error) {
        console.error('[ThemeManager] 移除主题样式失败:', error)
      }
    }

    this.currentThemeId = null

    // 更新数据库
    await this.dbPut('current-theme', { enabled: false })

    console.log('[ThemeManager] 主题已禁用')
  }

  /**
   * 启用主题
   * @param themeId 主题 ID
   */
  async enableTheme(themeId: string): Promise<boolean> {
    const theme = this.themes.get(themeId)
    if (!theme) {
      return false
    }

    // 保存启用状态到数据库
    await this.dbPut(`theme-${themeId}`, { enabled: true, appliedAt: Date.now() })

    // 更新主题信息
    const themeInfo = this.themeInfos.get(themeId)
    if (themeInfo) {
      themeInfo.isEnabled = true
    }

    // 禁用其他所有主题
    for (const [id, info] of this.themeInfos) {
      if (id !== themeId && info.isEnabled) {
        await this.dbPut(`theme-${id}`, { enabled: false })
        info.isEnabled = false
      }
    }

    return true
  }

  /**
   * 保存当前主题到用户数据
   * @param themeId 主题 ID
   */
  private async saveCurrentTheme(themeId: string): Promise<void> {
    await this.dbPut('current-theme', { themeId, appliedAt: Date.now() })
  }

  /**
   * 刷新当前主题（重新应用）
   */
  async refreshTheme(): Promise<void> {
    if (this.currentThemeId) {
      await this.applyTheme(this.currentThemeId)
    }
  }

  /**
   * 获取当前主题
   */
  getCurrentTheme(): ThemeConfig | null {
    if (!this.currentThemeId) return null
    return this.themes.get(this.currentThemeId) || null
  }

  /**
   * 获取所有可用主题列表
   */
  getAvailableThemes(): ThemeInfo[] {
    return Array.from(this.themeInfos.values())
  }

  /**
   * 获取已启用的主题
   */
  getEnabledTheme(): ThemeInfo | null {
    for (const info of this.themeInfos.values()) {
      if (info.isEnabled) {
        return info
      }
    }
    return null
  }

  /**
   * 根据 ID 获取主题配置
   * @param themeId 主题 ID
   */
  getThemeById(themeId: string): ThemeConfig | null {
    return this.themes.get(themeId) || null
  }

  /**
   * 根据 ID 获取主题路径
   * @param themeId 主题 ID
   */
  getThemePath(themeId: string): string | null {
    return this.themeInfos.get(themeId)?.path || null
  }

  /**
   * 导入主题
   * @param sourcePath 源路径（主题文件夹或 zip 包）
   */
  async importTheme(sourcePath: string): Promise<ThemeInfo> {
    // 先验证主题
    const validation = await this.validateTheme(sourcePath)
    if (!validation.valid) {
      throw new Error(validation.error || '主题验证失败')
    }

    // 复制到用户主题目录
    const themeName = path.basename(sourcePath)
    const targetPath = path.join(this.userThemeDir, themeName)

    await fs.cp(sourcePath, targetPath, { recursive: true })

    // 重新加载主题
    await this.loadThemes()

    // 返回导入的主题信息
    const themeInfo = this.themeInfos.get(themeName)
    if (!themeInfo) {
      throw new Error('主题导入失败')
    }

    return themeInfo
  }

  /**
   * 导出主题
   * @param themeId 主题 ID
   * @param targetPath 目标路径
   */
  async exportTheme(themeId: string, targetPath: string): Promise<void> {
    const themePath = this.themeInfos.get(themeId)?.path
    if (!themePath) {
      throw new Error(`主题不存在: ${themeId}`)
    }

    await fs.cp(themePath, targetPath, { recursive: true })
  }

  /**
   * 删除自定义主题
   * @param themeId 主题 ID
   */
  async deleteTheme(themeId: string): Promise<boolean> {
    const themeInfo = this.themeInfos.get(themeId)
    if (!themeInfo || themeInfo.isBuiltin) {
      return false // 内置主题不可删除
    }

    try {
      await fs.rm(themeInfo.path, { recursive: true, force: true })
      this.themes.delete(themeId)
      this.themeInfos.delete(themeId)

      // 如果删除的是当前主题，切换到默认主题
      if (this.currentThemeId === themeId) {
        await this.applyTheme(ThemeManager.DEFAULT_THEME_ID)
      }

      return true
    } catch (error) {
      console.error(`[ThemeManager] 删除主题失败: ${themeId}`, error)
      return false
    }
  }

  /**
   * 验证主题配置
   * @param configPath 主题配置路径
   */
  async validateTheme(configPath: string): Promise<{ valid: boolean; error?: string }> {
    try {
      const stats = await fs.stat(configPath)
      let themeJsonPath: string

      if (stats.isDirectory()) {
        // 先尝试根目录下的 plugin.json
        const rootPluginJson = path.join(configPath, 'plugin.json')
        try {
          await fs.access(rootPluginJson)
          themeJsonPath = rootPluginJson
        } catch {
          // 如果根目录没有，尝试 public 子目录
          themeJsonPath = path.join(configPath, 'public', 'plugin.json')
        }
      } else {
        themeJsonPath = configPath
      }

      const content = await fs.readFile(themeJsonPath, 'utf-8')
      const config: ThemeConfig = JSON.parse(content)

      // 验证必要字段
      if (!config.id) {
        return { valid: false, error: '缺少 id 字段' }
      }

      if (config.type !== 'theme') {
        return { valid: false, error: 'type 必须为 "theme"' }
      }

      if (!config.name) {
        return { valid: false, error: '缺少 name 字段' }
      }

      if (!config.themeType || !['light', 'dark', 'system'].includes(config.themeType)) {
        return { valid: false, error: 'themeType 必须为 light、dark 或 system' }
      }

      return { valid: true }
    } catch (error: any) {
      return { valid: false, error: error.message }
    }
  }
}

/**
 * 全局单例
 */
let themeManager: ThemeManager | null = null

/**
 * 获取主题管理器单例
 */
export function getThemeManager(): ThemeManager {
  if (!themeManager) {
    themeManager = new ThemeManager()
  }
  return themeManager
}
