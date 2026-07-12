import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useCommandsStore } from '@/commands/store'

/**
 * 手动 mock `@/services/tauri`。
 *
 * 为什么不用 `vi.spyOn(tauriSvc, 'call')`：
 *   `store.ts` 的 `invokeCmd` 闭包通过 `import * as tauriSvc` 读取 `tauriSvc.call`。
 *   vi.mock 的 hoisting 行为确保 store 模块加载时，`call` 已经被替换成我们的 mock。
 *
 * 关键：vi.hoisted() 在 vi.mock hoisting 之前运行，所以可以在这里安全地创建 mock fn。
 */
const { mockCall, setOverride, clearOverride } = vi.hoisted(() => {
  let handler: ((cmd: string, args?: unknown) => unknown) | null = null

  const fn = vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
    if (handler) return handler(cmd, args)
    switch (cmd) {
      case 'list_command_specs':
        return [
          { name: 'search', description: '搜索', aliases: ['s', 'find'], usage: 'search <query>' },
          { name: 'launch', description: '启动', aliases: ['run', 'open-app'], usage: 'launch <name>' },
          { name: 'open', description: '打开', aliases: [], usage: 'open <path>' },
          { name: 'config', description: '配置', aliases: ['cfg', 'setting'], usage: 'config' },
          { name: 'help', description: '帮助', aliases: ['-h', '--help'], usage: 'help' },
          { name: 'version', description: '版本', aliases: ['-v', '--version'], usage: 'version' },
          { name: 'index', description: '索引', aliases: ['idx'], usage: 'index <sub>' },
          { name: 'stats', description: '统计', aliases: [], usage: 'stats [type]' },
          { name: 'command', description: '自定义命令', aliases: ['cmd'], usage: 'command <sub>' },
        ]
      case 'dispatch_command':
        return { success: true, message: 'mock', data: undefined }
      default:
        return null
    }
  })

  return {
    mockCall: fn,
    setOverride: (h: (cmd: string, args?: unknown) => unknown) => { handler = h },
    clearOverride: () => { handler = null },
  }
})

vi.mock('@/services/tauri', () => ({
  call: mockCall,
  listenEvent: vi.fn(async () => () => undefined),
}))

function asMock() {
  return mockCall
}

beforeEach(() => {
  setActivePinia(createPinia())
  mockCall.mockClear()
  clearOverride()
  ;(globalThis as any).window = (globalThis as any).window ?? {}
  ;(globalThis as any).window.__TAURI__ = {}
})

afterEach(() => {
  mockCall.mockClear()
  clearOverride()
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
  })

  it('拉取后端成功 → store 列表填满', async () => {
    setOverride(async () => [
      { name: 'search', description: '搜索', aliases: ['s', 'find'], usage: 'search <query>' },
      { name: 'config', description: '配置', aliases: ['cfg'] },
    ])
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
    setOverride(async () => { throw new Error('IPC 失败') })
    const s = useCommandsStore()
    await s.loadFromBackend()
    expect(s.isLoaded).toBe(true)
    expect(s.specs.length).toBeGreaterThan(0)
    expect((s.lastError ?? '').includes('IPC 失败')).toBe(true)
  })

  it('force=true 时强制重新拉取', async () => {
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
  it('后端 success=false 触发 onError', async () => {
    setOverride(async () => ({ success: false, message: '失败' }))
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
    setOverride(async () => { throw new Error('网络层错误') })
    const s = useCommandsStore()
    const errs: Array<[unknown, string]> = []
    s.onError((e, id) => errs.push([e, id]))
    const out = await s.execute('launch')
    expect(out).toBeUndefined()
    expect((errs[0]?.[0] as Error)?.message).toContain('网络层错误')
  })

  it('onError 解绑后不再触发', async () => {
    setOverride(async () => ({ success: false, message: 'x' }))
    const s = useCommandsStore()
    let count = 0
    const off = s.onError(() => count++)
    off()
    await s.execute('config')
    expect(count).toBe(0)
  })

  it('成功的执行不进 onError', async () => {
    setOverride(async () => ({ success: true, message: 'OK' }))
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
  })

  it('execute 路径走 invoke 并走到后端', async () => {
    setOverride(async () => ({ success: true, message: 'OK' }))
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
    setOverride(async () => [
      { name: 'search', description: '搜索', aliases: ['find'] },
      { name: 'config', description: '配置', aliases: ['cfg'] },
      { name: 'help', description: '帮助', aliases: [] },
    ])
  })

  it('get 直接 by id / by alias', async () => {
    const s = useCommandsStore()
    await s.loadFromBackend()
    expect(s.get('search')?.id).toBe('search')
    expect(s.get('find')?.id).toBe('search')
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
