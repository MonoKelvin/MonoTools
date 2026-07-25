// ============================================================
// 设置注册表 — 全局单例，收集所有模块注册的设置
// ============================================================

import type { ModuleSettingsDef, SettingGroupDef, SettingItemDef } from './types'

class SettingsRegistry {
  private modules: ModuleSettingsDef[] = []

  /** 注册一个模块的设置 */
  register(def: ModuleSettingsDef): void {
    // 去重：同一 moduleId 的重复注册会覆盖旧的
    const existing = this.modules.findIndex(m => m.moduleId === def.moduleId)
    if (existing >= 0) {
      this.modules[existing] = def
    } else {
      this.modules.push(def)
    }
  }

  /** 获取所有已注册的分组（按 order 排序，同 order 按注册顺序） */
  getAllGroups(): SettingGroupDef[] {
    return this.modules
      .sort((a, b) => (a.order ?? 0) - (b.order ?? 0))
      .flatMap(m => m.groups)
  }

  /** 获取所有设置项的 key 列表 (用于批量取值) */
  getAllKeys(): string[] {
    return this.getAllGroups().flatMap(g => g.items.map(i => i.key))
  }

  /** 获取所有模块的注册信息 */
  getAllModules(): ModuleSettingsDef[] {
    return [...this.modules]
  }

  /** 按 key 查找设置项定义 */
  findItemDef(key: string): { group: SettingGroupDef; item: SettingItemDef } | null {
    for (const group of this.getAllGroups()) {
      const item = group.items.find(i => i.key === key)
      if (item) return { group, item }
    }
    return null
  }

  /** 清除所有注册（主要用于测试） */
  clear(): void {
    this.modules = []
  }
}

/** 全局单例 */
export const settingsRegistry = new SettingsRegistry()

/**
 * 模块用此函数声明自己的设置 (在 settings.ts 中调用)。
 *
 * @example
 * ```ts
 * // src/modules/search/settings.ts
 * import { defineModuleSettings } from '@/modules/settings'
 *
 * export const searchSettings = defineModuleSettings({
 *   moduleId: 'search',
 *   groups: [
 *     {
 *       id: 'file-search',
 *       label: '文件搜索',
 *       icon: 'FolderSearch',
 *       items: [
 *         { key: 'fileSearchEnabled', type: 'boolean', label: '启用文件搜索', default: true },
 *         { key: 'fileSearchRoots', type: 'pathList', label: '搜索目录', default: [] },
 *       ]
 *     }
 *   ]
 * })
 * ```
 */
export function defineModuleSettings(def: ModuleSettingsDef): ModuleSettingsDef {
  settingsRegistry.register(def)
  return def
}
