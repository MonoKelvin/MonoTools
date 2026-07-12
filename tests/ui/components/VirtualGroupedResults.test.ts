/**
 * VirtualGroupedResults 组件测试 —— 覆盖 Section 2 的折叠/展开 + 显示管道逻辑.
 *
 * 测试策略:
 * - 浅渲染 (不挂载 PrimeVue / 全局 directives), 仅测 props → emits + 状态.
 * - mock 掉 fileKinds / icons / store 等不相关依赖.
 * - 验证关键不变量:
 *   1. visibleItems 与 displayList 数量一致.
 *   2. 点击 toggle 触发 'toggle-group' emit, 父级调用 store.toggleGroupCollapse 后 store 状态更新.
 *   3. 折叠后 displayList 排除该组.
 *   4. row container 渲染了正确数量的行.
 */
import { beforeEach, describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { defineComponent, h, nextTick } from 'vue'
import { useSearchStore, type DisplayGroup, GROUP_ID } from '@/stores/search'
import type { SearchResult } from '@/types/search'

// Stub 子组件: AppResultItem / ResultItem / CheckButton 不需要在测试中真实渲染
const StubItem = defineComponent({
  name: 'StubItem',
  props: ['result', 'active', 'index'],
  setup(props: any) {
    return () =>
      h(
        'div',
        { class: 'stub-item', 'data-id': props.result?.id, 'data-active': props.active ? '1' : '0' },
        props.result?.title,
      )
  },
})

const CheckButtonStub = defineComponent({
  name: 'CheckButton',
  props: ['modelValue', 'size'],
  setup(props: any) {
    return () => h('div', { class: 'cb-stub', 'data-checked': props.modelValue ? '1' : '0' })
  },
})

// 用动态 import 拿 VGR, 避免顶层 await 问题
import VGR from '@/components/search/VirtualGroupedResults.vue'

function mkResult(over: Partial<SearchResult> = {}): SearchResult {
  return {
    id: 'r-' + Math.random().toString(36).slice(2, 8),
    title: 'item',
    subtitle: '',
    icon: null,
    category: 'apps',
    resultType: 'system-app',
    action: { type: 'launch', data: 'x' },
    score: 0.5,
    ...over,
  }
}

function makeGroup(over: Partial<DisplayGroup> = {}): DisplayGroup {
  return {
    id: GROUP_ID.system,
    title: 'Test',
    items: [mkResult({ id: '1' }), mkResult({ id: '2' })],
    visibleItems: [mkResult({ id: '1' }), mkResult({ id: '2' })],
    collapsed: false,
    kind: 'system',
    ...over,
  }
}

/**
 * 测试用父组件: 把 'toggle-group' 事件桥接到 store.toggleGroupCollapse,
 * 模拟 SearchPage 的真实行为.
 */
const Parent = defineComponent({
  name: 'Parent',
  components: { VGR },
  props: {
    groups: { type: Array, required: true },
    selectedIndex: { type: Number, default: 0 },
    hasQuery: { type: Boolean, default: true },
    query: { type: String, default: 'x' },
  },
  setup(props, { emit }) {
    const search = useSearchStore()
    const onToggle = (id: any) => {
      search.toggleGroupCollapse(id)
    }
    const onSelect = (item: SearchResult) => {
      emit('select', item)
    }
    return () =>
      h(VGR, {
        groups: props.groups as any,
        loading: false,
        selectedIndex: props.selectedIndex,
        height: 400,
        hasQuery: props.hasQuery,
        query: props.query,
        'onSelect': onSelect,
        'onOpen': () => {},
        'onHover': () => {},
        'onContextmenu': () => {},
        'onToggle-group': onToggle,
        'onShow-more-files': () => {},
      })
  },
})

describe('VirtualGroupedResults', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('renders rows for each visible item across groups', async () => {
    const search = useSearchStore()
    search.collapsedGroups = new Set() // 全部展开
    const groups: DisplayGroup[] = [
      makeGroup({
        id: GROUP_ID.system,
        title: '系统应用',
        items: [mkResult({ id: 's1' }), mkResult({ id: 's2' }), mkResult({ id: 's3' })],
        visibleItems: [mkResult({ id: 's1' }), mkResult({ id: 's2' }), mkResult({ id: 's3' })],
        kind: 'system',
      }),
      makeGroup({
        id: GROUP_ID.commands,
        title: '命令',
        items: [mkResult({ id: 'c1' }), mkResult({ id: 'c2' })],
        visibleItems: [mkResult({ id: 'c1' }), mkResult({ id: 'c2' })],
        kind: 'commands',
      }),
    ]
    const wrapper = mount(VGR, {
      props: { groups, loading: false, selectedIndex: 0, height: 400, hasQuery: true, query: 'x' },
      global: {
        stubs: {
          AppResultItem: StubItem,
          ResultItem: StubItem,
          CheckButton: CheckButtonStub,
        },
        mocks: { $t: (k: string) => k },
      },
    })
    await nextTick()
    const rows = wrapper.findAll('.vg__row')
    // 3 + 2 = 5 行
    expect(rows.length).toBe(5)
  })

  it('clicking the toggle emits toggle-group and parent updates store', async () => {
    const search = useSearchStore()
    expect(search.collapsedGroups.has(GROUP_ID.system)).toBe(false)
    const groups: DisplayGroup[] = [
      makeGroup({
        id: GROUP_ID.system,
        title: '系统应用',
        kind: 'system',
        items: [mkResult({ id: 's1' }), mkResult({ id: 's2' })],
        visibleItems: [mkResult({ id: 's1' }), mkResult({ id: 's2' })],
      }),
    ]
    const wrapper = mount(Parent, {
      props: { groups, selectedIndex: 0, hasQuery: true, query: 'x' },
      global: {
        stubs: {
          AppResultItem: StubItem,
          ResultItem: StubItem,
          CheckButton: CheckButtonStub,
        },
        mocks: { $t: (k: string) => k },
      },
    })
    await nextTick()
    const toggle = wrapper.find('.vg__group-toggle')
    expect(toggle.exists()).toBe(true)
    await toggle.trigger('click')
    await nextTick()
    // 通过 Parent 桥接, store 状态应已折叠该组
    expect(search.collapsedGroups.has(GROUP_ID.system)).toBe(true)
  })

  it('collapsed group hides its rows via v-show but keeps group header', async () => {
    const search = useSearchStore()
    // 让 system 组折叠但 files 组展开 → flatItems 非空, 渲染 scroller
    const groups: DisplayGroup[] = [
      makeGroup({
        id: GROUP_ID.system,
        title: '系统应用',
        collapsed: true,
        items: [mkResult({ id: 's1' }), mkResult({ id: 's2' })],
        visibleItems: [], // 折叠时 visibleItems = []
        kind: 'system',
      }),
      makeGroup({
        id: GROUP_ID.files,
        title: '所有文件',
        items: [mkResult({ id: 'f1' })],
        visibleItems: [mkResult({ id: 'f1' })],
        kind: 'files',
      }),
    ]
    const wrapper = mount(VGR, {
      props: { groups, loading: false, selectedIndex: 0, height: 400, hasQuery: true, query: 'x' },
      global: {
        stubs: {
          AppResultItem: StubItem,
          ResultItem: StubItem,
          CheckButton: CheckButtonStub,
        },
        mocks: { $t: (k: string) => k },
      },
    })
    await nextTick()
    // 折叠的系统组: 0 行
    // 展开的文件组: 1 行
    // 但 visibleGroups 过滤掉了 items=[] && collapsed=true... 等等
    // 让我们看看: items.length > 0 || collapsed = true (因为 collapsed)
    // 所以系统组还在, 但 visibleItems=[] → 0 行
    // 文件组: items.length=1 > 0 → 1 行
    const rows = wrapper.findAll('.vg__row')
    // 注意: VGR 把 v-show 元素保留在 DOM 中, 只是 display: none.
    // 所以 .vg__row 仍能找到 0+1 = 1 个 (折叠组的 0 行 + 展开组的 1 行)
    // 但 visibleItems=[] 时, v-for 不渲染任何行
    expect(rows.length).toBe(1)
    // 分组 header 仍存在
    const groupSections = wrapper.findAll('.vg__group')
    expect(groupSections.length).toBe(2)
  })

  it('passes selectedIndex to active row class', async () => {
    const groups: DisplayGroup[] = [
      makeGroup({
        id: GROUP_ID.system,
        title: '系统应用',
        items: [mkResult({ id: 'a' }), mkResult({ id: 'b' })],
        visibleItems: [mkResult({ id: 'a' }), mkResult({ id: 'b' })],
        kind: 'system',
      }),
    ]
    const wrapper = mount(VGR, {
      props: { groups, loading: false, selectedIndex: 1, height: 400, hasQuery: true, query: 'x' },
      global: {
        stubs: {
          AppResultItem: StubItem,
          ResultItem: StubItem,
          CheckButton: CheckButtonStub,
        },
        mocks: { $t: (k: string) => k },
      },
    })
    await nextTick()
    const activeRows = wrapper.findAll('.vg__row--active')
    expect(activeRows.length).toBe(1)
  })

  it('click on row emits select with the item', async () => {
    const groups: DisplayGroup[] = [
      makeGroup({
        id: GROUP_ID.system,
        title: '系统应用',
        items: [mkResult({ id: 'a' }), mkResult({ id: 'b' })],
        visibleItems: [mkResult({ id: 'a' }), mkResult({ id: 'b' })],
        kind: 'system',
      }),
    ]
    const wrapper = mount(Parent, {
      props: { groups, selectedIndex: 0, hasQuery: true, query: 'x' },
      global: {
        stubs: {
          AppResultItem: StubItem,
          ResultItem: StubItem,
          CheckButton: CheckButtonStub,
        },
        mocks: { $t: (k: string) => k },
      },
    })
    await nextTick()
    const firstRow = wrapper.findAll('.vg__row')[0]
    await firstRow.trigger('click')
    expect(wrapper.emitted('select')).toBeTruthy()
  })
})

/**
 * 回归测试: 启动时 store 还没产出 DisplayGroup, 父组件传 undefined 给 groups
 * 也不能让 setup 里的 watch / computed 抛 "Cannot read properties of undefined".
 * 这是导致启动白屏 (整页空白, 只有 "Uncaught (in promise) TypeError" 一条)
 * 的根因. 组件必须能在 groups=undefined 下正常挂载.
 */
describe('VirtualGroupedResults — undefined groups 防御', () => {
  it('groups=undefined 仍能挂载而不抛错 (启动防白屏)', async () => {
    const wrapper = mount(VGR, {
      props: {
        groups: undefined as unknown as DisplayGroup[],
        loading: true,
        selectedIndex: 0,
        height: 400,
        hasQuery: false,
        query: '',
      },
      global: {
        stubs: {
          AppResultItem: StubItem,
          ResultItem: StubItem,
          CheckButton: CheckButtonStub,
        },
        mocks: { $t: (k: string) => k },
      },
    })
    await nextTick()
    // 没抛错 + 渲染了空容器即可
    expect(wrapper.exists()).toBe(true)
    // 即使没有数据, 组件也应有一个可滚动的容器 (避免 DOM 结构塌陷)
    expect(wrapper.find('.vg__scroller').exists()).toBe(true)
  })

  it('groups 从 undefined 切到 [] 再切到 [1 个分组] 不抛错', async () => {
    const wrapper = mount(VGR, {
      props: {
        groups: undefined as unknown as DisplayGroup[],
        loading: false,
        selectedIndex: 0,
        height: 400,
        hasQuery: true,
        query: 'x',
      },
      global: {
        stubs: {
          AppResultItem: StubItem,
          ResultItem: StubItem,
          CheckButton: CheckButtonStub,
        },
        mocks: { $t: (k: string) => k },
      },
    })
    await nextTick()
    // 1) undefined → []
    await wrapper.setProps({ groups: [] as any })
    await nextTick()
    // 2) [] → 1 个分组
    await wrapper.setProps({
      groups: [
        makeGroup({
          id: GROUP_ID.system,
          title: '系统应用',
          items: [mkResult({ id: 'a' })],
          visibleItems: [mkResult({ id: 'a' })],
          kind: 'system',
        }),
      ] as any,
    })
    await nextTick()
    // 此时应渲染出 1 行
    expect(wrapper.findAll('.vg__row').length).toBe(1)
  })
})
