import { call } from './tauri'
import type { SearchResult, SearchOptions } from '@/types/search'
import type { Settings, ThemeMode } from '@/types/settings'

export const searchApi = {
    search(query: string, options?: Partial<SearchOptions>): Promise<SearchResult[]> {
        return call<SearchResult[]>('search_cmd', { query, options })
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
    fileSearch(query: string, limit?: number): Promise<SearchResult[]> {
        return call<SearchResult[]>('file_search', { query, limit })
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
}
