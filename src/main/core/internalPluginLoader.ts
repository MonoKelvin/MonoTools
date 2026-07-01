import { app } from 'electron'
import fsSync from 'fs'
import path from 'path'
import { pathToFileURL } from 'url'
import api from '../api/index'
import {
  BUNDLED_INTERNAL_PLUGIN_NAMES,
  BUNDLED_THEME_NAMES,
  getInternalPluginPath
} from './internalPlugins'
import { getInternalPluginUrl, getInternalPluginServerPort } from './internalPluginServer'

/**
 * 加载所有内置插件和主题
 * 在应用启动时调用，自动将内置插件添加到数据库
 */
export function loadInternalPlugins(): void {
  console.log('[InternalPlugin] 开始加载内置插件和主题...')

  const isDev = !app.isPackaged
  const existingPlugins = api.dbGet('plugins') || []

  // 移除旧的内置插件记录（基于名称判断）
  const filteredPlugins = existingPlugins.filter(
    (p: any) =>
      !BUNDLED_INTERNAL_PLUGIN_NAMES.includes(p.name) && !BUNDLED_THEME_NAMES.includes(p.name)
  )

  // 重新加载所有内置插件
  for (const pluginName of BUNDLED_INTERNAL_PLUGIN_NAMES) {
    try {
      const pluginPath = getInternalPluginPath(pluginName)
      const effectivePluginPath = isDev ? path.join(pluginPath, 'public') : pluginPath
      const pluginJsonPath = path.join(effectivePluginPath, 'plugin.json')

      if (!fsSync.existsSync(pluginJsonPath)) {
        console.error(
          `[InternalPlugin] 内置插件 ${pluginName} 的 plugin.json 不存在:`,
          pluginJsonPath
        )
        continue
      }

      const pluginConfig = JSON.parse(fsSync.readFileSync(pluginJsonPath, 'utf-8'))
      const logoPath = pluginConfig.logo ? path.join(effectivePluginPath, pluginConfig.logo) : ''

      const serverPort = getInternalPluginServerPort()
      const mainPath = pluginConfig.main
        ? serverPort > 0
          ? getInternalPluginUrl(pluginName, pluginConfig.main)
          : path.join(effectivePluginPath, pluginConfig.main)
        : undefined

      const pluginInfo = {
        name: pluginConfig.name,
        title: pluginConfig.title,
        version: pluginConfig.version,
        description: pluginConfig.description || '',
        logo: logoPath ? pathToFileURL(logoPath).href : '',
        path: effectivePluginPath,
        features: pluginConfig.features || [],
        isDevelopment: isDev,
        main: mainPath
      }

      filteredPlugins.push(pluginInfo)
      console.log(`[InternalPlugin] 已加载内置插件: ${pluginName}`)
    } catch (error) {
      console.error(`[InternalPlugin] 加载内置插件 ${pluginName} 失败:`, error)
    }
  }

  // 加载所有内置主题插件
  for (const themeName of BUNDLED_THEME_NAMES) {
    try {
      const themePath = getInternalPluginPath(themeName, true)
      const effectiveThemePath = isDev ? path.join(themePath, 'public') : themePath
      const pluginJsonPath = path.join(effectiveThemePath, 'plugin.json')

      if (!fsSync.existsSync(pluginJsonPath)) {
        console.error(
          `[InternalPlugin] 内置主题 ${themeName} 的 plugin.json 不存在:`,
          pluginJsonPath
        )
        continue
      }

      const themeConfig = JSON.parse(fsSync.readFileSync(pluginJsonPath, 'utf-8'))

      // 主题插件不需要 logo 和 main 字段，但为了兼容插件数据库，保留这些字段
      const logoPath = themeConfig.icon ? path.join(effectiveThemePath, themeConfig.icon) : ''

      const themeInfo = {
        name: themeConfig.name,
        title: themeConfig.title || themeConfig.name,
        version: themeConfig.version,
        description: themeConfig.description || '',
        logo: logoPath ? pathToFileURL(logoPath).href : '',
        path: effectiveThemePath,
        features: [], // 主题插件不需要 features
        isDevelopment: isDev,
        main: undefined, // 主题插件没有 WebContentsView
        isTheme: true, // 标记为主题插件
        themeType: themeConfig.themeType || 'system'
      }

      filteredPlugins.push(themeInfo)
      console.log(`[InternalPlugin] 已加载内置主题: ${themeName}`)
    } catch (error) {
      console.error(`[InternalPlugin] 加载内置主题 ${themeName} 失败:`, error)
    }
  }

  // 保存到数据库
  api.dbPut('plugins', filteredPlugins)
  console.log('[InternalPlugin] 内置插件和主题加载完成')
}
