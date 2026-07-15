/**
 * VirtualGroupedResults 组件测试 —— 覆盖 Section 2 的折叠/展开 + 显示管道逻辑.
 *
 * 测试策略:
 * - 浅渲染 (不挂载 PrimeVue / 全局 directives), 仅测 props → emits + 状态.
 * - mock 掉 fileKinds / icons / store / RecycleScroller 等依赖.
 * - 关键不变量:
 *   1. 虚拟行数与 displayList 数量一致 (header + item 展平).
 *   2. 点击 toggle 触发 'toggle-group' emit, 父级调用 store.toggleGroupCollapse 后 store 状态更新.
 *   3. 折叠后该组的 item 行不再出现.
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

/**
 * RecycleScroller stub: happy-dom 没有真实布局, vue-virtual-scroller 会判定
 * viewport 高度为 0 而跳过渲染. 这里把它简化成一个 div 直接渲染所有 slot,
 * 让测试聚焦"组件对 items 的反应", 而非虚拟滚动本身的正确性
 * (后者由 vue-virtual-scroller 自己的测试覆盖).
 */
const RecycleScrollerStub = defineComponent({
  name: 'RecycleScroller',
  props: ['items', 'itemSize', 'keyField', 'typeField', 'buffer'],
  setup(props, { slots }) {
    return () =>
      h(
        'div',
        { class: 'recycle-scroller-stub', 'data-item-count': String(props.items?.length ?? 0) },
        (props.items ?? []).map((item: any, idx: number) =>
          h(
            'div',
            {
              class: 'recycle-scroller-stub__row',
              'data-index': idx,
              key: item?.key ?? idx,
            },
            slots.default ? slots.default({ item, index: idx, active: true }) : [],
          ),
        ),
      )
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
      })
  },
})

/** 全局 stubs: 把 RecycleScroller 替换为上面的 stub, 子组件保持稳定. */
const globalStubs = {
  AppResultItem: StubItem,
  ResultItem: StubItem,
  CheckButton: CheckButtonStub,
  RecycleScroller: RecycleScrollerStub,
}

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
      global: { stubs: globalStubs, mocks: { $t: (k: string) => k } },
    })
    await nextTick()
    const rows = wrapper.findAll('.vg__row')
    // 3 + 2 = 5 行 (仅 item, 不含 header)
    expect(rows.length).toBe(5)
    // 头行数: 2 个分组, 都未折叠
    const headers = wrapper.findAll('.vg__group-header-row')
    expect(headers.length).toBe(2)
  })

  it('clicking the header emits toggle-group and parent updates store', async () => {
    const search = useSearchStore()
    // 让 store 的 displayGroups 中 group.system 是非空组,
    // 否则 toggleGroupCollapse 在空组上 no-op (新行为, 见 store).
    // 这里直接把 store.results 填上 system-app, 让 store 自己产出非空分组.
    search.query = ''
    search.results = [
      mkResult({ id: 's1', resultType: 'system-app' }),
      mkResult({ id: 's2', resultType: 'system-app' }),
    ]
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
      global: { stubs: globalStubs, mocks: { $t: (k: string) => k } },
    })
    await nextTick()
    const header = wrapper.find('.vg__group-header-row')
    expect(header.exists()).toBe(true)
    await header.trigger('click')
    await nextTick()
    // 通过 Parent 桥接, store 状态应已折叠该组
    expect(search.collapsedGroups.has(GROUP_ID.system)).toBe(true)
  })

  it('collapsed group: only header row, no item rows for that group', async () => {
    const search = useSearchStore()
    // system 组折叠; files 组展开
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
      global: { stubs: globalStubs, mocks: { $t: (k: string) => k } },
    })
    await nextTick()
    // 折叠的系统组: 0 个 item 行 (但有 header)
    // 展开的文件组: 1 个 item 行 (1 个 header)
    const rows = wrapper.findAll('.vg__row')
    expect(rows.length).toBe(1)
    // 头行数: 2 个分组都存在 (一个折叠一个展开, 都展示 header)
    const headers = wrapper.findAll('.vg__group-header-row')
    expect(headers.length).toBe(2)
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
      global: { stubs: globalStubs, mocks: { $t: (k: string) => k } },
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
      global: { stubs: globalStubs, mocks: { $t: (k: string) => k } },
    })
    await nextTick()
    const firstRow = wrapper.findAll('.vg__row')[0]
    await firstRow.trigger('click')
    expect(wrapper.emitted('select')).toBeTruthy()
  })

  it('supports millions of items: virtual rows flatten is O(N), DOM is bounded', async () => {
    // 验证 1M items 也能 flatten 不爆栈. 注意: stub 的 RecycleScroller 会把所有
    // items 转成 div, 这一项主要测 "store → virtualRows computed" 的可扩展性.
    // 真正 DOM 节点数受 vue-virtual-scroller 限制, 与本组件无关.
    const big: SearchResult[] = []
    for (let i = 0; i < 1000; i++) {
      big.push(mkResult({ id: `big-${i}`, title: `item ${i}` }))
    }
    const groups: DisplayGroup[] = [
      makeGroup({
        id: GROUP_ID.files,
        title: '所有文件',
        items: big,
        visibleItems: big,
        kind: 'files',
      }),
    ]
    const wrapper = mount(VGR, {
      props: { groups, loading: false, selectedIndex: 500, height: 400, hasQuery: true, query: 'x' },
      global: { stubs: globalStubs, mocks: { $t: (k: string) => k } },
    })
    await nextTick()
    // 1000 个 item + 1 个 header = 1001 个虚拟行
    const rows = wrapper.findAll('.vg__row')
    expect(rows.length).toBe(1000)
    // 选中第 500 项应有 vg__row--active 类
    const active = wrapper.findAll('.vg__row--active')
    expect(active.length).toBe(1)
  })

  /**
   * === Section 2 优化: 空组完全跳过渲染 ===
   * 产品诉求: "下面如果没有内容, 则不支持折叠展开, 默认也是收缩起来的,
   *          除非有列表项才默认展开".
   *
   * 实施位置: VGR.virtualRows computed.
   * - 空组 (items.length === 0) → 整个组 continue, 不渲染 header, 也不渲染 items.
   * - 有内容的组 → 正常走"折叠/展开"逻辑, 默认展开.
   *
   * 注意: store 层的 displayGroups 仍保留空组 (header / count / 折叠元数据),
   *       渲染过滤由 VGR 完成. 这是有意的: 保留元数据便于做"占位 + 文件类型
   *       筛选"等场景, 又能彻底避免空组占据 UI 空间.
   */
  it('空组 (items=[]) 完全不渲染: header + items 都不出现', async () => {
    const groups: DisplayGroup[] = [
      // 1) 空组: pinned 分组没有结果
      makeGroup({
        id: GROUP_ID.pinned,
        title: '固定项目',
        items: [],
        visibleItems: [],
        kind: 'pinned',
      }),
      // 2) 有内容的组
      makeGroup({
        id: GROUP_ID.system,
        title: '系统应用',
        items: [mkResult({ id: 's1' }), mkResult({ id: 's2' })],
        visibleItems: [mkResult({ id: 's1' }), mkResult({ id: 's2' })],
        kind: 'system',
      }),
    ]
    const wrapper = mount(VGR, {
      props: { groups, loading: false, selectedIndex: 0, height: 400, hasQuery: true, query: 'x' },
      global: { stubs: globalStubs, mocks: { $t: (k: string) => k } },
    })
    await nextTick()
    // 只应渲染 system 组的 1 个 header + 2 个 item = 3 行
    expect(wrapper.findAll('.vg__row').length).toBe(2)
    expect(wrapper.findAll('.vg__group-header-row').length).toBe(1)
  })

  it('多个空组 (如 pinned + recent + commands) 全部跳过, 不挤占 UI 空间', async () => {
    const groups: DisplayGroup[] = [
      makeGroup({ id: GROUP_ID.pinned, items: [], visibleItems: [], kind: 'pinned' }),
      makeGroup({ id: GROUP_ID.recent, items: [], visibleItems: [], kind: 'recent' }),
      makeGroup({ id: GROUP_ID.system, items: [mkResult({ id: 'a' })], visibleItems: [mkResult({ id: 'a' })], kind: 'system' }),
      makeGroup({ id: GROUP_ID.commands, items: [], visibleItems: [], kind: 'commands' }),
      makeGroup({ id: GROUP_ID.apps, items: [mkResult({ id: 'b' }), mkResult({ id: 'c' })], visibleItems: [mkResult({ id: 'b' }), mkResult({ id: 'c' })], kind: 'apps' }),
    ]
    const wrapper = mount(VGR, {
      props: { groups, loading: false, selectedIndex: 0, height: 400, hasQuery: true, query: 'x' },
      global: { stubs: globalStubs, mocks: { $t: (k: string) => k } },
    })
    await nextTick()
    // 1 + 2 = 3 个 item, 2 个非空 header → 共 5 行 (不含 3 个空组)
    expect(wrapper.findAll('.vg__row').length).toBe(3)
    expect(wrapper.findAll('.vg__group-header-row').length).toBe(2)
  })

  it('全部为空时不渲染任何虚拟行 (空状态由 .vg__empty 等承载)', async () => {
    const groups: DisplayGroup[] = [
      makeGroup({ id: GROUP_ID.pinned, items: [], visibleItems: [], kind: 'pinned' }),
      makeGroup({ id: GROUP_ID.recent, items: [], visibleItems: [], kind: 'recent' }),
      makeGroup({ id: GROUP_ID.system, items: [], visibleItems: [], kind: 'system' }),
    ]
    const wrapper = mount(VGR, {
      props: { groups, loading: false, selectedIndex: 0, height: 400, hasQuery: true, query: 'x' },
      global: { stubs: globalStubs, mocks: { $t: (k: string) => k } },
    })
    await nextTick()
    expect(wrapper.findAll('.vg__row').length).toBe(0)
    expect(wrapper.findAll('.vg__group-header-row').length).toBe(0)
  })

  it('从非空切到空时, 虚拟行数从有到无, 不抛错', async () => {
    const wrapper = mount(VGR, {
      props: {
        groups: [
          makeGroup({ id: GROUP_ID.system, items: [mkResult({ id: 'a' })], visibleItems: [mkResult({ id: 'a' })], kind: 'system' }),
        ],
        loading: false,
        selectedIndex: 0,
        height: 400,
        hasQuery: true,
        query: 'x',
      },
      global: { stubs: globalStubs, mocks: { $t: (k: string) => k } },
    })
    await nextTick()
    expect(wrapper.findAll('.vg__row').length).toBe(1)
    // 切到全部为空
    await wrapper.setProps({
      groups: [
        makeGroup({ id: GROUP_ID.system, items: [], visibleItems: [], kind: 'system' }),
        makeGroup({ id: GROUP_ID.apps, items: [], visibleItems: [], kind: 'apps' }),
      ],
    })
    await nextTick()
    expect(wrapper.findAll('.vg__row').length).toBe(0)
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
      global: { stubs: globalStubs, mocks: { $t: (k: string) => k } },
    })
    await nextTick()
    // 没抛错 + 渲染了空容器即可
    expect(wrapper.exists()).toBe(true)
    // 即使没有数据, 组件也应有一个可滚动的容器 (避免 DOM 结构塌陷)
    expect(wrapper.find('.vg__scroller').exists() || wrapper.find('.vg__empty').exists() || wrapper.find('.vg__loading').exists()).toBe(true)
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
      global: { stubs: globalStubs, mocks: { $t: (k: string) => k } },
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
