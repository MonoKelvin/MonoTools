import { defineStore } from 'pinia'

export { useThemeStore } from './theme'
export { useSettingsStore } from './settings'
export { useSearchStore } from './search'
export { useStartupStore } from './startup'

import { createPinia } from 'pinia'

export const pinia = createPinia()
