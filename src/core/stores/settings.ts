import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Settings } from '@/core/types/settings'
import { settingsApi } from '@/services'

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<Settings>({
    hotkey: 'Alt+Space',
    theme: 'dark',
    accentColor: '#ffffff',
    fileSearchEnabled: true,
    fileSearchRoots: [],
    enabledCategories: ['apps', 'files', 'commands'],
    pinToTop: true,
  })

  async function load() {
    try {
      const data = await settingsApi.getAll()
      if (data) settings.value = { ...settings.value, ...data }
    } catch {
      /* 设置加载失败静默处理 */
    }
  }

  async function update(partial: Partial<Settings>) {
    settings.value = { ...settings.value, ...partial }
    await settingsApi.setAll(settings.value)
  }

  return { settings, load, update }
})
