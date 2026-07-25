// ============================================================
// 设置模块框架 — 公共导出
// ============================================================

export * from './types'
export { settingsRegistry, defineModuleSettings } from './registry'

// ★ 集中 import 所有模块的 settings — 确保全部注册
// 新增模块时，在此加一行
import '@/core/settings/defaults'
import '@/modules/search/settings'
import '@/modules/commands/settings'
