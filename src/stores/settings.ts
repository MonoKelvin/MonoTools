import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Settings } from '@/types/settings'
import { settingsApi } from '@/services/settingsApi'

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<Settings>({
    hotkey: 'Alt+Space',
    theme: 'dark',
    accentColor: '#ff6b6b',
    fileSearchEnabled: true,
    fileSearchRoots: [],
    defaultDelaySeconds: 0,
    autoIndexOnStartup: true,
    enabledCategories: ['apps', 'files', 'commands', 'startup'],
    customAppPaths: [],
  })

  async function load() {
    try {
      const data = await settingsApi.getAll()
      if (data) settings.value = { ...settings.value, ...data }
    } catch (err) {
      console.warn('加载设置失败：', err)
    }
  }

  async function update(partial: Partial<Settings>) {
    settings.value = { ...settings.value, ...partial }
    await settingsApi.setAll(settings.value)
  }

  return { settings, load, update }
})
