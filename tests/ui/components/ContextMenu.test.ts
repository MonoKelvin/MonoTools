/**
 * ContextMenu 右键菜单测试 —— 覆盖 5 项关键行为:
 *   1. itemPath 兜底逻辑
 *   2. 打开位置空路径提示
 *   3. 属性空路径提示
 *   4. 删除空路径提示
 *   5. 正常路径操作调用正确后端 API
 *
 * 注意: mock 对象使用 vi.hoisted 创建，避免 Cannot access before initialization.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { defineComponent, h, nextTick } from 'vue'
import ContextMenu from '@/modules/search/components/ContextMenu.vue'

// === Stub 组件 ===

const StubAppResultItem = defineComponent({
  name: 'AppResultItem',
  props: ['result', 'active', 'index'],
  setup(props: any) {
    return () => h('div', { class: 'stub-app' }, props.result?.title)
  },
})

const StubEmptyState = defineComponent({
  name: 'MtEmptyState',
  props: ['icon', 'title', 'hint', 'padding'],
  setup() {
    return () => h('div', { class: 'stub-empty' })
  },
})

// === Mock 对象（vi.hoisted 避免初始化顺序问题） ===

const { shellApiMock, searchStoreMock, addMessageMock, useStatusMessagesFactory } = vi.hoisted(() => {
  const shellApiMock: any = {
    openFileLocation: vi.fn().mockResolvedValue(undefined),
    showProperties: vi.fn().mockResolvedValue(undefined),
    deleteToRecycleBin: vi.fn().mockResolvedValue(undefined),
  }

  const searchStoreMock: any = {
    executeItem: vi.fn().mockResolvedValue(undefined),
    isPinned: vi.fn().mockReturnValue(false),
    runSearch: vi.fn().mockResolvedValue(undefined),
  }

  const addMessageMock = vi.fn()

  const useStatusMessagesFactory = () => ({
    addMessage: addMessageMock,
  })

  return {
    shellApiMock,
    searchStoreMock,
    addMessageMock,
    useStatusMessagesFactory,
  }
})

// === Mock 注册 ===

vi.mock('@/services/api', () => ({
  shellApi: shellApiMock,
}))

vi.mock('@/modules/search/composables/useStatusMessages', () => ({
  useStatusMessages: useStatusMessagesFactory,
}))

vi.mock('@/modules/search', () => ({
  useSearchStore: () => searchStoreMock,
}))

// === 测试数据工厂 ===

type ActionType = 'launch' | 'open' | 'run' | 'navigate'

interface SearchResultOver {
  action?: { type: ActionType; data?: any }
  subtitle?: string
  title?: string
}

function mkResult(over: SearchResultOver = {}): any {
  return {
    id: 'ctx-' + Math.random().toString(36).slice(2, 8),
    title: 'notepad.exe',
    subtitle: 'C:\\Windows\\System32\\notepad.exe',
    meta: null,
    icon: null,
    category: 'apps',
    resultType: 'user-app',
    action: { type: 'launch', data: 'C:\\Windows\\System32\\notepad.exe' },
    score: 1,
    ...over,
  }
}

// === 公共 setup ===

beforeEach(() => {
  addMessageMock.mockClear()

  shellApiMock.openFileLocation.mockClear()
  shellApiMock.openFileLocation.mockResolvedValue(undefined)
  shellApiMock.showProperties.mockClear()
  shellApiMock.showProperties.mockResolvedValue(undefined)
  shellApiMock.deleteToRecycleBin.mockClear()
  shellApiMock.deleteToRecycleBin.mockResolvedValue(undefined)

  searchStoreMock.executeItem.mockClear()
  searchStoreMock.executeItem.mockResolvedValue(undefined)
  searchStoreMock.isPinned.mockClear()
  searchStoreMock.isPinned.mockReturnValue(false)
  searchStoreMock.runSearch.mockClear()
  searchStoreMock.runSearch.mockResolvedValue(undefined)

  vi.spyOn(window, 'confirm').mockReturnValue(true)
})

// === itemPath 兜底逻辑 ===

describe('ContextMenu - itemPath 兜底逻辑', () => {
  it('open/launch 且 action.data 是字符串 → 返回 action.data', async () => {
    const item = mkResult({ action: { type: 'open', data: 'C:\\test.exe' } })
    const wrapper = mount(ContextMenu, {
      props: { visible: false, x: 0, y: 0, item },
      global: {
        stubs: {
          AppResultItem: StubAppResultItem,
          MtEmptyState: StubEmptyState,
        },
      },
    })
    await nextTick()
    const itemPath = (wrapper.vm as any).itemPath
    expect(itemPath).toBe('C:\\test.exe')
  })

  it('open 且 action.data 非字符串 → 回退 subtitle', async () => {
    const item = mkResult({
      action: { type: 'open', data: 123 },
      subtitle: 'C:\\fallback\\app.exe',
    })
    const wrapper = mount(ContextMenu, {
      props: { visible: false, x: 0, y: 0, item },
      global: {
        stubs: {
          AppResultItem: StubAppResultItem,
          MtEmptyState: StubEmptyState,
        },
      },
    })
    await nextTick()
    const itemPath = (wrapper.vm as any).itemPath
    expect(itemPath).toBe('C:\\fallback\\app.exe')
  })

  it('run 类型 → 返回 subtitle', async () => {
    const item = mkResult({
      action: { type: 'run', data: { command: 'cmd', args: [] } },
      subtitle: 'cmd.exe',
    })
    const wrapper = mount(ContextMenu, {
      props: { visible: false, x: 0, y: 0, item },
      global: {
        stubs: {
          AppResultItem: StubAppResultItem,
          MtEmptyState: StubEmptyState,
        },
      },
    })
    await nextTick()
    const itemPath = (wrapper.vm as any).itemPath
    expect(itemPath).toBe('cmd.exe')
  })

  it('未知 action 类型 → 回退 subtitle/title', async () => {
    const item = mkResult({
      action: { type: 'navigate', data: '/settings' },
      subtitle: '',
      title: 'Settings',
    })
    const wrapper = mount(ContextMenu, {
      props: { visible: false, x: 0, y: 0, item },
      global: {
        stubs: {
          AppResultItem: StubAppResultItem,
          MtEmptyState: StubEmptyState,
        },
      },
    })
    await nextTick()
    const itemPath = (wrapper.vm as any).itemPath
    expect(itemPath).toBe('Settings')
  })
})

// === 空路径提示 ===

describe('ContextMenu - 空路径提示', () => {
  it('打开位置: path 为空时提示错误并关闭菜单', async () => {
    const item = mkResult({ subtitle: '', title: '', action: { type: 'open', data: '' } })
    const wrapper = mount(ContextMenu, {
      props: { visible: true, x: 10, y: 10, item },
      global: {
        stubs: {
          AppResultItem: StubAppResultItem,
          MtEmptyState: StubEmptyState,
        },
      },
    })
    await nextTick()
    const vm = wrapper.vm as any
    vm.handleOpenLocation()
    await nextTick()
    expect(addMessageMock).toHaveBeenCalledWith(
      expect.objectContaining({ text: '无法打开位置：当前项路径为空', type: 'error' }),
    )
    expect(shellApiMock.openFileLocation).not.toHaveBeenCalled()
  })

  it('属性: path 为空时提示错误并关闭菜单', async () => {
    const item = mkResult({ subtitle: '', title: '', action: { type: 'open', data: '' } })
    const wrapper = mount(ContextMenu, {
      props: { visible: true, x: 10, y: 10, item },
      global: {
        stubs: {
          AppResultItem: StubAppResultItem,
          MtEmptyState: StubEmptyState,
        },
      },
    })
    await nextTick()
    const vm = wrapper.vm as any
    vm.handleProperties()
    await nextTick()
    expect(addMessageMock).toHaveBeenCalledWith(
      expect.objectContaining({ text: '无法打开属性：当前项路径为空', type: 'error' }),
    )
    expect(shellApiMock.showProperties).not.toHaveBeenCalled()
  })

  it('删除: path 为空时提示错误并关闭菜单', async () => {
    const item = mkResult({ subtitle: '', title: '', action: { type: 'open', data: '' } })
    const wrapper = mount(ContextMenu, {
      props: { visible: true, x: 10, y: 10, item },
      global: {
        stubs: {
          AppResultItem: StubAppResultItem,
          MtEmptyState: StubEmptyState,
        },
      },
    })
    await nextTick()
    const vm = wrapper.vm as any
    await vm.handleDeleteConfirm()
    await nextTick()
    expect(addMessageMock).toHaveBeenCalledWith(
      expect.objectContaining({ text: '无法删除：当前项路径为空', type: 'error' }),
    )
    expect(window.confirm).not.toHaveBeenCalled()
    expect(shellApiMock.deleteToRecycleBin).not.toHaveBeenCalled()
  })
})

// === 正常路径操作调用 ===

describe('ContextMenu - 正常路径操作调用后端', () => {
  it('打开位置: path 非空时调用 shellApi.openFileLocation', async () => {
    const item = mkResult({ subtitle: 'C:\\Windows\\System32\\notepad.exe' })
    const wrapper = mount(ContextMenu, {
      props: { visible: true, x: 10, y: 10, item },
      global: {
        stubs: {
          AppResultItem: StubAppResultItem,
          MtEmptyState: StubEmptyState,
        },
      },
    })
    await nextTick()
    const vm = wrapper.vm as any
    vm.handleOpenLocation()
    await nextTick()
    expect(shellApiMock.openFileLocation).toHaveBeenCalledWith('C:\\Windows\\System32\\notepad.exe')
  })

  it('属性: path 非空时调用 shellApi.showProperties', async () => {
    const item = mkResult({ subtitle: 'C:\\Windows\\System32\\notepad.exe' })
    const wrapper = mount(ContextMenu, {
      props: { visible: true, x: 10, y: 10, item },
      global: {
        stubs: {
          AppResultItem: StubAppResultItem,
          MtEmptyState: StubEmptyState,
        },
      },
    })
    await nextTick()
    const vm = wrapper.vm as any
    vm.handleProperties()
    await nextTick()
    expect(shellApiMock.showProperties).toHaveBeenCalledWith('C:\\Windows\\System32\\notepad.exe')
  })

  it('删除: path 非空且用户确认后调用 shellApi.deleteToRecycleBin', async () => {
    const item = mkResult({ subtitle: 'C:\\Windows\\System32\\notepad.exe' })
    const wrapper = mount(ContextMenu, {
      props: { visible: true, x: 10, y: 10, item },
      global: {
        stubs: {
          AppResultItem: StubAppResultItem,
          MtEmptyState: StubEmptyState,
        },
      },
    })
    await nextTick()
    const vm = wrapper.vm as any
    vi.spyOn(window, 'confirm').mockReturnValueOnce(true)
    await vm.handleDeleteConfirm()
    await nextTick()
    expect(shellApiMock.deleteToRecycleBin).toHaveBeenCalledWith('C:\\Windows\\System32\\notepad.exe')
    expect(searchStoreMock.runSearch).toHaveBeenCalled()
  })
})
