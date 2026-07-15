import type { Component } from 'vue'

/**
 * 通用状态栏消息类型.
 *
 * ActionBar 作为通用展示组件, 只识别这些**语义类型**, 不感知任何业务
 * (索引 / 选中 / 盘符 / 搜索结果 等). 业务侧负责把具体状态翻译成下列
 * 类型之一后传入.
 *
 * - info:    普通信息 (灰色图标)
 * - success: 成功     (亮色对勾)
 * - warning: 警告     (琥珀三角)
 * - error:   错误     (红色圆圈)
 * - loading: 加载中   (金色 spinner + 动态省略号)
 * - debug:   调试     (弱化 bug 图标)
 */
export type StatusBarType =
  | 'info'
  | 'success'
  | 'warning'
  | 'error'
  | 'loading'
  | 'debug'

/**
 * 文本片段强调种类. 与 {@link StatusBarType} 正交:
 * type 决定整体图标 / 态, kind 决定单段文字的视觉权重.
 */
export type StatusSegmentKind = 'muted' | 'label' | 'primary' | 'number' | 'accent'

/**
 * 状态栏文本片段. 外部按语义切分后传入, ActionBar 按 kind 染色.
 */
export interface StatusSegment {
  text: string
  kind?: StatusSegmentKind
}

/**
 * ActionBar 展示单元. 外部构建后作为 `message` prop 传入,
 * ActionBar 只负责渲染 (图标 + 片段 + 过渡 + 加载省略号).
 *
 * - id:       过渡 key, 变化即触发模糊渐变动画. loading 态建议用
 *             粗粒度 id (如按卷序号), 避免计数刷新反复触发过渡.
 * - type:     决定默认图标 + 是否显示加载省略号 + 整体色调.
 * - segments: 结构化文本, 按 kind 染色. 空数组时仅展示图标.
 * - icon?:    覆盖默认 type 图标, 用于自定义控件场景. 不传则按 type 取默认.
 */
export interface StatusBarMessage {
  id: string | number
  type: StatusBarType
  segments: StatusSegment[]
  icon?: Component
}
