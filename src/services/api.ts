import { call } from './tauri'
import type { SearchResult, SearchOptions } from '@/modules/search'
import type { Settings, ThemeMode } from '@/core/types/settings'

export const searchApi = {
    /**
     * 执行搜索.
     * @param query 用户输入的查询字符串 (空字符串 = 列出全部).
     * @param options 扩展选项:
     *   - `limit`: 覆盖默认上限, 用于"显示更多" / loadMore 增量加载.
     *   - `categories`: 限制只在指定 category 中搜索 (apps / files / commands).
     *   - `includeHidden`: 是否包含隐藏文件.
     */
    search(
        query: string,
        options?: Partial<SearchOptions>,
    ): Promise<SearchResult[]> {
        return call<SearchResult[]>('search_cmd', { query, options })
    },
    /**
     * 分页搜索: 从 after_id 之后继续取 limit 条, 给"显示更多"按钮用.
     * 避免一次性 search 拉过多结果导致 IPC 序列化阻塞.
     */
    searchMore(
        query: string,
        afterId: number,
        options?: Partial<SearchOptions>,
    ): Promise<SearchResult[]> {
        return call<SearchResult[]>('search_more_cmd', { query, after_id: afterId, options })
    },
    execute(item: SearchResult): Promise<void> {
        return call<void>('execute_result', { item })
    },
    buildIndex(): Promise<string> {
        return call<string>('build_file_index', {})
    },
    getIndexStatus(): Promise<{ files: number; apps: number; commands: number }> {
        return call<{ files: number; apps: number; commands: number }>('get_index_status', {})
    },
}

/**
 * 应用图标 API —— 从可执行文件 (.exe / .lnk) 提取 32x32 PNG 真实图标.
 *
 * 错误处理协议 (与后端 `get_app_icon` IPC 对齐):
 * - 返回 `string | null`: null 表示提取失败, 前端降级到 Lucide 通用图标.
 * - 内部 IPC 错误 (网络 / Tauri 未注入) 同样返回 null, 永不抛错.
 */
export const appIconApi = {
  /**
   * 获取可执行文件图标的 base64 PNG.
   * @param path `.exe` / `.lnk` / `.bat` 等 Windows 可执行路径.
   */
  get(path: string): Promise<string | null> {
    return call<string | null>('get_app_icon', { path }).catch(() => null)
  },
  /**
   * 批量获取图标. 一次 IPC 拉多个 base64 PNG, 减少 RTT 开销.
   * @param paths 可执行文件路径数组 (会去重, 后端自动 cache).
   * @returns 顺序与 paths 一一对应的 base64 字符串数组, 失败位置为 null.
   */
  getBatch(paths: string[]): Promise<Array<string | null>> {
    return call<Array<string | null>>('get_app_icons_batch', { paths }).catch(
      () => paths.map(() => null),
    )
  },
}

export const commandApi = {
    list() {
        return call<unknown[]>('list_commands', {})
    },
    run(id: string) {
        return call<void>('run_command', { id })
    },
    add(cmd: unknown) {
        return call<string>('add_command', { cmd })
    },
    remove(id: string) {
        return call<void>('remove_command', { id })
    },
}

export const settingsApi = {
    get<T>(key: string) {
        return call<T | null>('get_setting', { key })
    },
    set<T>(key: string, value: T) {
        return call<void>('set_setting', { key, value })
    },
    getAll() {
        return call<Settings | null>('get_all_settings', {})
    },
    setAll(value: Settings) {
        return call<void>('set_all_settings', { value })
    },
}

export const hotkeyApi = {
    register(hotkey: string) {
        return call<void>('register_hotkey', { hotkey })
    },
    unregister() {
        return call<void>('unregister_hotkey', {})
    },
    current() {
        return call<string | null>('get_current_hotkey', {})
    },
}

export const themeApi = {
    get() {
        return call<{ mode: ThemeMode; accent: string } | null>('get_appearance', {})
    },
    set(appearance: { mode: ThemeMode; accent: string }) {
        return call<void>('set_appearance', { appearance })
    },
    /** 获取 Windows 系统当前主题 ("light" 或 "dark") */
    getSystemTheme() {
        return call<string>('get_system_theme', {})
    },
    /** 设置是否跟随系统主题 */
    setFollowSystemTheme(follow: boolean) {
        return call<void>('set_follow_system_theme', { value: follow })
    },
}

export const windowApi = {
    show() {
        return call<void>('show_window', {})
    },
    hide() {
        return call<void>('hide_window', {})
    },
    toggle() {
        return call<void>('toggle_window', {})
    },
    setHeight(height: number) {
        return call<void>('set_window_height', { height })
    },
}

export const pinTopApi = {
    get() {
        return call<boolean>('get_pin_top', {})
    },
    set(value: boolean) {
        return call<void>('set_pin_top', { value })
    },
}

export const shellApi = {
    open(path: string) {
        return call<void>('execute_result', { item: { action: { Open: path } } })
    },
    /** 在文件管理器中打开并选中指定文件 */
    openFileLocation(path: string) {
        return call<void>('open_file_location', { path })
    },
    /** 显示文件属性对话框 */
    showProperties(path: string) {
        return call<void>('show_file_properties', { path })
    },
    /** 删除文件到回收站 */
    deleteToRecycleBin(path: string) {
        return call<void>('delete_file_to_recycle_bin', { path })
    },
}

export const commandSpecsApi = {
    /** 列出后端已注册命令的所有 spec（id + description + aliases + usage） */
    list(): Promise<Array<{ name: string; description?: string; aliases?: string[]; usage?: string }>> {
        return call<Array<{ name: string; description?: string; aliases?: string[]; usage?: string }>>(
            'list_command_specs',
            {},
        )
    },
    /** 按 id 路由到后端命令执行；args 是字符串数组 */
    dispatch(commandId: string, args?: string[]): Promise<{ success: boolean; message: string; data?: unknown }> {
        return call<{ success: boolean; message: string; data?: unknown }>(
            'dispatch_command',
            { commandId, args: args ?? [] },
        )
    },
}

/**
 * 窗口监控 API —— 跟踪系统当前激活应用, 用于基于上下文的智能推荐.
 *
 * 数据流向:
 * - 每 2 秒轮询当前前台窗口, 防抖 3 次确认切换.
 * - 切换时 emit `window_changed` 事件; 前端 store 监听以更新推荐算法.
 * - 此 API 提供"一次性拉取当前快照"的能力, 给冷启动场景使用.
 *
 * 错误处理协议:
 * - 所有方法在 IPC 失败时 throw, 由调用方 (search store) 处理.
 * - listenChanged 在非 Tauri 环境返回 noop unlisten 函数.
 */
export const windowMonitorApi = {
    /**
     * 拉取窗口监控当前快照: 当前激活应用 + 最近 10 个历史.
     */
    getState(): Promise<{
        activeAppPath: string
        activeAppTitle: string
        recentApps: Array<{ path: string; title: string }>
    }> {
        return call<{
            activeAppPath: string
            activeAppTitle: string
            recentApps: Array<{ path: string; title: string }>
        }>('get_window_monitor_state', {})
    },
    /**
     * 后端 `window_changed` 事件订阅.
     * payload: { path: string; title: string; recent_count: number }
     */
    async listenChanged(
        handler: (payload: { path: string; title: string; recent_count: number }) => void,
    ): Promise<() => void> {
        return listenEvent<{ path: string; title: string; recent_count: number }>(
            'window_changed',
            handler,
        )
    },
}

/**
 * 固定项目 API —— 用户手动 pin 到首页的应用/文件.
 *
 * 错误处理协议:
 * - 所有方法在 IPC 失败时**静默**返回 null / 抛错, 由调用方 (search store) 处理.
 * - 持久化由后端 SQLite 完成 (`pin_repo`), 重启不丢失.
 */
export const pinApi = {
    /** 列出全部已 pin 的 id 列表 (按用户添加顺序). */
    list(): Promise<string[]> {
        return call<string[]>('list_pinned', {})
    },
    /** 添加一个 id 到 pin 列表. */
    add(id: string): Promise<void> {
        return call<void>('pin_item', { id })
    },
    /** 从 pin 列表移除一个 id. */
    remove(id: string): Promise<void> {
        return call<void>('unpin_item', { id })
    },
}
