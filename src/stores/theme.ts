import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { ThemeMode } from '@/types/settings'
import { settingsApi } from '@/services/settingsApi'

const THEME_KEY = 'theme'

export const useThemeStore = defineStore('theme', () => {
  const mode = ref<ThemeMode>('dark')
  const accent = ref('#ff6b6b')

  async function init() {
    try {
      const saved = await settingsApi.get<{ mode: ThemeMode; accent: string }>('appearance')
      if (saved) {
        mode.value = saved.mode
        accent.value = saved.accent
      }
    } catch {
      // 默认值
    }
  }

  async function applyTheme() {
    await init()
    updateClass()
    listenSystemTheme()
  }

  function updateClass() {
    const html = document.documentElement
    html.classList.remove('theme-dark', 'theme-light')
    if (mode.value === 'auto') {
      const system = window.matchMedia('(prefers-color-scheme: dark)').matches
      html.classList.add(system ? 'theme-dark' : 'theme-light')
      html.style.colorScheme = system ? 'dark' : 'light'
    } else {
      html.classList.add(`theme-${mode.value}`)
      html.style.colorScheme = mode.value === 'dark' ? 'dark' : 'light'
    }
    document.documentElement.style.setProperty('--accent', accent.value)
  }

  async function setMode(next: ThemeMode) {
    mode.value = next
    updateClass()
    await settingsApi.set('appearance', { mode: mode.value, accent: accent.value })
  }

  async function setAccent(color: string) {
    accent.value = color
    updateClass()
    await settingsApi.set('appearance', { mode: mode.value, accent: accent.value })
  }

  function listenSystemTheme() {
    if (!window.matchMedia) return
    const mq = window.matchMedia('(prefers-color-scheme: dark)')
    const fn = () => mode.value === 'auto' && updateClass()
    mq.addEventListener?.('change', fn)
  }

  return { mode, accent, init, applyTheme, setMode, setAccent }
})
