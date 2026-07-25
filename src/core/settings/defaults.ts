// ============================================================
// 核心/全局设置注册 (外观、窗口行为、快捷键、关于)
// ============================================================

import { defineModuleSettings } from '@/modules/settings'
import { hotkeyApi } from '@/services'
import { useThemeStore } from '@/core/stores/theme'
import type { ThemeMode } from '@/core/types/settings'

export const coreSettings = defineModuleSettings({
  moduleId: 'core',
  order: 0,
  groups: [
    {
      id: 'appearance',
      label: '外观',
      icon: 'Monitor',
      order: 10,
      description: '极简黑白灰主题，所有交互元素使用暖白色调',
      items: [
        {
          key: 'theme',
          type: 'select',
          label: '主题模式',
          description: '深色 / 浅色 / 跟随系统',
          default: 'dark',
          options: [
            { label: '深色', value: 'dark' },
            { label: '浅色', value: 'light' },
            { label: '跟随系统', value: 'auto' },
          ],
          onChange: async (value) => {
            const themeStore = useThemeStore()
            await themeStore.setMode(value as ThemeMode)
          },
        },
        {
          key: 'followSystemTheme',
          type: 'boolean',
          label: '跟随系统主题',
          description: '自动同步 Windows 浅色/深色模式',
          default: false,
          onChange: async (value) => {
            const themeStore = useThemeStore()
            themeStore.setFollowSystem(value as boolean)
          },
        },
      ],
    },
    {
      id: 'window',
      label: '窗口行为',
      icon: 'LayoutPanelTop',
      order: 20,
      items: [
        {
          key: 'pinToTop',
          type: 'boolean',
          label: '窗口置顶',
          description: '开启后搜索窗口始终显示在最上层',
          default: true,
        },
        {
          key: 'hotkey',
          type: 'hotkey',
          label: '全局快捷键',
          description: '按下此组合键唤起/隐藏搜索窗口',
          default: 'Alt+Space',
          onChange: async (value) => {
            if (typeof value === 'string' && value) {
              await hotkeyApi.unregister()
              await hotkeyApi.register(value)
            }
          },
        },
      ],
    },
    {
      id: 'about',
      label: '关于',
      icon: 'Info',
      order: 999,
      items: [
        {
          key: 'appVersion',
          type: 'info',
          label: '版本',
          content: import.meta.env.VITE_APP_VERSION || '0.1.0',
          default: null,
        },
      ],
    },
  ],
})
