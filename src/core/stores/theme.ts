import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { ThemeMode } from '@/core/types/settings'
import { settingsApi } from '@/services/api'

const THEME_KEY = 'theme'

export const useThemeStore = defineStore('theme', () => {
    // Raycast 是 dark-only。保留 mode 字段以兼容设置存储，但 UI 永远渲染 dark。
    const mode = ref<ThemeMode>('dark')
    const accent = ref('#ffffff')

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
        const next = (mode.value === 'light' ? 'theme-light' : 'theme-dark') as
            | 'theme-dark'
            | 'theme-light'
        html.classList.add(next)
        html.style.colorScheme = next === 'theme-light' ? 'light' : 'dark'
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
        // dark-only: no-op listener, but keep the contract stable
    }

    return { mode, accent, init, applyTheme, setMode, setAccent }
})
