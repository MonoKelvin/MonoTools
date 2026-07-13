/**
 * vue-virtual-scroller 2.0.0-beta.8 类型声明 shim.
 *
 * 背景: 该库在 2.0.0-beta 阶段未提供 .d.ts (官方仍在迭代).
 * 我们只用到 `RecycleScroller` 一个组件, 这里手动声明它的 props / 事件,
 * 让 `vue-tsc` 能通过类型检查.
 *
 * 与 src/components/search/VirtualGroupedResults.vue 的实际用法保持一致:
 *   :items, :item-size, key-field, type-field, :buffer, :style
 *   + 默认 slot 拿到 { item, index, active }
 *   + 实例方法: scrollToItem(index), scrollToPosition(pixels)
 *
 * 升级到正式 2.x 时, 删除本文件并用官方 .d.ts 即可.
 */
declare module 'vue-virtual-scroller' {
  import type { DefineComponent } from 'vue'

  /**
   * RecycleScroller props —— 仅声明我们用到的字段, 其余按需扩展.
   * 与 README 中 "Props" 章节对齐.
   */
  export interface RecycleScrollerProps {
    /** 完整数据列表. */
    items: ReadonlyArray<any>
    /** 固定行高 (px). 设为 null 切换到 variable size mode. */
    itemSize?: number | null
    /** 唯一键字段名, 默认 'id'. */
    keyField?: string
    /** 类型字段, 区分不同 pool, 默认 'type'. */
    typeField?: string
    /** 视口外延的预渲染 buffer (px), 默认 200. */
    buffer?: number
    /** 列表外层 class. */
    listClass?: string
    /** 单项外层 class. */
    itemClass?: string
    /** 列表外层 tag, 默认 'div'. */
    listTag?: string
    /** 单项外层 tag, 默认 'div'. */
    itemTag?: string
  }

  /**
   * RecycleScroller 实例方法 (通过 ref 拿到).
   */
  export interface RecycleScrollerInstance {
    /** 滚动到指定 item 索引. */
    scrollToItem(index: number): void
    /** 滚动到指定像素位置. */
    scrollToPosition(position: number): void
  }

  /**
   * RecycleScroller 组件: Vue 3 用法, 接收 props 并用默认 slot 渲染每行.
   */
  export const RecycleScroller: DefineComponent<RecycleScrollerProps>
}
