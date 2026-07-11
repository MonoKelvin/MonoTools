import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useCommandsStore } from '@/commands/store'
import * as tauriSvc from '@/services/tauri'

vi.mock('@/services/tauri', async () => {
  const actual = await vi.importActual<any>('@/services/tauri')
  return {
    ...actual,
    listenEvent: vi.fn(async () => () => undefined),
  }
})

const callSpy = vi.spyOn(tauriSvc, 'call')
function asMock() {
  return callSpy
}

function rewireBackends(impls: {
  list_command_specs: any
  dispatch_command: any
}) {
  callSpy.mockImplementation(async (cmd: string) => {
    if (cmd === 'list_command_specs') return impls.list_command_specs
    if (cmd === 'dispatch_command') return impls.dispatch_command
    throw new Error(`unexpected cmd ${cmd}`)
  })
}

beforeEach(() => {
  setActivePinia(createPinia())
  callSpy.mockReset()
  ;(globalThis as any).window = (globalThis as any).window ?? {}
  ;(globalThis as any).window.__TAURI__ = {}
})

afterEach(() => {
  callSpy.mockReset()
  delete (globalThis as any).window.__TAURI__
})

describe('useCommandsStore — loadFromBackend', () => {
  it('没有 Tauri 环境时不请求后端', async () => {
    delete (globalThis as any).window.__TAURI__
    const s = useCommandsStore()
    await s.loadFromBackend()
    expect(s.isLoaded).toBe(true)
    // 不调用后端，但 specs 自动 fallback 到 UI 内置
    expect(s.specs.length).toBeGreaterThan(0)
    expect(asMock()).not.toHaveBeenCalled()
    ;(globalThis as any).window = (globalThis as any).window ?? {}
    ;(globalThis as any).window.__TAURI__ = {}
  })

  it('拉取后端成功 → store 列表填满', async () => {
    rewireBackends({
      list_command_specs: [
        { name: 'search', description: '搜索', aliases: ['s', 'find'], usage: 'search <query>' },
        { name: 'config', description: '配置', aliases: ['cfg'] },
      ],
      dispatch_command: { success: true, message: 'ok' },
    })
    const s = useCommandsStore()
    await s.loadFromBackend()
    expect(s.isLoaded).toBe(true)
    expect(s.lastError).toBeNull()
    const ids = s.specs.map((x) => x.id)
    expect(ids).toContain('search')
    expect(ids).toContain('config')
    expect(ids).toContain('search.cmd.execute-selected')
    const search = s.get('search')
    expect(search?.title).toBe('搜索')
  })

  it('拉取失败 → fallback builtin + 记录 lastError', async () => {
    ;(globalThis as any).window.__TAURI__ = {}
    callSpy.mockImplementation(async () => {
      throw new Error('IPC 失败')
    })
    const s = useCommandsStore()
    await s.loadFromBackend()
    expect(s.isLoaded).toBe(true)
    expect(s.specs.length).toBeGreaterThan(0)
    expect((s.lastError ?? '').includes('IPC 失败')).toBe(true)
  })

  it('force=true 时强制重新拉取', async () => {
    ;(globalThis as any).window.__TAURI__ = {}
    rewireBackends({ list_command_specs: [], dispatch_command: { success: true } })
    const s = useCommandsStore()
    await s.loadFromBackend()
    expect(asMock().mock.calls.filter((c) => c[0] === 'list_command_specs').length).toBe(1)
    await s.loadFromBackend()
    expect(asMock().mock.calls.filter((c) => c[0] === 'list_command_specs').length).toBe(1)
    await s.loadFromBackend(true)
    expect(asMock().mock.calls.filter((c) => c[0] === 'list_command_specs').length).toBe(2)
  })
})

describe('useCommandsStore — execute / errors', () => {
  function mockFallback(impl: (...args: unknown[]) => unknown) {
    callSpy.mockImplementation((cmd: string, args?: unknown) => {
      if (cmd === 'list_command_specs') return []
      return impl(cmd, args)
    })
  }

  it('后端 success=false 触发 onError', async () => {
    mockFallback(() => ({ success: false, message: '失败' }))
    const s = useCommandsStore()
    const errs: Array<[unknown, string]> = []
    s.onError((e, id) => errs.push([e, id]))
    const out = await s.execute('config')
    expect(!!out && out.success).toBe(false)
    expect(errs.length).toBeGreaterThanOrEqual(1)
    expect((errs[0][0] as Error).message).toContain('失败')
    expect(errs[0][1]).toBe('config')
  })

  it('execute promise 抛错时同样进 onError', async () => {
    mockFallback(() => {
      throw new Error('网络层错误')
    })
    const s = useCommandsStore()
    const errs: Array<[unknown, string]> = []
    s.onError((e, id) => errs.push([e, id]))
    const out = await s.execute('launch')
    expect(out).toBeUndefined()
    expect((errs[0]?.[0] as Error)?.message).toContain('网络层错误')
  })

  it('onError 解绑后不再触发', async () => {
    mockFallback(() => ({ success: false, message: 'x' }))
    const s = useCommandsStore()
    let count = 0
    const off = s.onError(() => count++)
    off()
    await s.execute('config')
    expect(count).toBe(0)
  })

  it('成功的执行不进 onError', async () => {
    mockFallback(() => ({ success: true, message: 'OK' }))
    const s = useCommandsStore()
    const errs: unknown[] = []
    s.onError((e) => errs.push(e))
    const out = await s.execute('config')
    expect(!!out && out.success).toBe(true)
    expect(errs.length).toBe(0)
  })

  it('非 Tauri 环境下 execute 返回 mock 成功', async () => {
    delete (globalThis as any).window.__TAURI__
    const s = useCommandsStore()
    const out = await s.execute('launch')
    expect(!!out && out.success).toBe(true)
    ;(globalThis as any).window.__TAURI__ = {}
  })

  it('execute 路径走 invoke 并走到后端', async () => {
    ;(globalThis as any).window.__TAURI__ = {}
    mockFallback(() => ({ success: true, message: 'OK' }))
    const s = useCommandsStore()
    const before = asMock().mock.calls.length
    const out = await s.execute('config')
    expect(!!out && out.success).toBe(true)
    expect(asMock().mock.calls.length).toBe(before + 1)
    expect(asMock().mock.calls[before][0]).toBe('dispatch_command')
  })
})

describe('useCommandsStore — query helpers', () => {
  beforeEach(() => {
    rewireBackends({
      list_command_specs: [
        { name: 'search', description: '搜索', aliases: ['find'] },
        { name: 'config', description: '配置', aliases: ['cfg'] },
        { name: 'help', description: '帮助', aliases: [] },
      ],
      dispatch_command: { success: true },
    })
  })

  it('get 直接 by id / by alias', async () => {
    const s = useCommandsStore()
    await s.loadFromBackend()
    expect(s.get('search')?.id).toBe('search')
    expect(s.get('find')?.id).toBe('search')
    expect(s.get('config')?.id).toBe('config')
    expect(s.get('cfg')?.id).toBe('config')
    expect(s.get('missing')).toBeUndefined()
  })

  it('list 包含后端 3 条 + UI 内置 specs 补充', async () => {
    const s = useCommandsStore()
    await s.loadFromBackend()
    const ids = s.list().map((x) => x.id)
    expect(ids).toContain('search')
    expect(ids).toContain('config')
    expect(ids).toContain('help')
    expect(ids).toContain('search.cmd.execute-selected')
    expect(ids).toContain('app.cmd.navigate.settings')
  })

  it('listByCategory 只包含匹配分类', async () => {
    const s = useCommandsStore()
    await s.loadFromBackend()
    const searchCat = s.listByCategory('search')
    expect(searchCat.length).toBeGreaterThan(0)
    expect(searchCat.every((x) => x.category === 'search')).toBe(true)
  })

  it('reset 把 state 清空并标记未 load', async () => {
    const s = useCommandsStore()
    await s.loadFromBackend()
    s.reset()
    expect(s.isLoaded).toBe(false)
    expect(s.specs).toEqual([])
    expect(s.lastError).toBeNull()
  })

  it('override 直接覆盖 specs（mock 路径）', () => {
    const s = useCommandsStore()
    s.override([
      { id: 'a', title: 'A', category: 'system' },
      { id: 'b', title: 'B', category: 'app' },
    ])
    expect(s.isLoaded).toBe(true)
    expect(s.list().map((s) => s.id).sort()).toEqual(['a', 'b'])
  })

  it('categories 收集仅存在的分类', async () => {
    const s = useCommandsStore()
    await s.loadFromBackend()
    const cats = new Set(s.categories)
    expect(cats.has('search')).toBe(true)
    expect(cats.has('app')).toBe(true)
    expect(cats.has('window')).toBe(true)
  })
})
