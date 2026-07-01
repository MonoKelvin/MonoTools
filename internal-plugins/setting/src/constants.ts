// 默认配置常量
export const DEFAULT_PLACEHOLDER = '搜索应用和指令 / 粘贴文件或图片'
export const DEFAULT_AVATAR = '/logo.png'

// 自动粘贴选项类型
export type AutoPasteOption = 'off' | '1s' | '3s' | '5s' | '10s'

// 自动清空选项类型
export type AutoClearOption = 'immediately' | '1m' | '2m' | '3m' | '5m' | '10m' | 'never'

// 自动返回搜索选项类型
export type AutoBackToSearchOption = 'immediately' | '30s' | '1m' | '3m' | '5m' | '10m' | 'never'

// 主题类型
export type ThemeType = 'system' | 'light' | 'dark'

// 主题色类型
export type PrimaryColor = 'blue' | 'purple' | 'green' | 'orange' | 'red' | 'pink' | 'custom'

// 超级面板鼠标按键类型
export type MouseButtonType = 'middle' | 'right' | 'back' | 'forward'
