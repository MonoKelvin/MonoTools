// shim for Vue SFC type-checking
declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<{}, {}, any>
  export default component
  // 允许从 .vue 文件中导入 TypeScript 接口
  // 例如: import type { MtMenuItem } from './MtMenu.vue'
}

// 允许导入 .vue 文件中导出的具体类型
declare module '@/ui/components/MtMenu.vue' {
  import type { Component } from 'vue'
  export interface MtMenuItem {
    key?: string
    label?: string
    icon?: Component
    shortcut?: string
    danger?: boolean
    disabled?: boolean
    divider?: boolean
    custom?: string
  }
}
