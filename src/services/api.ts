import { call } from './tauri'
import type { SearchResult, SearchOptions } from '@/types/search'
import type { Settings, ThemeMode } from '@/types/settings'

export const searchApi = {
  search(query: string, options?: Partial<SearchOptions>): Promise<SearchResult[]> {
    return call<SearchResult[]>('search', { query, options })
  },
  execute(item: SearchResult): Promise<void> {
    return call<void>('execute_search_result', { item })
  },
}

export const startupApi = {
  list() {
    return call<unknown[]>('list_startup_items', {})
  },
  toggle(id: string, enabled: boolean) {
    return call<void>('toggle_startup_item', { id, enabled })
  },
  add(item: unknown) {
    return call<string>('add_startup_item', { item })
  },
  remove(id: string) {
    return call<void>('remove_startup_item', { id })
  },
  update(item: unknown) {
    return call<void>('update_startup_item', { item })
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
    return call<void>('show_search_window', {})
  },
  hide() {
    return call<void>('hide_search_window', {})
  },
  toggle() {
    return call<void>('toggle_search_window', {})
  },
}
