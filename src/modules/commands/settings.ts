// ============================================================
// 命令模块设置注册 (暂无设置项, 预留扩展)
// ============================================================

import { defineModuleSettings } from '@/modules/settings'

export const commandsSettings = defineModuleSettings({
  moduleId: 'commands',
  order: 20,
  groups: [],
})
