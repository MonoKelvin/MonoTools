import { createPinia } from 'pinia'

export const pinia = createPinia()

export { useThemeStore } from './theme'
export { useSettingsStore } from './settings'
