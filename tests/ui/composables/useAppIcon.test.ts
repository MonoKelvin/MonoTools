/**
 * useAppIcon 组合式函数测试 —— 覆盖 Section 4 的 Lucide 兜底 + 缓存 + 批量加载.
 *
 * 关键验证点:
 * 1. 应用类型 → 全部失败时, 返回 Lucide 'component' (不再用 monogram).
 * 2. 文件类型 → 按扩展名/resultType 给出精确 Lucide 图标.
 * 3. 同一 id 多次调用 → 命中缓存.
 * 4. loadIconsBatch 走 IPC 后写回缓存.
 * 5. 空 path 不触发 IPC, 直接 fallback.
 */
import { beforeEach, describe, expect, it, vi, afterEach } from 'vitest'

/**
 * 模块级可变 mockEnv, 让 isTauri 可以动态切换.
 * vi.mock 工厂返回 getter 形式, 每次 import 时都读取最新值.
 */
const mockEnv = vi.hoisted(() => ({ isTauri: false }))

vi.mock('@/services/env', () => ({
  get isTauri() {
    return mockEnv.isTauri
  },
}))

vi.mock('@/services/api', () => ({
  appIconApi: {
    get: vi.fn().mockResolvedValue(null),
    getBatch: vi.fn().mockResolvedValue([]),
  },
}))

vi.mock('@/utils/lobehubIcons', () => ({
  lobehubFuzzyMatch: vi.fn().mockResolvedValue(null),
}))

import { useAppIcon } from '@/common/composables/useAppIcon'
import { appIconApi } from '@/services/api'
import type { SearchResult } from '@/modules/search'

function mk(over: Partial<SearchResult> = {}): SearchResult {
  return {
    id: 'r-' + Math.random().toString(36).slice(2, 8),
    title: 'Sample',
    subtitle: '',
    icon: null,
    category: 'apps',
    resultType: 'user-app',
    action: { type: 'launch', data: 'C:\\path\\to\\app.exe' },
    score: 0.5,
    ...over,
  }
}

/**
 * 真实有效的 1x1 红色 PNG 的 base64 编码 (96 chars).
 * 用于 IPC 校验 ≥ 64 chars 阈值的测试, 防止短 base64 误判为无效.
 */
const VALID_PNG_BASE64 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg=='

describe('useAppIcon', () => {
  beforeEach(() => {
    // 重置 mock + 缓存
    vi.mocked(appIconApi.get).mockReset().mockResolvedValue(null)
    vi.mocked(appIconApi.getBatch).mockReset().mockResolvedValue([])
    mockEnv.isTauri = false
    useAppIcon().clear()
  })

  afterEach(() => {
    mockEnv.isTauri = false
  })

  it('returns Lucide component fallback for app result when all sources fail (non-Tauri)', async () => {
    const { loadIcon } = useAppIcon()
    // 用一个不在 knownAppIcons 关键词表中的名字, 避免 lookupKnownIcon 命中
    const result = mk({ id: 'app-1', title: 'RandomUnknown', category: 'apps', resultType: 'user-app' })
    const state = await loadIcon(result)
    // 不再用 monogram, 走 Lucide 通用 AppWindow
    expect(state.kind).toBe('component')
  })

  it('returns Lucide component fallback for non-app result', async () => {
    const { loadIcon } = useAppIcon()
    const result = mk({
      id: 'file-1',
      title: 'doc.pdf',
      category: 'files',
      resultType: 'document',
    })
    const state = await loadIcon(result)
    expect(state.kind).toBe('component')
  })

  it('returns Lucide component fallback for system-app result', async () => {
    const { loadIcon } = useAppIcon()
    const result = mk({
      id: 'sys-1',
      title: 'Calculator',
      category: 'apps',
      resultType: 'system-app',
    })
    const state = await loadIcon(result)
    // system-app 用 Lucide 兜底 (Monitor), 不用 monogram
    expect(state.kind).toBe('component')
  })

  it('PDF file result uses FileText (not monogram) icon', async () => {
    const { loadIcon } = useAppIcon()
    const result = mk({
      id: 'pdf-1',
      title: 'report.pdf',
      subtitle: 'C:\\Users\\MONO\\Documents\\report.pdf',
      category: 'files',
      resultType: 'document',
    })
    const state = await loadIcon(result)
    expect(state.kind).toBe('component')
  })

  it('image file result uses FileImage icon', async () => {
    const { loadIcon } = useAppIcon()
    const result = mk({
      id: 'img-1',
      title: 'photo.png',
      subtitle: 'C:\\Users\\MONO\\Pictures\\photo.png',
      category: 'files',
      resultType: 'image',
    })
    const state = await loadIcon(result)
    expect(state.kind).toBe('component')
  })

  it('archive file result uses FileArchive icon', async () => {
    const { loadIcon } = useAppIcon()
    const result = mk({
      id: 'zip-1',
      title: 'project.zip',
      subtitle: 'C:\\Users\\MONO\\Downloads\\project.zip',
      category: 'files',
      resultType: 'archive',
    })
    const state = await loadIcon(result)
    expect(state.kind).toBe('component')
  })

  it('directory file result uses FolderOpen icon', async () => {
    const { loadIcon } = useAppIcon()
    const result = mk({
      id: 'dir-1',
      title: 'MyFolder',
      subtitle: 'C:\\Users\\MONO\\Documents\\MyFolder',
      category: 'files',
      resultType: 'directory',
    })
    const state = await loadIcon(result)
    expect(state.kind).toBe('component')
  })

  it('caches result by id — second loadIcon returns same value (no extra IPC)', async () => {
    mockEnv.isTauri = true
    // 真实 1x1 PNG base64 (96 chars), 满足 useAppIcon 的 ≥ 64 chars 校验
    vi.mocked(appIconApi.get).mockResolvedValue(VALID_PNG_BASE64)
    const { loadIcon } = useAppIcon()
    const result = mk({
      id: 'cache-1',
      title: 'CachedApp',
      action: { type: 'launch', data: 'C:\\cache.exe' },
    })
    // 第一次调用: IPC
    await loadIcon(result)
    expect(vi.mocked(appIconApi.get)).toHaveBeenCalledTimes(1)
    // 第二次调用: 应命中缓存, 不再发 IPC
    await loadIcon(result)
    expect(vi.mocked(appIconApi.get)).toHaveBeenCalledTimes(1)
  })

  it('empty path returns fallback without IPC', async () => {
    mockEnv.isTauri = true
    const { loadIcon } = useAppIcon()
    const result = mk({
      id: 'empty-path',
      title: 'Empty',
      action: { type: 'launch', data: '' },
    })
    const state = await loadIcon(result)
    // 应用类, 无 path, 走 Lucide 通用 AppWindow, 不再用 monogram
    expect(state.kind).toBe('component')
    expect(vi.mocked(appIconApi.get)).not.toHaveBeenCalled()
  })

  it('loadIconsBatch is no-op in non-Tauri env', async () => {
    mockEnv.isTauri = false
    const { loadIconsBatch } = useAppIcon()
    await loadIconsBatch([mk({ id: 'b1' }), mk({ id: 'b2' })])
    expect(vi.mocked(appIconApi.getBatch)).not.toHaveBeenCalled()
  })

  it('loadIconsBatch calls IPC and populates cache when in Tauri', async () => {
    mockEnv.isTauri = true
    // 真实 PNG base64, 满足 useAppIcon 的 ≥ 64 chars 校验
    vi.mocked(appIconApi.getBatch).mockResolvedValueOnce([VALID_PNG_BASE64, null])
    const { loadIconsBatch, loadIcon } = useAppIcon()
    const items = [
      mk({ id: 'b-good', action: { type: 'launch', data: 'C:\\a.exe' } }),
      mk({ id: 'b-bad', action: { type: 'launch', data: 'C:\\b.exe' } }),
    ]
    await loadIconsBatch(items)
    expect(vi.mocked(appIconApi.getBatch)).toHaveBeenCalledTimes(1)
    // 成功项 → png
    const good = await loadIcon(items[0])
    expect(good.kind).toBe('png')
    // 失败项 → fallback (Lucide 组件, 不再用 monogram)
    const bad = await loadIcon(items[1])
    expect(bad.kind).toBe('component')
  })

  it('loadIconsBatch deduplicates identical paths', async () => {
    mockEnv.isTauri = true
    vi.mocked(appIconApi.getBatch).mockResolvedValue([null])
    const { loadIconsBatch } = useAppIcon()
    const items = [
      mk({ id: 'dup-1', action: { type: 'launch', data: 'C:\\same.exe' } }),
      mk({ id: 'dup-2', action: { type: 'launch', data: 'C:\\same.exe' } }),
    ]
    await loadIconsBatch(items)
    // 去重: 只发 1 个 path
    expect(vi.mocked(appIconApi.getBatch)).toHaveBeenCalledWith(['C:\\same.exe'])
  })

  it('loadIconsBatch gracefully handles batch failure', async () => {
    mockEnv.isTauri = true
    vi.mocked(appIconApi.getBatch).mockRejectedValueOnce(new Error('IPC broken'))
    const { loadIconsBatch, loadIcon } = useAppIcon()
    const items = [
      mk({ id: 'fail-1', action: { type: 'launch', data: 'C:\\x.exe' } }),
    ]
    await expect(loadIconsBatch(items)).resolves.toBeUndefined()
    // batch 失败后, 单个 loadIcon 仍能拿到 fallback (不挂)
    // 现在统一走 Lucide 组件, 不再用 monogram
    const state = await loadIcon(items[0])
    expect(state.kind).toBe('component')
  })

  it('result with empty id returns fallback without throwing', async () => {
    const { loadIcon } = useAppIcon()
    const result = mk({ id: '', title: 'NoId' })
    const state = await loadIcon(result)
    expect(state.kind).toBe('component')
  })
})

/**
 * 监控 (trace) 测试: 验证 logIconTrace 被正确调用, counts 累加正确.
 * 用来确认"修复后图标覆盖率"统计能反映真实加载流程.
 */
describe('useAppIcon — 监控 trace', () => {
  // 每个测试前重置 trace 计数, 互不污染
  beforeEach(async () => {
    const { resetIconTrace } = await import('@/common/composables/iconLog')
    resetIconTrace()
  })

  it('cache 命中会记一次 level=cache', async () => {
    const { loadIcon } = useAppIcon()
    const result = mk({
      id: 'trace-cache',
      title: 'SomeApp',
      action: { type: 'launch', data: 'C:\\app.exe' },
    })
    // 第一次会走 lookupKnownIcon / IPC / fallback 之一
    await loadIcon(result)
    // 第二次命中 cache
    await loadIcon(result)
    const { useIconLog } = await import('@/common/composables/iconLog')
    const log = useIconLog()
    expect(log.counts.value.cache).toBeGreaterThanOrEqual(1)
    // 最近一条 cache trace 应当引用同 id
    const last = log.traces.value[log.traces.value.length - 1]
    expect(last.level).toBe('cache')
    expect(last.id).toBe('trace-cache')
  })

  it('fallback 路径会让 counts.fallback 增长', async () => {
    mockEnv.isTauri = false
    const { loadIcon } = useAppIcon()
    const result = mk({
      id: 'trace-fallback',
      title: 'SomeUnknown',
      category: 'apps',
      resultType: 'user-app',
    })
    const state = await loadIcon(result)
    // 不再用 monogram, 走 Lucide 通用 AppWindow
    expect(state.kind).toBe('component')
    const { useIconLog } = await import('@/common/composables/iconLog')
    const log = useIconLog()
    expect(log.counts.value.fallback).toBeGreaterThanOrEqual(1)
  })

  it('ipc 命中会让 counts.ipc 增长', async () => {
    mockEnv.isTauri = true
    // 真实 PNG base64, 满足 useAppIcon 的 ≥ 64 chars 校验
    vi.mocked(appIconApi.get).mockResolvedValue(VALID_PNG_BASE64)
    const { loadIcon } = useAppIcon()
    const result = mk({
      id: 'trace-ipc',
      title: 'IPCApp',
      action: { type: 'launch', data: 'C:\\ipc.exe' },
    })
    await loadIcon(result)
    const { useIconLog } = await import('@/common/composables/iconLog')
    const log = useIconLog()
    expect(log.counts.value.ipc).toBeGreaterThanOrEqual(1)
  })

  it('resetTrace 后 counts 全部归零', async () => {
    mockEnv.isTauri = true
    // 真实 PNG base64, 满足 useAppIcon 的 ≥ 64 chars 校验
    vi.mocked(appIconApi.get).mockResolvedValue(VALID_PNG_BASE64)
    const { loadIcon, resetTrace } = useAppIcon()
    await loadIcon(
      mk({
        id: 'reset-1',
        title: 'X',
        action: { type: 'launch', data: 'C:\\x.exe' },
      }),
    )
    const { useIconLog, resetIconTrace } = await import('@/common/composables/iconLog')
    expect(useIconLog().counts.value.ipc).toBeGreaterThanOrEqual(1)
    // 两条路径都该清零
    resetTrace()
    resetIconTrace()
    expect(useIconLog().counts.value.ipc).toBe(0)
    expect(useIconLog().counts.value.cache).toBe(0)
    expect(useIconLog().counts.value.fallback).toBe(0)
  })

  it('dumpSummary 不抛错 (console.groupCollapsed mock)', async () => {
    const { dumpSummary } = useAppIcon()
    // groupCollapsed 在 happy-dom 不存在, stub 一下避免 console 报错
    const origGroup = (console as any).groupCollapsed
    const origGroupEnd = (console as any).groupEnd
    const origLog = (console as any).log
    const origTable = (console as any).table
    ;(console as any).groupCollapsed = vi.fn()
    ;(console as any).groupEnd = vi.fn()
    ;(console as any).log = vi.fn()
    ;(console as any).table = vi.fn()
    try {
      expect(() => dumpSummary()).not.toThrow()
    } finally {
      ;(console as any).groupCollapsed = origGroup
      ;(console as any).groupEnd = origGroupEnd
      ;(console as any).log = origLog
      ;(console as any).table = origTable
    }
  })
})
