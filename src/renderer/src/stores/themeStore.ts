/**
 * 主题状态管理
 *
 * 管理当前主题状态，提供主题切换、主题信息查询等功能
 * 插件化架构：主题作为特殊插件，复用插件管理机制
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export const useThemeStore = defineStore('theme', () => {
  // 当前主题 ID
  const currentThemeId = ref<string>('linear-dark')

  // 当前主题配置
  const currentTheme = ref<any>(null)

  // 所有可用主题列表
  const availableThemes = ref<any[]>([])

  // 主题加载状态
  const loading = ref(false)

  // 错误信息
  const error = ref<string | null>(null)

  // 计算属性：是否为深色主题
  const isDarkTheme = computed(() => {
    return currentTheme.value?.themeType === 'dark'
  })

  /**
   * 加载所有主题
   */
  async function loadThemes(): Promise<void> {
    loading.value = true
    error.value = null

    try {
      const result = await window.monotools.loadThemes()
      if (result.success && result.themes) {
        availableThemes.value = result.themes
        console.log('[ThemeStore] 主题列表已加载:', result.themes.length)
      } else {
        console.error('[ThemeStore] 加载主题失败:', result.error)
        error.value = result.error || '加载主题失败'
      }
    } catch (error: any) {
      console.error('[ThemeStore] 加载主题异常:', error)
      error.value = error.message || '加载主题失败'
    } finally {
      loading.value = false
    }
  }

  /**
   * 应用主题
   * @param themeId 主题 ID
   */
  async function applyTheme(themeId: string): Promise<void> {
    const theme = availableThemes.value.find((t) => t.id === themeId)
    if (!theme) {
      console.error(`[ThemeStore] 主题不存在: ${themeId}`)
      return
    }

    loading.value = true
    error.value = null

    try {
      const result = await window.monotools.applyTheme(themeId, {
        injectCustomCSS: true
      })

      if (result.success) {
        currentThemeId.value = themeId
        // 更新当前主题配置
        const themeConfig = await getThemeById(themeId)
        currentTheme.value = themeConfig

        // 更新主题启用状态
        const themeInfo = availableThemes.value.find((t) => t.id === themeId)
        if (themeInfo) {
          themeInfo.isEnabled = true
          // 禁用其他主题
          availableThemes.value.forEach((t) => {
            if (t.id !== themeId) {
              t.isEnabled = false
            }
          })
        }

        console.log('[ThemeStore] 主题已应用:', theme.name)
      } else {
        console.error('[ThemeStore] 应用主题失败:', result.error)
        error.value = result.error || '应用主题失败'
      }
    } catch (error: any) {
      console.error('[ThemeStore] 应用主题异常:', error)
      error.value = error.message || '应用主题失败'
    } finally {
      loading.value = false
    }
  }

  /**
   * 禁用当前主题
   */
  async function disableCurrentTheme(): Promise<void> {
    try {
      const result = await window.monotools.disableTheme()
      if (result.success) {
        currentThemeId.value = ''
        currentTheme.value = null
        // 更新主题启用状态
        availableThemes.value.forEach((t) => {
          t.isEnabled = false
        })
        console.log('[ThemeStore] 主题已禁用')
      } else {
        console.error('[ThemeStore] 禁用主题失败:', result.error)
      }
    } catch (error: any) {
      console.error('[ThemeStore] 禁用主题异常:', error)
    }
  }

  /**
   * 获取指定主题配置
   * @param themeId 主题 ID
   */
  async function getThemeById(themeId: string): Promise<any> {
    try {
      const result = await window.monotools.getThemeById(themeId)
      if (result.success && result.theme) {
        return result.theme
      }
      return null
    } catch (error) {
      console.error(`[ThemeStore] 获取主题失败: ${themeId}`, error)
      return null
    }
  }

  /**
   * 刷新当前主题（重新应用）
   */
  async function refreshCurrentTheme(): Promise<void> {
    if (currentThemeId.value) {
      await applyTheme(currentThemeId.value)
    }
  }

  /**
   * 初始化主题（从存储读取并加载）
   */
  async function initTheme(): Promise<void> {
    try {
      // 先加载主题列表
      await loadThemes()

      // 从本地存储读取用户选择的主题
      const savedThemeId = localStorage.getItem('selectedThemeId')
      if (savedThemeId) {
        const themeExists = availableThemes.value.some((t) => t.id === savedThemeId)
        if (themeExists) {
          await applyTheme(savedThemeId)
          return
        }
      }

      // 默认应用第一个可用主题（通常是 linear-dark）
      if (availableThemes.value.length > 0) {
        const defaultTheme = availableThemes.value[0]
        await applyTheme(defaultTheme.id)
      }
    } catch (error) {
      console.error('[ThemeStore] 初始化主题失败:', error)
    }
  }

  return {
    // 状态
    currentThemeId,
    currentTheme,
    availableThemes,
    loading,
    error,
    // 计算属性
    isDarkTheme,
    // 方法
    loadThemes,
    applyTheme,
    disableCurrentTheme,
    getThemeById,
    refreshCurrentTheme,
    initTheme
  }
})
