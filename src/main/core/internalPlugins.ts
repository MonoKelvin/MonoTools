import { app } from 'electron'
import path from 'path'

/**
 * 随包内置插件名称列表
 * 这些插件存在于 internal-plugins 目录，并由宿主在启动时自动装载。
 */
export const BUNDLED_INTERNAL_PLUGIN_NAMES = ['setting', 'system'] as const

/**
 * 内置主题插件名称列表
 * 这些主题作为特殊插件存在于 internal-plugins/themes 目录
 */
export const BUNDLED_THEME_NAMES = ['linear-dark', 'vercel-light'] as const

/**
 * 内部 API 特权插件名称列表
 * 这些插件允许调用 window.monotools.internal，但不一定是随包内置插件。
 */
export const INTERNAL_API_PLUGIN_NAMES = [
  ...BUNDLED_INTERNAL_PLUGIN_NAMES,
  'monotools-developer-plugin__dev',
  'monotools-developer-plugin'
] as const

export type BundledInternalPluginName = (typeof BUNDLED_INTERNAL_PLUGIN_NAMES)[number]
export type BundledThemeName = (typeof BUNDLED_THEME_NAMES)[number]
export type InternalApiPluginName = (typeof INTERNAL_API_PLUGIN_NAMES)[number]

export const CUSTOM_INTERNAL_API_PLUGIN_NAMES_KEY = 'customInternalApiPluginNames'

export function normalizeCustomInternalApiPluginNames(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return []
  }

  return Array.from(
    new Set(
      value
        .map((name) => (typeof name === 'string' ? name.trim() : ''))
        .filter((name) => name.length > 0)
    )
  )
}

/**
 * 判断是否为随包内置插件
 * @param pluginName 插件名称
 * @returns 是否为随包内置插件
 */
export function isBundledInternalPlugin(pluginName: string): boolean {
  return BUNDLED_INTERNAL_PLUGIN_NAMES.includes(pluginName as BundledInternalPluginName)
}

/**
 * 判断插件是否允许调用内部 API
 * @param pluginName 插件名称
 * @returns 是否拥有内部 API 权限
 */
export function canPluginUseInternalApi(
  pluginName: string,
  customPluginNames: string[] = []
): boolean {
  if (INTERNAL_API_PLUGIN_NAMES.includes(pluginName as InternalApiPluginName)) {
    return true
  }

  return customPluginNames.includes(pluginName)
}

/**
 * 判断是否为内置主题
 * @param pluginName 插件名称
 * @returns 是否为内置主题
 */
export function isBundledTheme(pluginName: string): boolean {
  return BUNDLED_THEME_NAMES.includes(pluginName as BundledThemeName)
}

/**
 * 获取内置插件或主题路径
 * @param pluginName 插件名称或主题名称
 * @param isTheme 是否为主题
 * @returns 插件路径
 */
export function getInternalPluginPath(
  pluginName: BundledInternalPluginName | BundledThemeName,
  isTheme = false
): string {
  const isDev = !app.isPackaged

  if (isDev) {
    // 开发环境：使用源码目录
    if (isTheme) {
      return path.resolve(process.cwd(), 'internal-plugins', 'themes', pluginName)
    }
    return path.resolve(process.cwd(), 'internal-plugins', pluginName)
  } else {
    // 生产环境：从 resources 加载
    if (isTheme) {
      return path.join(
        process.resourcesPath,
        'app.asar.unpacked',
        'internal-plugins',
        'themes',
        pluginName
      )
    }
    return path.join(process.resourcesPath, 'app.asar.unpacked', 'internal-plugins', pluginName)
  }
}
