import { describe, expect, it, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useSettingsStore } from '@/stores/settings'

function switchApiMock(impl: Record<string, any>): void {
  // 直接重新 import 模块来替换 `settingsApi` —— 因为 Pinia store 在 mount 时
  // 已经 bound 到了 fn，本测试只验证 store 层 patch 状态是否正确传递。
  ;(require('@/services') as any) // eslint-disable-line
}

describe('settings store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('使用默认值初始化', () => {
    const s = useSettingsStore()
    expect(s.settings.hotkey).toBe('Alt+Space')
    expect(s.settings.theme).toBe('dark')
    expect(s.settings.pinToTop).toBe(true)
    expect(s.settings.fileSearchEnabled).toBe(true)
    expect(s.settings.enabledCategories).toEqual(['apps', 'files', 'commands'])
  })

  it('update 合并到现有 settings', async () => {
    const s = useSettingsStore()
    // 避免真实 invoke：暂时 override settingsApi.update
    const api = await import('@/services/settingsApi')
    api.settingsApi.setAll = vi.fn(async () => undefined)

    await s.update({ hotkey: 'Ctrl+Shift+K' })
    expect(s.settings.hotkey).toBe('Ctrl+Shift+K')
    expect(s.settings.theme).toBe('dark')
    expect(api.settingsApi.setAll).toHaveBeenCalled()
  })
})
