/**
 * useIconRenderer —— 通用图标渲染组合式函数.
 *
 * 用途: 封装"图标的渲染状态"管理, 消除 AppResultItem / ResultItem
 * (以及未来可能新增的) 80% 重复的 icon 加载代码.
 *
 * 与 useAppIcon 的关系:
 * - `useAppIcon`: 4-tier 加载链 (cache → known → lobehub → ipc → fallback).
 *   关注"如何获取图标数据" (IconState).
 * - `useIconRenderer`: 渲染层. 关注"如何把 IconState 展示给用户",
 *   含 imgReady 状态机, 350ms 兜底 timer, loadToken race 防护,
 *   nextTick naturalWidth 检测, onImgLoad/onImgError 处理.
 *
 * 调用方只需要:
 * ```ts
 * const { iconState, imgReady, refresh, onImgLoad, onImgError, dispose } =
 *   useIconRenderer({
 *     fallbackComponent: AppWindow,           // 占位 Lucide
 *     containerSelector: (id) => `[data-my-id="${id}"] img`,  // DOM 查找
 *     debugTag: 'AppResultItem',             // 日志前缀
 *   })
 *
 * onMounted(() => refresh(props.result))
 * watch(() => props.result?.id, () => refresh(props.result))
 * onBeforeUnmount(dispose)
 * ```
 *
 * 模板保持完全不变, 仍用 `iconState.value` / `imgReady` / `@load` / `@error`.
 */

import { ref, nextTick, onBeforeUnmount, type Ref, type Component } from 'vue'
import type { SearchResult } from '@/modules/search'
import { useAppIcon, type IconState } from './useAppIcon'
import { ICON_CONFIG } from '@/core/config/icon'

export interface UseIconRendererOptions {
    /** 占位 Lucide 组件 (初始 + 失败兜底) */
    fallbackComponent: Component
    /** 容器选择器: 给 result.id → "[data-xxx='...'] img". 用于 nextTick naturalWidth 检测 */
    containerSelector: (id: string) => string
    /** 调试日志 tag: "[AppResultItem:img]" / "[ResultItem:img]" */
    debugTag: string
    /** 禁用日志 (生产可关) */
    silent?: boolean
}

export interface UseIconRendererReturn {
    iconState: Ref<IconState>
    imgReady: Ref<boolean>
    /** 重新触发加载. 在 result 变化时调用 */
    refresh: (result: SearchResult) => Promise<void>
    /** img 元素 @load 事件回调 */
    onImgLoad: (ev: Event) => void
    /** img 元素 @error 事件回调 */
    onImgError: (ev: Event) => void
    /** 卸载时调用, 清理 timer (composable 已挂 onBeforeUnmount, 显式调用为兼容) */
    dispose: () => void
}

export function useIconRenderer(opts: UseIconRendererOptions): UseIconRendererReturn {
    const { loadIcon } = useAppIcon()
    const iconState = ref<IconState>({ kind: 'component', value: opts.fallbackComponent })
    const imgReady = ref(false)

    /** 350ms 兜底 timer 句柄 */
    let imgFallbackTimer: ReturnType<typeof setTimeout> | null = null
    /** loadToken 防止快速切换 result.id 时旧 promise 覆盖新结果 */
    let loadToken = 0

    function log(level: 'log' | 'warn', ...args: unknown[]) {
        if (opts.silent) return
        // eslint-disable-next-line no-console
        console[level](...args)
    }

    function clearImgFallback() {
        if (imgFallbackTimer) {
            clearTimeout(imgFallbackTimer)
            imgFallbackTimer = null
        }
    }

    /**
     * 提取 src 字符串以便日志打印 (兼容 png/svg).
     */
    function srcOf(state: IconState | undefined): string | undefined {
        if (!state) return undefined
        if (state.kind === 'png' || state.kind === 'svg') return state.value
        return undefined
    }

    /**
     * imgReady 兜底 timer: 在以下场景, `<img>` 的 @load 事件可能丢失,
     * 导致 imgReady 永远 = false → opacity 0 → 看上去是空白:
     * - 虚拟列表 v-for 复用 DOM, src 字符串相同时 Chromium 短路不发 load
     * - WebView2 内部 image state machine 与 DOM patch 顺序耦合
     * - happy-dom 测试环境根本不实现 img loading
     * - 缓存命中 + 相同 IconState 引用, Vue setter 跳过更新
     *
     * 因此: 每次进入 png/svg 路径时, 启动 `ICON_CONFIG.loadFallbackMs` ms 兜底.
     * 若 @load 在这之前已触发, 提前 clearTimeout 取消兜底.
     * 若 @load 丢失, 兜底强制显示.
     */
    function scheduleImgFallback(currentId: string) {
        clearImgFallback()
        imgFallbackTimer = setTimeout(() => {
            if (!imgReady.value) {
                log('warn', `[${opts.debugTag}:img] @load timeout (${ICON_CONFIG.loadFallbackMs}ms), force ready for id=${currentId}`)
                imgReady.value = true
            }
        }, ICON_CONFIG.loadFallbackMs)
    }

    /**
     * 重新触发图标加载. 在 onMounted / result 变化时调用.
     * 关键: loadToken 防止快速切换时旧 promise 覆盖新结果.
     */
    async function refresh(result: SearchResult) {
        const myToken = ++loadToken
        clearImgFallback()
        const next = await loadIcon(result)
        if (myToken !== loadToken) {
            return
        }

        const prev = iconState.value
        const isSame =
            prev === next ||
            (prev?.kind === next.kind &&
                (prev.kind === 'component' ? prev.value === (next as any).value : prev.kind === 'monogram' ? (prev as any).letter === (next as any).letter : prev.value === (next as any).value))
        if (isSame && imgReady.value) {
            return
        }

        iconState.value = next

        if (next.kind === 'component' || next.kind === 'monogram') {
            imgReady.value = true
        } else {
            imgReady.value = false
            nextTick(() => {
                const img = document.querySelector(opts.containerSelector(result?.id ?? '')) as HTMLImageElement | null
                if (img && img.naturalWidth > 0) {
                    imgReady.value = true
                    return
                }
                scheduleImgFallback(result?.id ?? '')
            })
        }
    }

    function onImgLoad(ev: Event) {
        imgReady.value = true
        clearImgFallback()
    }

    function onImgError(_ev: Event) {
        const id = (iconState.value as any)?.id ?? ''
        log('warn',
            `[${opts.debugTag}:img] @error id=${id} ` +
            `srcHead="${srcOf(iconState.value)?.slice(0, 60) ?? 'N/A'}"`,
        )
        // 加载失败 → 降级到 Lucide 通用图标, 避免破图
        iconState.value = { kind: 'component', value: opts.fallbackComponent }
        imgReady.value = true
        clearImgFallback()
    }

    function dispose() {
        clearImgFallback()
    }

    // composable 内部自动清理, 调用方也可用 dispose 手动调用
    onBeforeUnmount(dispose)

    return { iconState, imgReady, refresh, onImgLoad, onImgError, dispose }
}
