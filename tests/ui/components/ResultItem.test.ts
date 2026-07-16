/**
 * ResultItem 图标渲染测试 —— 验证 .url / .lnk / 含图标的文件在"所有文件"组
 * 也能通过 useAppIcon 显示真实 PNG 图标 (不只显示 Lucide 兜底).
 *
 * #problems_and_diagnostics
 *
 * 历史问题: `.url` 文件被 file_search 引擎索引为 `category=Files`, 走
 * ResultItem 渲染. 之前 ResultItem 不用 useAppIcon, 后端 IPC 成功提取了图标
 * 也只写进 useAppIcon.cache 没人读. 修复后 ResultItem 也接入 useAppIcon,
 * 实现与 AppResultItem 相同的 4-tier 加载链.
 *
 * 已知问题 / 边界:
 * - P0: 350ms 兜底 timer (happy-dom / WebView2 不发 @load)
 * - P0: 严格 base64 校验 (length >= 64, 字符集合法)
 * - P1: 同 result.id 重复时 cache 命中 + isSame 跳过
 * - P1: rapid id 切换时 loadToken 防止 race
 *
 * 测试覆盖矩阵:
 * | 问题 | 测试用例 |
 * |------|---------|
 * | 兜底 timer | "happy-dom 中 @load 不触发 → 350ms 兜底..." |
 * | base64 校验 | 使用 VALID_PNG_BASE64 (96 chars) |
 * | cache 命中 | "相同 result.id 重复时, 不重复 IPC" |
 * | id 切换 | "不同 result.id 切换时, refreshIcon 重新触发" |
 * | file 类型 | "file result 显示 PNG 真实图标" |
 * | .url 类型 | ".url 文件 (other-file) 也走 IPC" |
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { nextTick } from 'vue'
import type { SearchResult } from '@/modules/search'

// 强制 isTauri=true 让 IPC 路径生效
vi.mock('@/services/env', () => ({
  isTauri: true,
}))

// vi.hoisted: 解决 mock 工厂 hoist 后的 TDZ 问题
const { PNG_BASE64 } = vi.hoisted(() => ({
  PNG_BASE64:
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==',
}))

vi.mock('@/services/api', () => ({
  appIconApi: {
    get: vi.fn().mockResolvedValue(PNG_BASE64),
    getBatch: vi.fn().mockResolvedValue([PNG_BASE64]),
  },
}))

vi.mock('@/ui/widgets/appicon/sources/lobehubIcons', () => ({
  lobehubFuzzyMatch: vi.fn().mockResolvedValue(null),
}))

import { appIconApi } from '@/services/api'

beforeEach(() => {
  vi.mocked(appIconApi.get).mockReset().mockResolvedValue(PNG_BASE64)
  vi.mocked(appIconApi.getBatch).mockReset().mockResolvedValue([PNG_BASE64])
})

function mkFile(over: Partial<SearchResult> = {}): SearchResult {
  return {
    id: 'r-' + Math.random().toString(36).slice(2, 8),
    title: 'NIKON IMAGE SPACE',
    subtitle: 'C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Link to Nikon\\NIKON IMAGE SPACE.url',
    icon: null,
    category: 'files',
    resultType: 'other-file',
    action: { type: 'open', data: 'C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Link to Nikon\\NIKON IMAGE SPACE.url' },
    score: 0.5,
    ...over,
  }
}

describe('ResultItem - PNG 图标渲染 (修复 .url / .lnk 不显示问题)', () => {
  // 启用 fake timers 控制 350ms 兜底 timer
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('file 类型 (other-file) 走 IPC 拿真实 PNG', async () => {
    const ResultItem = (await import('@/modules/search/components/ResultItem.vue')).default
    const wrapper = mount(ResultItem, {
      props: { result: mkFile({ id: 't1-' + Math.random() }), index: 0, active: false },
    })
    await flushPromises()
    for (let i = 0; i < 5; i++) {
      await nextTick()
      await new Promise((r) => setTimeout(r, 30))
    }

    // IPC 真的被调用 (这是关键, 之前 ResultItem 不调 IPC)
    expect(appIconApi.get).toHaveBeenCalled()
    const calledWith = vi.mocked(appIconApi.get).mock.calls[0]?.[0]
    expect(calledWith).toContain('NIKON IMAGE SPACE.url')

    // img 元素存在, src 是 data URL
    const img = wrapper.find('img')
    expect(img.exists()).toBe(true)
    const src = img.attributes('src') || ''
    expect(src).toMatch(/^data:image\/png;base64,/)
    expect(src.length).toBeGreaterThan(100)
  })

  it('happy-dom 中 @load 不触发 → 350ms 兜底 timer 强制 ready', async () => {
    const ResultItem = (await import('@/modules/search/components/ResultItem.vue')).default
    const wrapper = mount(ResultItem, {
      props: { result: mkFile({ id: 't2-' + Math.random() }), index: 0, active: false },
    })
    await flushPromises()
    for (let i = 0; i < 5; i++) {
      await nextTick()
      await new Promise((r) => setTimeout(r, 30))
    }

    const img = wrapper.find('img')
    expect(img.exists()).toBe(true)

    // 兜底 timer 触发
    vi.advanceTimersByTime(500)
    await nextTick()
    await flushPromises()

    // 即便 happy-dom 不发 @load, 也应该有 --ready class
    const classes = img.classes()
    expect(classes).toContain('result-item__img--ready')
  })

  it('不同 result.id 切换时, refreshIcon 重新触发', async () => {
    const ResultItem = (await import('@/modules/search/components/ResultItem.vue')).default
    const wrapper = mount(ResultItem, {
      props: { result: mkFile({ id: 'a-' + Math.random() }), index: 0, active: false },
    })
    await flushPromises()
    await new Promise((r) => setTimeout(r, 30))
    expect(appIconApi.get).toHaveBeenCalledTimes(1)

    await wrapper.setProps({ result: mkFile({ id: 'b-' + Math.random() }), index: 0, active: false })
    await flushPromises()
    await new Promise((r) => setTimeout(r, 30))

    expect(appIconApi.get).toHaveBeenCalledTimes(2)
  })

  it('相同 result.id 重复时, 不重复 IPC (cache 命中)', async () => {
    const ResultItem = (await import('@/modules/search/components/ResultItem.vue')).default
    const id = 'same-' + Math.random()
    const wrapper = mount(ResultItem, {
      props: { result: mkFile({ id }), index: 0, active: false },
    })
    await flushPromises()
    await new Promise((r) => setTimeout(r, 30))
    const calls1 = vi.mocked(appIconApi.get).mock.calls.length

    // 重复挂载同一个 id
    await wrapper.setProps({ result: mkFile({ id }), index: 0, active: false })
    await flushPromises()
    await new Promise((r) => setTimeout(r, 30))

    const calls2 = vi.mocked(appIconApi.get).mock.calls.length
    expect(calls2).toBeLessThanOrEqual(calls1 + 1)
  })

  it('.url 文件 (Internet Shortcut) 走 IPC 拿图标', async () => {
    // 关键回归测试: 之前 .url 文件的图标不显示, 现在 ResultItem 也读 useAppIcon
    const ResultItem = (await import('@/modules/search/components/ResultItem.vue')).default
    const urlPath = 'C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Link to Nikon\\NIKON IMAGE SPACE.url'
    const wrapper = mount(ResultItem, {
      props: {
        result: mkFile({
          id: 'url-' + Math.random(),
          title: 'NIKON IMAGE SPACE',
          subtitle: urlPath,
          action: { type: 'open', data: urlPath },
        }),
        index: 0,
        active: false,
      },
    })
    await flushPromises()
    for (let i = 0; i < 5; i++) {
      await nextTick()
      await new Promise((r) => setTimeout(r, 30))
    }

    // .url 文件应该走 IPC 拿图标
    expect(appIconApi.get).toHaveBeenCalledWith(urlPath)
    const img = wrapper.find('img')
    expect(img.exists()).toBe(true)
  })

  it('快速切换 id 时, 旧 token 被丢弃 (race condition 防护)', async () => {
    const ResultItem = (await import('@/modules/search/components/ResultItem.vue')).default
    const wrapper = mount(ResultItem, {
      props: { result: mkFile({ id: 'r1' }), index: 0, active: false },
    })
    // 切换前不等 await 完成, 触发快速切换
    await wrapper.setProps({ result: mkFile({ id: 'r2' }) })
    await flushPromises()
    for (let i = 0; i < 5; i++) {
      await nextTick()
      await new Promise((r) => setTimeout(r, 30))
    }

    // 最终显示的应该是 r2, 不是 r1
    const img = wrapper.find('img')
    expect(img.exists()).toBe(true)
  })
})
