/**
 * GroupSection 组件测试 —— 覆盖关键交互逻辑:
 *   1. active/hover 类在 list/grid/icon 三种布局下正确渲染
 *   2. 空组不可折叠（data-interactive="0" + click 不 emit）
 *   3. 排序 combobox 切换 emit sort-change
 *   4. 布局 combobox 切换 emit layout-change
 *   5. 双击项目 emit open
 *   6. hover 进入/离开 emit hover / -1
 *   7. 右键菜单冒泡带 event.target
 *   8. startIndex 参与 globalIndex 计算
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { defineComponent, h, nextTick } from 'vue'
import GroupSection from '@/modules/search/components/GroupSection.vue'

// === Stubs ===

const StubAppItem = defineComponent({
  name: 'AppResultItem',
  props: ['result', 'active', 'index'],
  setup(props: any) {
    return () => h('div', { class: 'stub-app', 'data-id': props.result?.id }, props.result?.title)
  },
})

const StubResultItem = defineComponent({
  name: 'ResultItem',
  props: ['result', 'active', 'index'],
  setup(props: any) {
    return () => h('div', { class: 'stub-result', 'data-id': props.result?.id }, props.result?.title)
  },
})

const StubCombo = defineComponent({
  name: 'MtComboBox',
  props: ['modelValue', 'options'],
  setup(props, { emit }) {
    return () =>
      h('div', {
        class: 'stub-combo',
        'data-value': props.modelValue,
        onClick: () => emit('update:modelValue', 'name'),
      })
  },
})

// === 数据 ===

function mkResult(over: any = {}): any {
  return {
    id: 'g-' + Math.random().toString(36).slice(2, 8),
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

function mountGroup(over: any = {}) {
  const items = [mkResult({ id: 'a' }), mkResult({ id: 'b' }), mkResult({ id: 'c' })]
  return mount(GroupSection, {
    props: {
      id: 'g-1',
      title: '所有应用',
      icon: 'div',
      items,
      visibleItems: items,
      collapsed: false,
      kind: 'apps',
      selectedGlobalIndex: -1,
      hoveredGlobalIndex: -1,
      startIndex: 0,
      ...over,
    },
    global: {
      stubs: {
        AppResultItem: StubAppItem,
        ResultItem: StubResultItem,
        MtComboBox: StubCombo,
      },
    },
  })
}

// === 测试 ===

describe('GroupSection - 交互逻辑', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('list 布局下 active/hover 类由 selectedLocalIndex / hoveredLocalIndex 驱动', async () => {
    const wrapper = mountGroup({ selectedGlobalIndex: 1, hoveredGlobalIndex: 2 })
    await nextTick()
    const rows = wrapper.findAll('.gs-item')
    expect(rows.at(0).classes('gs-item--active')).toBe(false)
    expect(rows.at(1).classes('gs-item--active')).toBe(true)
    expect(rows.at(2).classes('gs-item--hover')).toBe(true)
  })

  it('grid 布局下 active 类同样生效', async () => {
    const wrapper = mountGroup({ selectedGlobalIndex: 0, defaultLayout: 'grid-fixed' })
    await nextTick()
    const rows = wrapper.findAll('.gs-item')
    expect(rows.at(0).classes('gs-item--active')).toBe(true)
    expect(rows.at(1).classes('gs-item--active')).toBe(false)
  })

  it('icon 布局下 active 类同样生效', async () => {
    const wrapper = mountGroup({ selectedGlobalIndex: 2, defaultLayout: 'icon' })
    await nextTick()
    const rows = wrapper.findAll('.gs-item')
    expect(rows.at(2).classes('gs-item--active')).toBe(true)
  })

  it('空组渲染为 data-interactive="0"，且 click header 不 emit toggle-collapse', async () => {
    const wrapper = mount(GroupSection, {
      props: {
        id: 'g-empty',
        title: '空分组',
        icon: 'div',
        items: [],
        visibleItems: [],
        collapsed: false,
        kind: 'system',
      },
      global: {
        stubs: {
          AppResultItem: StubAppItem,
          ResultItem: StubResultItem,
          MtComboBox: StubCombo,
        },
      },
    })
    await nextTick()
    const section = wrapper.find('.group-section')
    expect(section.attributes('data-interactive')).toBe('0')
    await section.find('.group-header').trigger('click')
    await nextTick()
    expect(wrapper.emitted('toggle-collapse')).toBeFalsy()
  })

  it('非空组点击 header emit toggle-collapse', async () => {
    const wrapper = mountGroup()
    await nextTick()
    await wrapper.find('.group-header').trigger('click')
    await nextTick()
    expect(wrapper.emitted('toggle-collapse')).toBeTruthy()
    expect(wrapper.emitted('toggle-collapse')![0]).toEqual(['g-1'])
  })

  it('排序 combobox 切换 emit sort-change', async () => {
    const wrapper = mountGroup()
    await nextTick()
    const combo = wrapper.find('.group-sort-toggle .stub-combo')
    await combo.trigger('click')
    await nextTick()
    expect(wrapper.emitted('sort-change')).toBeTruthy()
    expect(wrapper.emitted('sort-change')![0][0]).toBe('name')
  })

  it('布局 combobox 切换 emit layout-change', async () => {
    const wrapper = mountGroup({ defaultLayout: 'list' })
    await nextTick()
    const combo = wrapper.find('.group-layout-toggle .stub-combo')
    await combo.trigger('click')
    await nextTick()
    expect(wrapper.emitted('layout-change')).toBeTruthy()
    expect(wrapper.emitted('layout-change')![0][0]).toBe('name')
  })

  it('双击项目 emit open', async () => {
    const wrapper = mountGroup()
    await nextTick()
    const row = wrapper.find('.gs-item')
    await row.trigger('dblclick')
    await nextTick()
    const emitted = wrapper.emitted('open')
    expect(emitted).toBeTruthy()
    expect(emitted![0][0].id).toBe('a')
  })

  it('hover 进入/离开 emit hover / -1', async () => {
    const wrapper = mountGroup()
    await nextTick()
    const rows = wrapper.findAll('.gs-item')
    await rows.at(1).trigger('mouseenter')
    await nextTick()
    expect(wrapper.emitted('hover')![0][0]).toBe(1)
    await rows.at(1).trigger('mouseleave')
    await nextTick()
    expect(wrapper.emitted('hover')![1][0]).toBe(-1)
  })

  it('右键点击项目行时 emit contextmenu，并带上 event.target', async () => {
    const wrapper = mountGroup()
    await nextTick()
    const row = wrapper.find('.gs-item')
    await row.trigger('contextmenu')
    await nextTick()
    const emitted = wrapper.emitted('contextmenu')
    expect(emitted).toBeTruthy()
    expect(emitted![0][0]).toBeInstanceOf(MouseEvent)
    expect(emitted![0][1].id).toBe('a')
    expect(emitted![0][3]).toBeInstanceOf(HTMLElement)
    expect(emitted![0][3]).toBe(row.element)
  })

  it('startIndex 参与 globalIndex 计算', async () => {
    const wrapper = mountGroup({ startIndex: 10, selectedGlobalIndex: 12 })
    await nextTick()
    const rows = wrapper.findAll('.gs-item')
    expect(rows.at(2).classes('gs-item--active')).toBe(true)
    await rows.at(2).trigger('dblclick')
    await nextTick()
    expect(wrapper.emitted('open')![0][0].id).toBe('c')
  })
})
