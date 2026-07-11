import { describe, expect, it, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useThemeStore } from '@/stores/theme'

describe('useThemeStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('默认 dark 模式与 #ffffff accent', () => {
    const t = useThemeStore()
    expect(t.mode).toBe('dark')
    expect(t.accent).toBe('#ffffff')
  })

  it('setMode 更新 mode & class', async () => {
    const api = await import('@/services/settingsApi')
    api.settingsApi.set = vi.fn(async () => undefined)
    const t = useThemeStore()
    await t.setMode('light')
    expect(t.mode).toBe('light')
    expect(api.settingsApi.set).toHaveBeenCalled()
    expect(document.documentElement.classList.contains('theme-light')).toBe(true)
  })

  it('setAccent 更新 CSS 变量', async () => {
    const api = await import('@/services/settingsApi')
    api.settingsApi.set = vi.fn(async () => undefined)
    const t = useThemeStore()
    await t.setAccent('#ff6161')
    expect(t.accent).toBe('#ff6161')
    expect(document.documentElement.style.getPropertyValue('--accent')).toBe('#ff6161')
  })
})
