import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Settings } from '@/types/settings'
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
      /* 设置加载失败已在调用方（SettingsPanel）处理；此处静默避免控制台噪音 */
    }
  }

  async function update(partial: Partial<Settings>) {
    settings.value = { ...settings.value, ...partial }
    await settingsApi.setAll(settings.value)
  }

  return { settings, load, update }
})
