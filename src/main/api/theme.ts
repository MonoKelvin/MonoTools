/**
 * 主题 IPC 接口
 *
 * 提供渲染进程和插件访问主题管理的通道
 */

import { ipcMain } from 'electron'
import { getThemeManager } from '../core/themeManager.js'
import type { ThemeApplyOptions } from '../core/theme.types.js'

/**
 * 主题 IPC 处理器
 */
export class ThemeIPC {
  private themeManager = getThemeManager()

  /**
   * 注册所有主题相关的 IPC 处理器
   */
  register(): void {
    // 加载所有主题
    ipcMain.handle('theme:load', async () => {
      try {
        await this.themeManager.loadThemes()
        return { success: true, themes: this.themeManager.getAvailableThemes() }
      } catch (error: any) {
        return { success: false, error: error.message }
      }
    })

    // 应用主题
    ipcMain.handle('theme:apply', async (_event, themeId: string, options?: ThemeApplyOptions) => {
      try {
        await this.themeManager.applyTheme(themeId, options)
        return { success: true }
      } catch (error: any) {
        return { success: false, error: error.message }
      }
    })

    // 获取当前主题
    ipcMain.handle('theme:getCurrent', async () => {
      try {
        const theme = this.themeManager.getCurrentTheme()
        return { success: true, theme }
      } catch (error: any) {
        return { success: false, error: error.message }
      }
    })

    // 获取所有可用主题列表
    ipcMain.handle('theme:getAvailable', async () => {
      try {
        const themes = this.themeManager.getAvailableThemes()
        return { success: true, themes }
      } catch (error: any) {
        return { success: false, error: error.message }
      }
    })

    // 根据 ID 获取主题
    ipcMain.handle('theme:getById', async (_event, themeId: string) => {
      try {
        const theme = this.themeManager.getThemeById(themeId)
        return { success: true, theme }
      } catch (error: any) {
        return { success: false, error: error.message }
      }
    })

    // 导入主题
    ipcMain.handle('theme:import', async (_event, sourcePath: string) => {
      try {
        const themeInfo = await this.themeManager.importTheme(sourcePath)
        return { success: true, theme: themeInfo }
      } catch (error: any) {
        return { success: false, error: error.message }
      }
    })

    // 导出主题
    ipcMain.handle('theme:export', async (_event, themeId: string, targetPath: string) => {
      try {
        await this.themeManager.exportTheme(themeId, targetPath)
        return { success: true }
      } catch (error: any) {
        return { success: false, error: error.message }
      }
    })

    // 删除主题
    ipcMain.handle('theme:delete', async (_event, themeId: string) => {
      try {
        const success = await this.themeManager.deleteTheme(themeId)
        return { success }
      } catch (error: any) {
        return { success: false, error: error.message }
      }
    })

    // 验证主题
    ipcMain.handle('theme:validate', async (_event, configPath: string) => {
      try {
        const result = await this.themeManager.validateTheme(configPath)
        return result
      } catch (error: any) {
        return { valid: false, error: error.message }
      }
    })

    // 禁用主题
    ipcMain.handle('theme:disable', async () => {
      try {
        await this.themeManager.disableTheme()
        return { success: true }
      } catch (error: any) {
        return { success: false, error: error.message }
      }
    })

    // 获取主题目录路径
    ipcMain.handle('theme:getThemeDir', async () => {
      try {
        return {
          success: true,
          themeDir: this.themeManager.getThemeDir(),
          userThemeDir: this.themeManager.getUserThemeDir()
        }
      } catch (error: any) {
        return { success: false, error: error.message }
      }
    })
  }

  /**
   * 注销所有 IPC 处理器
   */
  unregister(): void {
    const channels = [
      'theme:load',
      'theme:apply',
      'theme:getCurrent',
      'theme:getAvailable',
      'theme:getById',
      'theme:import',
      'theme:export',
      'theme:delete',
      'theme:validate',
      'theme:getThemeDir'
    ]

    for (const channel of channels) {
      ipcMain.removeHandler(channel)
    }
  }
}

/**
 * 全局单例
 */
let themeIPC: ThemeIPC | null = null

/**
 * 获取主题 IPC 单例
 */
export function getThemeIPC(): ThemeIPC {
  if (!themeIPC) {
    themeIPC = new ThemeIPC()
  }
  return themeIPC
}
