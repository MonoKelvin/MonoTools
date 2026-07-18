import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { ThemeMode } from '@/core/types/settings'
import { settingsApi, themeApi } from '@/services/api'

const THEME_KEY = 'theme'
const FOLLOW_SYSTEM_KEY = 'followSystemTheme'
const POLL_INTERVAL = 2000

export const useThemeStore = defineStore('theme', () => {
    // Raycast 是 dark-only。保留 mode 字段以兼容设置存储，但 UI 永远渲染 dark。
    const mode = ref<ThemeMode>('dark')
    const accent = ref('#ffffff')
    const followSystem = ref(false)

    let pollTimer: ReturnType<typeof setInterval> | null = null

    async function init() {
        try {
            const saved = await settingsApi.get<{ mode: ThemeMode; accent: string; followSystemTheme?: boolean }>('appearance')
            if (saved) {
                mode.value = saved.mode
                accent.value = saved.accent
            }
        } catch {
            // 默认值
        }

        try {
            const follow = await settingsApi.get<boolean>(FOLLOW_SYSTEM_KEY)
            if (follow !== null && follow !== undefined) {
                followSystem.value = follow
            }
        } catch {
            // 默认 false
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
        // 手动切换时自动关闭跟随系统
        if (followSystem.value && next !== 'auto') {
            await setFollowSystem(false)
        }
        mode.value = next
        updateClass()
        await settingsApi.set('appearance', { mode: mode.value, accent: accent.value })
    }

    async function setAccent(color: string) {
        accent.value = color
        updateClass()
        await settingsApi.set('appearance', { mode: mode.value, accent: accent.value })
    }

    async function setFollowSystem(value: boolean) {
        followSystem.value = value
        try {
            await settingsApi.set(FOLLOW_SYSTEM_KEY, value)
        } catch {
            // 静默失败
        }
        if (value) {
            // 开启时立即同步一次
            await syncSystemTheme()
        }
    }

    async function syncSystemTheme() {
        try {
            const systemTheme = await themeApi.getSystemTheme()
            if (systemTheme === 'light' || systemTheme === 'dark') {
                const newMode = systemTheme === 'light' ? 'light' : 'dark'
                if (mode.value !== newMode) {
                    mode.value = newMode as ThemeMode
                    updateClass()
                    await settingsApi.set('appearance', { mode: mode.value, accent: accent.value })
                }
            }
        } catch {
            // 获取失败时忽略
        }
    }

    function listenSystemTheme() {
        // 清除旧的定时器
        if (pollTimer) {
            clearInterval(pollTimer)
            pollTimer = null
        }

        // 每 2 秒轮询一次系统主题
        pollTimer = setInterval(() => {
            if (followSystem.value) {
                syncSystemTheme()
            }
        }, POLL_INTERVAL)
    }

    return { mode, accent, followSystem, init, applyTheme, setMode, setAccent, setFollowSystem }
})
