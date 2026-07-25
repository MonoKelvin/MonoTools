// ============================================================
// 框架核心逻辑 — 加载/保存/变更/业务回调
// ============================================================

import { ref, reactive, onMounted, computed, onBeforeUnmount } from 'vue'
import { settingsApi } from '@/services'
import { settingsRegistry } from '../registry'
import type { SettingValue } from '../types'

export function useSettingsFramework() {
  const loading = ref(true)
  const saving = ref(false)
  const values = reactive<Record<string, SettingValue>>({})
  const dirtyKeys = ref(new Set<string>())

  /** debounce 保存句柄 */
  let saveTimer: ReturnType<typeof setTimeout> | null = null
  /** 待保存的值快照 */
  let pendingSave: Record<string, SettingValue> | null = null

  /** 所有需要展示的分组 */
  const groups = computed(() => settingsRegistry.getAllGroups())

  /** 从后端加载所有设置值 */
  async function loadAll() {
    loading.value = true
    try {
      const keys = settingsRegistry.getAllKeys()
      const result = await settingsApi.getBulk(keys)
      for (const key of keys) {
        const def = settingsRegistry.findItemDef(key)
        values[key] = (result?.[key] as SettingValue | undefined) ?? def?.item.default ?? null
      }
    } catch {
      // 降级: 使用默认值
      for (const key of settingsRegistry.getAllKeys()) {
        const def = settingsRegistry.findItemDef(key)
        values[key] = def?.item.default ?? null
      }
    } finally {
      loading.value = false
    }
  }

  /** debounce 保存 (200ms) */
  function scheduleSave(payload: Record<string, SettingValue>) {
    pendingSave = { ...payload }
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = setTimeout(async () => {
      if (!pendingSave) return
      try {
        await settingsApi.setBulk(pendingSave)
        dirtyKeys.value.clear()
      } catch (err) {
        console.error('[SettingsFramework] bulk save failed:', err)
        await loadAll()
      } finally {
        saving.value = false
        pendingSave = null
      }
    }, 200)
  }

  /** 单个设置项变更 */
  async function updateValue(key: string, value: SettingValue) {
    // 验证
    const def = settingsRegistry.findItemDef(key)
    if (def?.item.validate && !def.item.validate(value)) {
      console.warn(`[SettingsFramework] validation failed for "${key}"`)
      return
    }

    values[key] = value
    dirtyKeys.value.add(key)
    saving.value = true

    // debounce 持久化
    scheduleSave({ ...values })

    // 触发业务回调
    const onChange = def?.item.onChange
    if (onChange) {
      try {
        await onChange(value)
      } catch (err) {
        console.error(`[SettingsFramework] onChange failed for "${key}":`, err)
      }
    }
  }

  onMounted(() => loadAll())

  onBeforeUnmount(() => {
    if (saveTimer) {
      clearTimeout(saveTimer)
      if (pendingSave) {
        settingsApi.setBulk(pendingSave).catch(() => {})
      }
    }
  })

  return {
    loading,
    saving,
    values,
    groups,
    updateValue,
    reload: loadAll,
  }
}
