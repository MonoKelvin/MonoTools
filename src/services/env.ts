// 简单的 Tauri API 探测：检测是否在 Tauri 环境下运行
// __TAURI__ 是 Tauri 2.3+ 的推荐检测方式
export const isTauri = !!(window as any).__TAURI__

// 因 import.meta.env 可能与 Vite 兼容性问题，写一个安全的封装
const env = (import.meta as any).env || {}
export const isDev = env.DEV || env.NODE_ENV === 'development'
