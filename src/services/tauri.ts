import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { isTauri } from './env'

/**
 * 统一封装 Tauri 后端调用。
 * 在开发期的浏览器（非 Tauri 环境）下，提供 mock 数据，便于纯前端调试。
 */

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    return mockBackend<T>(cmd, args)
  }
  try {
    return await invoke<T>(cmd, args)
  } catch (err) {
    console.error(`invoke(${cmd}) failed:`, err)
    throw err
  }
}

export async function listenEvent<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  if (!isTauri) {
    return () => {
      /* noop */
    }
  }
  return listen<T>(event, (e) => handler(e.payload))
}

const store = new Map<string, unknown>()
function mockGet<T>(key: string): T | undefined {
  return store.get(key) as T | undefined
}
function mockSet<T>(key: string, value: T): void {
  store.set(key, value)
}

async function mockBackend<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  await new Promise((r) => setTimeout(r, 12))
  switch (cmd) {
    case 'search':
      return mockSearch(args?.query as string) as T
    case 'get_setting':
      return mockGet(args?.key as string) as T
    case 'set_setting':
      mockSet(args?.key as string, args?.value)
      return true as T
    case 'list_commands':
      return [] as T
    case 'show_search_window':
      return true as T
    case 'hide_search_window':
      return true as T
    case 'register_hotkey':
      return true as T
    default:
      return ([] as unknown) as T
  }
}

function mockSearch(query: string) {
  const lower = query.toLowerCase()
  const sample = [
    { name: 'Google Chrome', subtitle: 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe', category: 'apps' },
    { name: 'VS Code', subtitle: 'C:\\Users\\...\\Code.exe', category: 'apps' },
    { name: 'Spotify', subtitle: 'C:\\Users\\...\\Spotify.exe', category: 'apps' },
    { name: 'README.md', subtitle: 'E:\\work\\code\\MTools\\README.md', category: 'files' },
    { name: 'git commit', subtitle: 'Shell Command', category: 'commands' },
  ]
  return sample
    .filter((s) => !query.trim() || s.name.toLowerCase().includes(lower))
    .map((s, idx) => ({
      id: String(idx),
      title: s.name,
      subtitle: s.subtitle,
      icon: null,
      category: s.category,
      action: { type: 'launch', data: s.subtitle },
      score: 1 - idx * 0.1,
    }))
}
