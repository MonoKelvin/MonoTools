/**
 * 主题使用 Hook
 *
 * 提供便捷的主题相关功能访问
 */

import { computed, watch } from 'vue'
import { useThemeStore } from '../stores/themeStore'

/**
 * 主题 Hook
 */
export function useTheme() {
  const themeStore = useThemeStore()

  /**
   * 动态生成 CSS 类名（基于主题配置）
   * @param prefix 前缀
   * @param suffix 后缀
   */
  function getThemeClass(prefix: string, suffix?: string): string {
    return suffix ? `${prefix}-${suffix}` : prefix
  }

  /**
   * 监听主题变化
   * @param callback 回调函数
   */
  function onThemeChange(callback: (theme: any) => void): () => void {
    return watch(
      () => themeStore.currentTheme,
      (newTheme) => {
        callback(newTheme)
      },
      { immediate: true }
    )
  }

  /**
   * 动态生成 CSS 变量值
   * @param property 属性名
   * @param value 值
   */
  function getCSSVariable(property: string, value: string): string {
    return `var(--${property}, ${value})`
  }

  return {
    // 状态
    currentTheme: computed(() => themeStore.currentTheme),
    currentThemeId: computed(() => themeStore.currentThemeId),
    availableThemes: computed(() => themeStore.availableThemes),
    isDarkTheme: computed(() => themeStore.isDarkTheme),
    loading: computed(() => themeStore.loading),
    // 方法
    getThemeClass,
    getCSSVariable,
    onThemeChange,
    applyTheme: themeStore.applyTheme,
    loadThemes: themeStore.loadThemes,
    refreshCurrentTheme: themeStore.refreshCurrentTheme,
    initTheme: themeStore.initTheme
  }
}
