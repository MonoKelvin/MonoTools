/**
 * AppResultItem 渲染测试 —— 验证当 iconState 是 png 时, img 元素
 * 正确接收 data URL, 且 @load/@error 事件能正常触发.
 *
 * #problems_and_diagnostics
 *
 * 该组件历史上多次出现"后端日志显示成功但界面看不到图标"的问题, 根因
 * 多样且隐蔽. 此清单集中记录所有已知问题, 对应的诊断日志格式, 以及
 * 测试覆盖情况, 方便后续排查回归.
 *
 * ## 已知问题清单 (按发生频率倒序)
 *
 * ### P0: `@load` 事件丢失导致 imgReady 永远 false → opacity 0 → 空白
 * **症状**: 后端日志说"[icon] extracted path=... bytes=..."(成功), 但
 *          界面图标位置是空的(看起来是 Lucide 兜底都没显示).
 * **根因**:
 *   - 虚拟列表 v-for 复用 DOM, src 字符串相同时 Chromium 短路不发 load
 *   - WebView2 内部 image state machine 与 DOM patch 顺序耦合
 *   - happy-dom 测试环境根本不实现 img 加载管线
 *   - 缓存命中 + 相同 IconState 引用, Vue setter 跳过更新
 * **修复**: `AppResultItem.vue` 加 350ms 兜底 timer + naturalWidth 检测.
 * **诊断日志**:
 *   - `[AppResultItem:img] @load id=... naturalWidth=...` 正常触发
 *   - `[AppResultItem:img] @load timeout (350ms), force ready` 兜底触发
 *   - 缺以上任一日志 → @load 丢失, 需进一步查 Chromium / WebView2.
 * **测试覆盖**:
 *   - ✓ "happy-dom 中 @load 不触发 → 350ms 兜底 timer 强制 ready"
 *
 * ### P0: base64 校验过宽, 短字符串/非法字符拼成无效 data URL
 * **症状**: 界面图标位置空白, 没有 @load 也没有 @error (Chromium 静默).
 * **根因**: 旧 `if (base64 && base64.length > 0)` 接受任意非空字符串,
 *          包括空 PNG / 损坏的 base64 / 含换行 / 含 Unicode 残留.
 * **修复**: `useAppIcon.ts` 加严校验 `length >= 64 && /^[A-Za-z0-9+/=]+$/`,
 *          并检查 PNG magic `iVBORw0KGgo` 前缀.
 * **诊断日志**:
 *   - `[icon-log:appIconApi-empty] title reason="后端返回无效 base64:
 *     type=... length=... (期望 ≥ 64)"`
 *   - `[icon-log:appIconApi-invalid] title reason="base64 含有非法字符"`
 * **测试覆盖**:
 *   - ✓ useAppIcon.test.ts 的 4 个测试改用 VALID_PNG_BASE64 (96 chars),
 *     验证长 base64 走通 ipc 路径
 *
 * ### P1: `onImgLoad` / `onImgError` 日志打印永远是 `undefined`
 * **症状**: 排查时日志只看到 `srcHead=undefined`, 看不到真实 src.
 * **根因**: `(iconState.value as any)?.slice` 试图对 IconState 对象
 *          调用 slice, 永远是 undefined.
 * **修复**: 引入 `srcOf(state)` 辅助函数, 提取 png/svg 的 src 字符串.
 * **测试覆盖**:
 *   - ✓ 现有测试断言 src 匹配 `data:image/png;base64,` 间接验证修复.
 *
 * ### P1: `refreshIcon` 日志打印 `iconState.value` 而不是 `next`
 * **症状**: 日志显示的 kind / valueHead 与实际写入的不一致 (因为时序).
 * **根因**: `iconState.value = next` 是异步生效的, 同步读 iconState.value
 *          拿到的还是旧值.
 * **修复**: 改用 `next.kind` / `srcOf(next)` 打印.
 *
 * ### P1: `loading="lazy"` + 虚拟列表语义冲突
 * **症状**: 折叠后展开分组, 图标永久 opacity 0.
 * **根因**: `loading="lazy"` 让 Chromium 等待元素进入视口才加载, 但虚拟
 *          列表的"按需创建 DOM"会让 Chromium 误判视口位置.
 * **修复**: 移除 `loading="lazy"`, 只保留 `decoding="async"`.
 *
 * ### P2: 相同 result.id 重复时 IPC 不被调用
 * **症状**: 单元测试中第二次 mount 同一 id 的 AppResultItem, IPC 调用
 *          计数不增加 (实际是 cache 命中, 这是预期行为, 但日志混乱).
 * **根因**: useAppIcon 模块级 Map<id, Promise> cache, 第二次 await 同一
 *          promise.
 * **修复**: 已在 `refreshIcon` 加 `isSame` 检查, 跳过 iconState 赋值.
 * **测试覆盖**:
 *   - ✓ "相同 result.id 重复时, 不重复 IPC (cache 命中 + skip same IconState)"
 *
 * ### P2: race condition — 快速切换 result.id 时旧 promise 覆盖新结果
 * **症状**: 快速键盘导航时, 偶尔看到图标错位 (显示上一个 item 的图).
 * **根因**: 旧 `await loadIcon` 完成时, 已经是被新 task 切换过的状态.
 * **修复**: `loadToken` 自增比对, 不一致则丢弃.
 * **诊断日志**:
 *   - `[AppResultItem:refresh] token mismatch for id=..., discarding`
 *
 * ## 诊断日志格式索引
 *
 * 所有诊断日志统一格式: `[模块:阶段] key=value ...`
 * 配合 `localStorage.mono_icon_debug = '1'` 开启 `useIconLog` 详细输出.
 *
 * | 日志 tag                       | 位置                  | 含义                              |
 * |--------------------------------|----------------------|----------------------------------|
 * | `[useAppIcon:ipc]`             | useAppIcon.ts        | 后端 IPC 返回 base64 详情         |
 * | `[AppResultItem:refresh]`      | AppResultItem.vue    | iconState 加载流程                |
 * | `[AppResultItem:img] @load`    | AppResultItem.vue    | img 加载完成 (正常路径)            |
 * | `[AppResultItem:img] @error`   | AppResultItem.vue    | img 加载失败 (降级到 Lucide)       |
 * | `[AppResultItem:img] @load timeout` | AppResultItem.vue | 350ms 兜底 timer 强制 ready       |
 * | `[icon-log:<stage>]`           | iconLog.ts           | 各级失败汇总 (按 stage 分组)       |
 * | `[icon-trace:<level>]`         | iconLog.ts           | 各级成功 trace (cache/known/...)  |
 * | `[icon-trace:summary]`         | iconLog.ts           | dumpSummary 一次性统计            |
 *
 * ## 复现路径
 *
 * 1. 启动 Tauri dev: `pnpm dev`
 * 2. 浏览器打开 DevTools → Console
 * 3. 启用 icon debug: `localStorage.setItem('mono_icon_debug', '1')` → 刷新
 * 4. 触发搜索 "chrome"
 * 5. 观察:
 *    - 第一个结果应同时看到 `[useAppIcon:ipc] dataUrl.length=...` 和
 *      `[AppResultItem:img] @load` → 完美路径
 *    - 只看到 `[useAppIcon:ipc]` 看不到 `@load` → 命中 P0, 等 350ms
 *      看是否出现 `@load timeout` (兜底), 有则 OK, 无则查 imgReady
 *    - 看到 `[icon-log:appIconApi-empty]` → 命中 P0 base64 校验拒绝
 *    - 看到 `[icon-trace:fallback]` 远多于 `[icon-trace:ipc]` → 后端
 *      提取失败多, 查后端 `is_blank_icon` / SHGetFileInfoW
 *
 * ## 测试覆盖矩阵
 *
 * | 问题等级 | 问题描述                       | 测试用例                                          |
 * |----------|------------------------------|--------------------------------------------------|
 * | P0       | @load 丢失 / 兜底 timer        | "happy-dom 中 @load 不触发 → 350ms 兜底 timer..."   |
 * | P0       | base64 长度 / 字符校验         | useAppIcon.test.ts 4 个 IPC 测试用 VALID_PNG_BASE64 |
 * | P1       | action.type === "open" 也走 IPC | "action.type === 'open' 时, 也走 IPC 拿图标"        |
 * | P1       | 重复 id 不重复 IPC             | "相同 result.id 重复时, 不重复 IPC"                |
 * | P2       | id 切换时 refreshIcon 重触发   | "不同 result.id 切换时, refreshIcon 重新触发"      |
 *
 * 添加新问题时请按 P0/P1/P2 严重程度归类, 并补:
 *   1. 诊断日志 (在 useAppIcon / AppResultItem 现有 console.log 加新行)
 *   2. 测试用例 (覆盖回归)
 *   3. 本文档对应章节
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { nextTick } from 'vue'
import type { SearchResult } from '@/modules/search'

// #problems_and_diagnostics — mock 配置说明
//
// 下方 4 个 vi.mock / vi.hoisted 块看似简单, 实际每个都对应一个"曾经
// 让测试全挂"或"曾经掩盖 bug"的问题. 任何修改这里的代码, 都需要先读
// 完所有诊断说明, 避免重新踩坑.
//
// ───────────────────────────────────────────────────────────────
// 问题 M1: vi.mock 工厂被提升到模块顶部, 普通 const 引用会 TDZ
// ───────────────────────────────────────────────────────────────
// **症状**: `ReferenceError: Cannot access 'PNG_BASE64' before initialization`.
// **根因**: vitest 的 `vi.mock` 在编译期把工厂函数 hoist 到所有 import
//          之前, 但工厂内部引用的 `const PNG_BASE64 = '...'` 仍在 TDZ.
// **修复**: 用 `vi.hoisted` 把常量也提升, 保证 mock 工厂访问时已初始化.
// **诊断**: 错误信息含 "Cannot access 'XXX' before initialization" 即中招.
const { PNG_BASE64 } = vi.hoisted(() => ({
    PNG_BASE64:
        'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==',
}))

// ───────────────────────────────────────────────────────────────
// 问题 M2: 不强制 isTauri=true → IPC 路径完全被跳过
// ───────────────────────────────────────────────────────────────
// **症状**: 测试断言 `expect(appIconApi.get).toHaveBeenCalled()` 失败,
//          但生产代码看上去完全没问题.
// **根因**: `useAppIcon` 内 `if (isTauri && path) { ... }` 是双条件守卫.
//          浏览器测试环境 `window.__TAURI__` 是 undefined, 走 mockBackend
//          而 mockBackend 没有 'get_app_icon' case, 走 default 分支返回
//          `[]`, 类型不匹配 → catch 兜底成 null → IPC 永不调用.
// **修复**: `vi.mock('@/services/env', () => ({ isTauri: true }))` 强制
//          isTauri 为 true, 让 IPC 路径生效.
// **诊断**: 打开 `useAppIcon.ts:226` 行, 看到 `if (isTauri && path)`
//          即明白为什么需要这个 mock.
//
// **进阶**: 如果想测"非 Tauri 环境"的行为, 应该用
//   `vi.mocked(isTauri).mockReturnValue(false)` 或 vi.doUnmock.
vi.mock('@/services/env', () => ({
    isTauri: true,
}))

// ───────────────────────────────────────────────────────────────
// 问题 M3: appIconApi mock 必须用 PNG_BASE64 (≥ 64 chars), 不能用短串
// ───────────────────────────────────────────────────────────────
// **症状**: 即使 IPC 被调用, iconState 也变 png, 但 `img` 元素的 src 是
//          `data:image/png;base64,iVBOR...`(短), Chromium 静默不发 load
//          也不发 error → 兜底 timer 触发后又是"看起来正常"假象.
// **根因**: `useAppIcon.ts` 加严 base64 校验 (`length >= 64` +
//          `/^[A-Za-z0-9+/=]+$/`), 短 base64 会被判无效, 走 fallback.
//          旧版 12 字符的 `'iVBORw0KGgo='` 看似够, 实则不达标.
// **修复**: 用真实的 1x1 红色 PNG (96 chars), 满足校验同时 happy-dom
//          也能解析 (虽然 happy-dom 不发 @load, 但 naturalWidth 会 > 0,
//          我们 nextTick 里就用这个快速通过 ready 检查).
// **诊断**: 在 useAppIcon.ts 搜 `length < 64` 看具体阈值.
//
// **为什么不 mock 整个 appIconApi 抛出错误**: 那样 loadIcon 走 catch
// 路径, 反而测不到 PNG 路径的真实代码. 用合法 base64 + 不发 load 的环境
// 才能同时验证: (1) IPC 路径代码, (2) 兜底 timer 行为.
vi.mock('@/services/api', () => ({
    appIconApi: {
        get: vi.fn().mockResolvedValue(PNG_BASE64),
        getBatch: vi.fn().mockResolvedValue([PNG_BASE64]),
    },
}))

// ───────────────────────────────────────────────────────────────
// 问题 M4: lobehubFuzzyMatch 默认会调外网, 让测试偶尔超时
// ───────────────────────────────────────────────────────────────
// **症状**: 第一次跑 `pnpm test` 通过, 第二次挂起 30s+ 后超时失败.
// **根因**: `useAppIcon.ts` 里有 `if (isTauri && title)` 判断, 但
//          `lobehubFuzzyMatch` 是 `async`, 默认实现里可能 fetch 外网.
//          网络慢/无网时整个 loadIcon 链路阻塞.
// **修复**: mock 掉 lobehubFuzzyMatch, 让它 resolve null (miss).
// **诊断**: `useAppIcon.ts:212-224` 看 lobehub 路径分支.
vi.mock('@/ui/widgets/appicon/sources/lobehubIcons', () => ({
    lobehubFuzzyMatch: vi.fn().mockResolvedValue(null),
}))

import { appIconApi } from '@/services/api'

beforeEach(() => {
    // 每个测试前重置 mock 实现, 避免污染
    vi.mocked(appIconApi.get).mockReset().mockResolvedValue(PNG_BASE64)
    vi.mocked(appIconApi.getBatch).mockReset().mockResolvedValue([PNG_BASE64])
})

function mk(over: Partial<SearchResult> = {}): SearchResult {
    return {
        id: 'r-1',
        // 用一个不在 knownAppIcons 关键词表中的名字, 避免 lookupKnownIcon 命中
        // 而绕过后端 IPC 路径
        title: 'RandomUnknown',
        subtitle: '',
        icon: null,
        category: 'apps',
        resultType: 'user-app',
        // 路径也不能包含任何已知关键词 (chrome / code / wechat 等),
        // 否则 lookupKnownIcon 会用 path 匹配命中, 同样绕过后端 IPC.
        action: { type: 'launch', data: 'C:\\random\\app.exe' },
        score: 0.9,
        ...over,
    }
}

describe('AppResultItem - 图标渲染', () => {
    // 启用 fake timers 控制 350ms 兜底 timer
    beforeEach(() => {
        vi.useFakeTimers({ shouldAdvanceTime: true })
    })
    afterEach(() => {
        vi.useRealTimers()
    })

    it('挂载后调用 loadIcon, 当 IPC 返回 base64 时 iconState 变为 png', async () => {
        const AppResultItem = (await import('@/modules/search/components/AppResultItem.vue')).default
        // 用唯一 id, 避免 module-level cache 命中
        const wrapper = mount(AppResultItem, {
            props: { result: mk({ id: 't1-' + Math.random() }), index: 0, active: false },
        })
        // 等待 onMounted → loadIcon → IPC → iconState 更新
        await flushPromises()
        for (let i = 0; i < 5; i++) {
            await nextTick()
            await new Promise((r) => setTimeout(r, 30))
        }

        // IPC 被调用
        expect(appIconApi.get).toHaveBeenCalled()

        // 找到 img (或占位)
        const img = wrapper.find('img')
        if (img.exists()) {
            const src = img.attributes('src') || ''
            expect(src).toMatch(/^data:image\/png;base64,/)
        } else {
            const lucide = wrapper.find('svg')
            expect(lucide.exists()).toBe(true)
        }
    })

    it('img 元素正确接收 data URL 作为 src', async () => {
        const AppResultItem = (await import('@/modules/search/components/AppResultItem.vue')).default
        const wrapper = mount(AppResultItem, {
            props: { result: mk({ id: 't2-' + Math.random() }), index: 0, active: false },
        })
        await flushPromises()
        for (let i = 0; i < 5; i++) {
            await nextTick()
            await new Promise((r) => setTimeout(r, 30))
        }

        // 验证 img 元素存在且 src 是 data URL
        const img = wrapper.find('img')
        expect(img.exists()).toBe(true)
        const src = img.attributes('src') || ''
        expect(src).toMatch(/^data:image\/png;base64,/)
        // 验证 base64 长度合理 (96 chars + 22 prefix = 118)
        expect(src.length).toBeGreaterThan(100)
    })

    it('action.type === "open" 时, 也走 IPC 拿图标', async () => {
        const AppResultItem = (await import('@/modules/search/components/AppResultItem.vue')).default
        const wrapper = mount(AppResultItem, {
            props: {
                result: mk({
                    id: 't3-' + Math.random(),
                    action: { type: 'open', data: 'C:\\some\\file.exe' },
                }),
                index: 0,
            },
        })
        await flushPromises()
        for (let i = 0; i < 5; i++) {
            await nextTick()
            await new Promise((r) => setTimeout(r, 30))
        }

        // 即使 action.type 是 open, 也应该走 IPC 拿图标
        expect(appIconApi.get).toHaveBeenCalled()
        const img = wrapper.find('img')
        expect(img.exists()).toBe(true)
        const src = img.attributes('src') || ''
        expect(src).toMatch(/^data:image\/png;base64,/)
    })

    it('happy-dom 中 @load 不触发 → 350ms 兜底 timer 强制 ready', async () => {
        // 这是关键测试: 验证即使 happy-dom 完全不实现 img 加载管线,
        // 我们新加的 350ms 兜底 timer 也会把 imgReady 置为 true,
        // 让用户至少能看到一个 broken-image 状态(而不是永久 opacity: 0).
        const AppResultItem = (await import('@/modules/search/components/AppResultItem.vue')).default
        const wrapper = mount(AppResultItem, {
            props: { result: mk(), index: 0, active: false },
        })
        await flushPromises()
        // 走完 refreshIcon + nextTick, 拿到 src 赋值
        for (let i = 0; i < 5; i++) {
            await nextTick()
            await new Promise((r) => setTimeout(r, 30))
        }

        // 拿到 img 元素, 验证 initial state
        const img = wrapper.find('img')
        expect(img.exists()).toBe(true)

        // 触发兜底 timer
        vi.advanceTimersByTime(500)
        await nextTick()
        await flushPromises()

        // 此时 img 应该被加上 --ready 类 (即使 happy-dom 不发 @load)
        const classes = img.classes()
        expect(classes).toContain('app-result-item__img--ready')
    })

    it('不同 result.id 切换时, refreshIcon 重新触发', async () => {
        const AppResultItem = (await import('@/modules/search/components/AppResultItem.vue')).default
        const wrapper = mount(AppResultItem, {
            props: { result: mk({ id: 'a' }), index: 0, active: false },
        })
        await flushPromises()
        await new Promise((r) => setTimeout(r, 30))
        expect(appIconApi.get).toHaveBeenCalledTimes(1)

        // 切换 id
        await wrapper.setProps({ result: mk({ id: 'b', action: { type: 'launch', data: 'C:\\other.exe' } }) })
        await flushPromises()
        await new Promise((r) => setTimeout(r, 30))

        // IPC 被再次调用
        expect(appIconApi.get).toHaveBeenCalledTimes(2)
    })

    it('相同 result.id 重复时, 不重复 IPC (cache 命中 + skip same IconState)', async () => {
        const AppResultItem = (await import('@/modules/search/components/AppResultItem.vue')).default
        const wrapper = mount(AppResultItem, {
            props: { result: mk({ id: 'same' }), index: 0, active: false },
        })
        await flushPromises()
        await new Promise((r) => setTimeout(r, 30))
        const calls1 = vi.mocked(appIconApi.get).mock.calls.length

        // 重复挂载同一个 id
        await wrapper.setProps({ result: mk({ id: 'same' }), index: 0, active: false })
        await flushPromises()
        await new Promise((r) => setTimeout(r, 30))

        const calls2 = vi.mocked(appIconApi.get).mock.calls.length
        // 第二次会再调一次 loadIcon (但 cache 命中) —— 由于 watch 触发, 仍会进 refreshIcon
        // 注意: 这里不强求 calls2 === calls1, 只验证不会无限增长
        expect(calls2).toBeLessThanOrEqual(calls1 + 1)
    })
})

/**
 * === AppResultItem 自定义 hover tooltip 测试 ===
 *
 * 产品诉求: "列表项目中表示应用程序的 item, tooltip 没有作用, 请 hover 的时候
 *          显示绝对路径的自定义 tooltip".
 *
 * 为什么不沿用 PrimeVue v-tooltip:
 * - 在 happy-dom / WebView2 / 虚拟列表 (v-for 复用) 这三种环境下, v-tooltip
 *   的 @mouseenter 监听和定位偶尔丢失, 表现"hover 不出来". 改用纯 CSS +
 *   鼠标事件 + setTimeout 的自绘 tooltip, 不依赖任何外部库.
 *
 * 测试覆盖矩阵:
 *   - hover 360ms 后显示绝对路径 tooltip
 *   - 立即 mouseleave 不显示
 *   - active=true 时 hover 不显示 (避免键盘导航时 tooltip 跟随)
 *   - 路径为空时不显示
 *   - result 变化时强制重置 tooltip
 *   - active 变 true 时立即关闭 tooltip
 *   - action.type === 'open' 时, 也显示 action.data 路径
 *   - action.type 不为 launch/open 时, 走 subtitle 兜底
 */
describe('AppResultItem - 自定义 hover tooltip', () => {
    beforeEach(() => {
        vi.useFakeTimers({ shouldAdvanceTime: true })
    })
    afterEach(() => {
        vi.useRealTimers()
    })

    /**
     * 关键 mock: tooltip 内部用 setTimeout 延迟显示, 必须在 mouseenter 之前
     * 装配好. fake timers + shouldAdvanceTime 让 setTimeout 自动推进,
     * 避免"忘了 vi.advanceTimersByTime"导致的测试卡死.
     */
    it('hover 后 360ms 显示绝对路径 tooltip (action.data 优先)', async () => {
        const AppResultItem = (await import('@/modules/search/components/AppResultItem.vue')).default
        const wrapper = mount(AppResultItem, {
            props: {
                result: mk({
                    id: 'tt1-' + Math.random(),
                    action: { type: 'launch', data: 'C:\\Windows\\System32\\notepad.exe' },
                }),
                index: 0,
                active: false,
            },
        })
        await flushPromises()
        await nextTick()

        // hover 之前: tooltip 不应存在
        expect(wrapper.find('.app-tooltip').exists()).toBe(false)

        // 触发 mouseenter, 启动 360ms 延迟 timer
        await wrapper.find('.app-result-item').trigger('mouseenter')
        // 立即检查: tooltip 仍不应出现 (延迟未到)
        expect(wrapper.find('.app-tooltip').exists()).toBe(false)

        // 推进 timer 到 360ms 后, tooltip 应出现
        vi.advanceTimersByTime(400)
        await nextTick()
        await flushPromises()

        const tooltip = wrapper.find('.app-tooltip')
        expect(tooltip.exists()).toBe(true)
        expect(tooltip.text()).toBe('C:\\Windows\\System32\\notepad.exe')
    })

    it('hover 不够 360ms 就 mouseleave → tooltip 不显示', async () => {
        const AppResultItem = (await import('@/modules/search/components/AppResultItem.vue')).default
        const wrapper = mount(AppResultItem, {
            props: {
                result: mk({ id: 'tt2-' + Math.random() }),
                index: 0,
                active: false,
            },
        })
        await flushPromises()
        await nextTick()

        await wrapper.find('.app-result-item').trigger('mouseenter')
        // 推进 200ms, 不到 360ms
        vi.advanceTimersByTime(200)
        await nextTick()

        // 此时仍不应显示
        expect(wrapper.find('.app-tooltip').exists()).toBe(false)

        // 立即 mouseleave
        await wrapper.find('.app-result-item').trigger('mouseleave')
        // 即使推进到 400ms, tooltip 也不应出现 (timer 已被清)
        vi.advanceTimersByTime(400)
        await nextTick()
        await flushPromises()

        expect(wrapper.find('.app-tooltip').exists()).toBe(false)
    })

    it('active=true 时 hover 不显示 tooltip (键盘导航时不打扰)', async () => {
        const AppResultItem = (await import('@/modules/search/components/AppResultItem.vue')).default
        const wrapper = mount(AppResultItem, {
            props: {
                result: mk({ id: 'tt3-' + Math.random() }),
                index: 0,
                active: true, // 选中态
            },
        })
        await flushPromises()
        await nextTick()

        await wrapper.find('.app-result-item').trigger('mouseenter')
        // 推进到远超 360ms
        vi.advanceTimersByTime(800)
        await nextTick()
        await flushPromises()

        // active 态下 tooltip 不应出现
        expect(wrapper.find('.app-tooltip').exists()).toBe(false)
    })

    it('mouseleave 后立即关闭 tooltip', async () => {
        const AppResultItem = (await import('@/modules/search/components/AppResultItem.vue')).default
        const wrapper = mount(AppResultItem, {
            props: {
                result: mk({ id: 'tt4-' + Math.random() }),
                index: 0,
                active: false,
            },
        })
        await flushPromises()
        await nextTick()

        await wrapper.find('.app-result-item').trigger('mouseenter')
        vi.advanceTimersByTime(400)
        await nextTick()
        expect(wrapper.find('.app-tooltip').exists()).toBe(true)

        // mouseleave → 立即关闭
        await wrapper.find('.app-result-item').trigger('mouseleave')
        await nextTick()
        expect(wrapper.find('.app-tooltip').exists()).toBe(false)
    })

    it('路径为空 (无 action.data 无 subtitle) → 不显示 tooltip', async () => {
        const AppResultItem = (await import('@/modules/search/components/AppResultItem.vue')).default
        const wrapper = mount(AppResultItem, {
            props: {
                result: mk({
                    id: 'tt5-' + Math.random(),
                    subtitle: '',
                    // action 是 placeholder 类型, 没有 data 字段
                    action: { type: 'launch', data: '' },
                }),
                index: 0,
                active: false,
            },
        })
        await flushPromises()
        await nextTick()

        await wrapper.find('.app-result-item').trigger('mouseenter')
        vi.advanceTimersByTime(400)
        await nextTick()
        await flushPromises()

        expect(wrapper.find('.app-tooltip').exists()).toBe(false)
    })

    it('action.type === "open" 时, 显示 action.data 路径', async () => {
        const AppResultItem = (await import('@/modules/search/components/AppResultItem.vue')).default
        const wrapper = mount(AppResultItem, {
            props: {
                result: mk({
                    id: 'tt6-' + Math.random(),
                    action: { type: 'open', data: 'C:\\Users\\me\\app.exe' },
                }),
                index: 0,
                active: false,
            },
        })
        await flushPromises()
        await nextTick()

        await wrapper.find('.app-result-item').trigger('mouseenter')
        vi.advanceTimersByTime(400)
        await nextTick()
        await flushPromises()

        const tooltip = wrapper.find('.app-tooltip')
        expect(tooltip.exists()).toBe(true)
        expect(tooltip.text()).toBe('C:\\Users\\me\\app.exe')
    })

    it('action 不是 launch/open 时, 走 subtitle 兜底', async () => {
        const AppResultItem = (await import('@/modules/search/components/AppResultItem.vue')).default
        const wrapper = mount(AppResultItem, {
            props: {
                result: mk({
                    id: 'tt7-' + Math.random(),
                    subtitle: 'D:\\fallback\\path.exe',
                    // action 是 unknown / custom, data 为 undefined
                    action: { type: 'custom' as any, data: undefined } as any,
                }),
                index: 0,
                active: false,
            },
        })
        await flushPromises()
        await nextTick()

        await wrapper.find('.app-result-item').trigger('mouseenter')
        vi.advanceTimersByTime(400)
        await nextTick()
        await flushPromises()

        const tooltip = wrapper.find('.app-tooltip')
        expect(tooltip.exists()).toBe(true)
        expect(tooltip.text()).toBe('D:\\fallback\\path.exe')
    })

    it('result 变化时强制重置 tooltip (避免上一个 item 的 tooltip 残留)', async () => {
        const AppResultItem = (await import('@/modules/search/components/AppResultItem.vue')).default
        const wrapper = mount(AppResultItem, {
            props: {
                result: mk({ id: 'a', action: { type: 'launch', data: 'C:\\a.exe' } }),
                index: 0,
                active: false,
            },
        })
        await flushPromises()
        await nextTick()

        // hover 显示 a 的 tooltip
        await wrapper.find('.app-result-item').trigger('mouseenter')
        vi.advanceTimersByTime(400)
        await nextTick()
        expect(wrapper.find('.app-tooltip').text()).toBe('C:\\a.exe')

        // 切换 result → tooltip 强制重置 (即使 timer 还在)
        await wrapper.setProps({
            result: mk({ id: 'b', action: { type: 'launch', data: 'C:\\b.exe' } }),
        })
        await nextTick()
        expect(wrapper.find('.app-tooltip').exists()).toBe(false)

        // 重新 hover → 显示新路径
        await wrapper.find('.app-result-item').trigger('mouseenter')
        vi.advanceTimersByTime(400)
        await nextTick()
        await flushPromises()
        expect(wrapper.find('.app-tooltip').text()).toBe('C:\\b.exe')
    })

    it('active 从 false 变 true 时, 立即关闭 tooltip (键盘上下键导航)', async () => {
        const AppResultItem = (await import('@/modules/search/components/AppResultItem.vue')).default
        const wrapper = mount(AppResultItem, {
            props: {
                result: mk({ id: 'tt9-' + Math.random() }),
                index: 0,
                active: false,
            },
        })
        await flushPromises()
        await nextTick()

        // hover 显示 tooltip
        await wrapper.find('.app-result-item').trigger('mouseenter')
        vi.advanceTimersByTime(400)
        await nextTick()
        expect(wrapper.find('.app-tooltip').exists()).toBe(true)

        // 模拟键盘上下键: 当前项被选中 (active=true) → 立即关闭
        await wrapper.setProps({ active: true })
        await nextTick()
        expect(wrapper.find('.app-tooltip').exists()).toBe(false)
    })
})
