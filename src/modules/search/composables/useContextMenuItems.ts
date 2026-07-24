/**
 * 右键菜单项目提供器 —— 按 group kind 动态生成菜单配置.
 *
 * 采用 Provider 设计模式：每个分组 (pinned / recent / apps / files / commands)
 * 独立配置自己的菜单规则，右键菜单展开时由 SearchPage 按需动态生成菜单项目.
 *
 * ContextMenu.vue 只负责渲染，无任何业务逻辑.
 * 新增分组或调整菜单规则只需修改此文件.
 *
 * 设计原则：
 * 1. ContextMenu 不感知 isPinned、isBuiltin 等业务状态，这些由父级传入
 * 2. 每个分组可自定义是否需要 pin/delete/execute 等特殊操作
 * 3. 菜单 key 约定：open=打开, execute=执行命令, pin=固定, pin-off=取消固定, delete=删除
 * 4. 系统命令不可编辑/删除，用户命令可以
 */

import type { MtMenuItem } from '@/ui/components/MtMenu.vue'
import type { SearchResult } from '@/modules/search'
import {
    Play,
    CornerDownLeft,
    FolderOpen,
    Copy,
    FileText,
    Pin,
    PinOff,
    Info,
    Trash2,
    Settings2,
} from '@lucide/vue'
import type { Component } from 'vue'

// ============================================================================
// 类型定义
// ============================================================================

/** 菜单项扩展字段（用于传递业务状态） */
export interface ContextMenuAction {
    /** 菜单项 key，用于事件回传 */
    key?: string
    /** 显示标签 */
    label?: string
    /** 图标组件 */
    icon?: Component
    /** 快捷键提示 */
    shortcut?: string
    /** 危险操作（红色高亮） */
    danger?: boolean
    /** 分隔线 */
    divider?: boolean
}

/**
 * 菜单规则定义.
 * - items: 菜单项列表，按顺序展示
 * - extraActions: 分组级别扩展操作（如命令编辑/删除）
 * - shouldShow: 条件显隐函数（可选）
 */
export interface ContextMenuRule {
    /** 菜单项列表 */
    items: ContextMenuAction[]
    /** 扩展操作（仅在条件满足时追加到菜单末尾） */
    extraActions?: ContextMenuAction[]
}

/**
 * 右键菜单上下文：由父级在调用 buildContextMenuItems 时传入.
 * 包含所有业务状态，Provider 不自行判断.
 */
export interface ContextMenuContext {
    /** 当前右键点击的搜索结果项 */
    item: SearchResult
    /** 分组 kind: 'pinned' | 'recent' | 'apps' | 'files' | 'commands' */
    kind: string
    /** 该项是否已被固定 */
    isPinned: boolean
    /** 是否为内置命令（仅 commands 分组使用） */
    isBuiltin: boolean
}

// ============================================================================
// 菜单规则定义
// ============================================================================

/**
 * 固定项目 / 最近访问菜单项.
 * 不需要固定到首页，不需要删除.
 */
const PINNED_RECENT_ITEMS: ContextMenuAction[] = [
    { key: 'open', label: '打开', icon: Play, shortcut: 'Enter' },
    { key: 'open-location', label: '打开文件所在路径', icon: FolderOpen, shortcut: 'Ctrl+Enter' },
    { key: 'copy-path', label: '复制文件路径', icon: Copy, shortcut: 'Ctrl+C' },
    { key: 'copy-dir', label: '复制目录路径', icon: Copy, shortcut: 'Ctrl+Shift+C' },
    { key: 'copy-name', label: '复制名称', icon: FileText },
    { key: 'properties', label: '属性', icon: Info, shortcut: 'Alt+Enter' },
]

/**
 * 应用程序菜单项.
 * 不需要删除，可以固定到首页.
 */
const APP_ITEMS: ContextMenuAction[] = [
    { key: 'open', label: '打开', icon: Play, shortcut: 'Enter' },
    { key: 'open-location', label: '打开文件所在路径', icon: FolderOpen, shortcut: 'Ctrl+Enter' },
    { key: 'copy-path', label: '复制文件路径', icon: Copy, shortcut: 'Ctrl+C' },
    { key: 'copy-name', label: '复制名称', icon: FileText },
    { key: 'properties', label: '属性', icon: Info, shortcut: 'Alt+Enter' },
]

/**
 * 所有文件菜单项.
 * 不需要固定到首页，需要删除.
 */
const FILE_ITEMS: ContextMenuAction[] = [
    { key: 'open', label: '打开', icon: Play, shortcut: 'Enter' },
    { key: 'open-location', label: '打开文件所在路径', icon: FolderOpen, shortcut: 'Ctrl+Enter' },
    { key: 'copy-path', label: '复制文件路径', icon: Copy, shortcut: 'Ctrl+C' },
    { key: 'copy-dir', label: '复制目录路径', icon: Copy, shortcut: 'Ctrl+Shift+C' },
    { key: 'copy-name', label: '复制名称', icon: FileText },
    { key: 'properties', label: '属性', icon: Info, shortcut: 'Alt+Enter' },
    { key: 'delete', label: '删除', icon: Trash2, shortcut: 'Delete', danger: true },
]

/**
 * 命令菜单项.
 * "打开" 换成 "执行".
 */
const COMMAND_ITEMS: ContextMenuAction[] = [
    { key: 'execute', label: '执行', icon: CornerDownLeft, shortcut: 'Enter' },
    { key: 'copy-name', label: '复制命令名称', icon: FileText },
]

/**
 * 用户命令扩展操作（编辑/删除）.
 * 仅对用户自定义命令展示，系统命令不展示.
 */
const COMMAND_EXTRA_ACTIONS: ContextMenuAction[] = [
    { key: 'edit-command', label: '编辑命令', icon: Settings2, shortcut: 'F2' },
    { key: 'delete-command', label: '删除命令', icon: Trash2, danger: true },
]

// ============================================================================
// 分组级菜单规则表
// ============================================================================

/**
 * 每个分组独立的菜单规则.
 * 新增分组只需在此表添加配置即可.
 */
export const CONTEXT_MENU_RULES: Record<string, ContextMenuRule> = {
    // 固定项目: 无 pin 无 delete
    pinned: { items: PINNED_RECENT_ITEMS },

    // 最近访问: 无 pin 无 delete
    recent: { items: PINNED_RECENT_ITEMS },

    // 应用程序: 有 pin 无 delete
    apps: { items: APP_ITEMS },

    // 所有文件: 无 pin 有 delete
    files: { items: FILE_ITEMS },

    // 命令: execute 代替 open，用户命令可编辑/删除
    commands: {
        items: COMMAND_ITEMS,
        extraActions: COMMAND_EXTRA_ACTIONS,
    },
}

// ============================================================================
// 菜单构建逻辑
// ============================================================================

/**
 * 根据分组 kind 获取对应的菜单规则.
 */
function getMenuRule(kind: string): ContextMenuRule | undefined {
    return CONTEXT_MENU_RULES[kind]
}

/**
 * 将 ContextMenuAction 转换为 MtMenuItem.
 */
function actionToMenuItem(action: ContextMenuAction): MtMenuItem {
    return {
        key: action.key,
        label: action.label,
        icon: action.icon as unknown as Component,
        shortcut: action.shortcut,
        danger: action.danger,
        divider: action.divider,
    }
}

/**
 * 构建右键菜单 MtMenuItem 列表.
 *
 * 流程：
 * 1. 根据 kind 获取分组菜单规则
 * 2. 将 ContextMenuAction 转换为 MtMenuItem
 * 3. 过滤 pin/pin-off 互斥显示（由 isPinned 决定）
 * 4. 追加 extraActions（由 isBuiltin 决定）
 *
 * @param ctx 包含所有业务状态的上下文
 * @returns 预计算好的 MtMenuItem[]，可直接传给 ContextMenu 组件
 */
export function buildContextMenuItems(ctx: ContextMenuContext): MtMenuItem[] {
    const { item, kind, isPinned, isBuiltin } = ctx
    const rule = getMenuRule(kind)
    if (!rule) return []

    const items: MtMenuItem[] = []

    // 转换菜单项
    for (const action of rule.items) {
        // pin/pin-off 互斥显示
        if (action.key === 'pin' && isPinned) continue
        if (action.key === 'pin-off' && !isPinned) continue

        items.push(actionToMenuItem(action))
    }

    // 命令分组：系统命令不追加编辑/删除
    if (rule.extraActions && !isBuiltin) {
        for (const action of rule.extraActions) {
            items.push(actionToMenuItem(action))
        }
    }

    return items
}
