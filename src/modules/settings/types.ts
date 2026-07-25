// ============================================================
// 设置项类型定义 — 纯框架层，不包含任何业务含义
// ============================================================

/** 设置项的值类型枚举 */
export type SettingValueType =
  | 'boolean'
  | 'string'
  | 'number'
  | 'select'
  | 'select-multi'
  | 'hotkey'
  | 'pathList'
  | 'info'

/** 设置项的值 */
export type SettingValue = boolean | string | number | string[] | null

/** 单个设置项的定义 — 由各模块声明 */
export interface SettingItemDef {
  /** 唯一 key，对应后端 settings 中的字段 (camelCase) */
  key: string
  /** 设置项标签 */
  label: string
  /** 设置项描述 (可选，显示在 label 下方) */
  description?: string
  /** 控件类型 */
  type: SettingValueType
  /** 默认值 */
  default: SettingValue
  /** 仅 type='select' / 'select-multi' 时有效 */
  options?: Array<{ label: string; value: string }>
  /** placeholder (type='string') */
  placeholder?: string
  /** 仅 type='number' 时有效 */
  min?: number
  max?: number
  /** 步进值 (默认 1) */
  step?: number
  /** type='info' 时的只读文本内容 */
  content?: string
  /** 可选: 自定义验证函数，返回 false 阻止保存 */
  validate?: (value: SettingValue) => boolean
  /** 设置变更后触发的业务回调 (在持久化成功后执行) */
  onChange?: (value: SettingValue) => void | Promise<void>
}

/** 设置分组定义 — 由各模块声明 */
export interface SettingGroupDef {
  /** 分组唯一 id */
  id: string
  /** 分组标题 (显示在卡片 header) */
  label: string
  /** 分组描述 (可选，显示在标题下方) */
  description?: string
  /** 分组图标 (lucide icon name 字符串) */
  icon?: string
  /** 排序权重 (越小越靠前) */
  order?: number
  /** 该分组下的设置项 */
  items: SettingItemDef[]
}

/** 模块设置注册集合 */
export interface ModuleSettingsDef {
  /** 模块标识 (用于排序和调试) */
  moduleId: string
  /** 排序权重 (越小越靠前) */
  order?: number
  /** 该模块贡献的设置分组 */
  groups: SettingGroupDef[]
}

/** 设置项当前值 (运行时) */
export interface SettingItemState {
  key: string
  value: SettingValue
  dirty: boolean
}
