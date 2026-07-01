/**
 * 主题生命周期管理器
 *
 * 专门处理主题插件的特殊逻辑：
 * - 启用：注入 CSS
 * - 禁用：移除 CSS
 * - 主题插件不创建 WebContentsView
 */

import { BrowserWindow } from 'electron'
import fs from 'fs'
import path from 'path'

/**
 * 主题生命周期管理
 */
export class ThemeLifecycle {
  private mainWindow?: BrowserWindow
  private currentThemePluginPath?: string

  /**
   * 设置主窗口
   */
  setMainWindow(window: BrowserWindow): void {
    this.mainWindow = window
  }

  /**
   * 判断插件是否为主题插件
   * @param plugin 插件信息
   */
  isThemePlugin(plugin: any): boolean {
    return plugin.isTheme === true || plugin.features?.[0]?.type === 'theme'
  }

  /**
   * 主题插件启用（加载 CSS）
   * @param plugin 插件信息
   */
  async onThemePluginEnabled(plugin: any): Promise<void> {
    if (!this.isThemePlugin(plugin)) {
      return
    }

    console.log('[ThemeLifecycle] 启用主题插件:', plugin.name)

    try {
      // 查找 style.css
      const cssPath = this.findThemeCSS(plugin.path)
      if (!cssPath) {
        console.warn(`[ThemeLifecycle] 主题插件 ${plugin.name} 未找到 style.css`)
        return
      }

      // 读取 CSS 内容
      const cssContent = fs.readFileSync(cssPath, 'utf-8')

      // 注入到渲染进程
      await this.injectThemeCSS(cssContent)

      // 记录当前启用的主题
      this.currentThemePluginPath = plugin.path

      console.log(`[ThemeLifecycle] 主题插件 ${plugin.name} 已启用`)
    } catch (error) {
      console.error(`[ThemeLifecycle] 启用主题插件 ${plugin.name} 失败:`, error)
    }
  }

  /**
   * 主题插件禁用（移除 CSS）
   * @param plugin 插件信息
   */
  async onThemePluginDisabled(plugin: any): Promise<void> {
    if (!this.isThemePlugin(plugin)) {
      return
    }

    console.log('[ThemeLifecycle] 禁用主题插件:', plugin.name)

    try {
      // 移除主题 CSS
      await this.removeThemeCSS()

      // 清除记录
      if (this.currentThemePluginPath === plugin.path) {
        this.currentThemePluginPath = undefined
      }

      console.log(`[ThemeLifecycle] 主题插件 ${plugin.name} 已禁用`)
    } catch (error) {
      console.error(`[ThemeLifecycle] 禁用主题插件 ${plugin.name} 失败:`, error)
    }
  }

  /**
   * 查找主题插件的 style.css
   * @param pluginPath 插件路径
   */
  private findThemeCSS(pluginPath: string): string | null {
    // 可能的 CSS 文件路径
    const possiblePaths = [
      path.join(pluginPath, 'style.css'),
      path.join(pluginPath, 'public', 'style.css'),
      path.join(pluginPath, 'dist', 'style.css')
    ]

    for (const cssPath of possiblePaths) {
      if (fs.existsSync(cssPath)) {
        return cssPath
      }
    }

    return null
  }

  /**
   * 注入主题 CSS 到渲染进程
   * @param cssContent CSS 内容
   */
  private async injectThemeCSS(cssContent: string): Promise<void> {
    if (!this.mainWindow) {
      console.warn('[ThemeLifecycle] 主窗口未设置，无法注入主题 CSS')
      return
    }

    try {
      // 在 #app 元素上设置 data-theme 属性
      await this.mainWindow.webContents.executeJavaScript(`
        (function() {
          const appEl = document.getElementById('app')
          if (appEl) {
            appEl.setAttribute('data-theme', 'active')
          }
        })()
      `)

      // 移除旧的主题样式
      await this.mainWindow.webContents.executeJavaScript(`
        (function() {
          const oldStyle = document.getElementById('theme-plugin-styles')
          if (oldStyle) {
            oldStyle.remove()
          }
        })()
      `)

      // 创建新的样式标签
      await this.mainWindow.webContents.executeJavaScript(`
        (function() {
          const style = document.createElement('style')
          style.id = 'theme-plugin-styles'
          style.textContent = ${JSON.stringify(cssContent)}
          document.head.appendChild(style)

          console.log('[Theme] 主题样式已注入')
        })()
      `)
    } catch (error) {
      console.error('[ThemeLifecycle] 注入主题 CSS 失败:', error)
    }
  }

  /**
   * 移除主题 CSS
   */
  private async removeThemeCSS(): Promise<void> {
    if (!this.mainWindow) {
      return
    }

    try {
      await this.mainWindow.webContents.executeJavaScript(`
        (function() {
          const style = document.getElementById('theme-plugin-styles')
          if (style) {
            style.remove()
            console.log('[Theme] 主题样式已移除')
          }
        })()
      `)
    } catch (error) {
      console.error('[ThemeLifecycle] 移除主题 CSS 失败:', error)
    }
  }

  /**
   * 重新加载当前主题
   */
  async reloadCurrentTheme(): Promise<void> {
    if (!this.mainWindow || !this.currentThemePluginPath) {
      return
    }

    // 找到当前主题的插件信息
    const plugins = this.mainWindow.webContents.executeJavaScript(
      `window.monotools.dbGet('plugins')`
    ) as Promise<any[]>
    const currentPlugin = (await plugins)?.find((p: any) => p.path === this.currentThemePluginPath)

    if (currentPlugin && this.isThemePlugin(currentPlugin)) {
      await this.onThemePluginEnabled(currentPlugin)
    }
  }
}

/**
 * 全局单例
 */
let themeLifecycle: ThemeLifecycle | null = null

/**
 * 获取主题生命周期管理器单例
 */
export function getThemeLifecycle(): ThemeLifecycle {
  if (!themeLifecycle) {
    themeLifecycle = new ThemeLifecycle()
  }
  return themeLifecycle
}
