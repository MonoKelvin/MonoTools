/**
 * IconSource barrel —— 统一导出.
 *
 * 外部仅需要:
 * ```ts
 * import { resolveIconByRegistry, type IconState } from '@/composables/iconSources'
 * ```
 */

export * from './types'
export * from './registry'
export { KnownIconSource, extractPath } from './known'
export { LobehubIconSource } from './lobehub'
export { IpcIconSource } from './ipc'
export { FallbackIconSource } from './fallback'
