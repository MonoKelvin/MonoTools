/**
 * TooltipRenderer — 全局单例渲染器
 *
 * 职责：
 * - 统一管理所有 tooltip 的 DOM、显示/隐藏
 * - 批量监听全局 scroll/resize 事件（只监听一次）
 * - 统一的位置计算和动画管理
 *
 * 使用方式：
 * - MtTooltip 组件通过 register() 注册到渲染器
 * - 渲染器负责所有实际的 DOM 操作和定位
 */

export type TooltipPlacement = 'top' | 'bottom' | 'left' | 'right'

export type TooltipEntry = {
    el: HTMLElement
    text: string
    placement: TooltipPlacement
    offsetX: number
    offsetY: number
    delay: number
    maxWidth: number
    sidePadding: number
}

export type TooltipRenderState = {
    visible: boolean
    text: string
    placement: TooltipPlacement
    x: number
    y: number
    opacity: number
    animationOrigin: 'left' | 'right' | 'top' | 'bottom' | 'default'
}

export class TooltipRenderer {
    /** 注册的 tooltip 实例 map: el -> entry */
    private entries = new Map<HTMLElement, TooltipEntry>()
    /** 当前显示的 tooltip 实例 */
    private activeEl: HTMLElement | null = null
    /** 当前显示的 tooltip 状态 */
    private state: TooltipRenderState = {
        visible: false,
        text: '',
        placement: 'bottom',
        x: 0,
        y: 0,
        opacity: 0,
        animationOrigin: 'default',
    }
    /** 延迟 timer */
    private showTimer: ReturnType<typeof setTimeout> | null = null
    /** 是否需要重新定位 */
    private needsReposition = false
    /** reposition timer for rAF batching */
    private repositionTimer: number | null = null
    /** 全局 scroll/resize 监听器 */
    private boundOnScroll: (() => void) | null = null
    private boundOnResize: (() => void) | null = null
    /** 渲染容器 DOM */
    private container: HTMLElement | null = null
    /** tooltip DOM */
    private tooltipEl: HTMLElement | null = null
    /** 文本 DOM */
    private textEl: HTMLElement | null = null
    private initialized = false

    /** 获取当前激活的 entry */
    getActiveEntry(): TooltipEntry | null {
        return this.activeEl ? this.entries.get(this.activeEl) ?? null : null
    }

    /** 获取当前显示状态 */
    getState(): TooltipRenderState {
        return this.state
    }

    /** 注册一个 tooltip */
    register(el: HTMLElement, entry: Omit<TooltipEntry, 'el'>) {
        this.entries.set(el, { ...entry, el })
    }

    /** 更新 tooltip 配置 */
    update(el: HTMLElement, entry: Partial<Omit<TooltipEntry, 'el'>>) {
        const existing = this.entries.get(el)
        if (!existing) return

        this.entries.set(el, {
            ...existing,
            ...entry,
            el,
        })

        // 如果当前显示的是这个 tooltip，立即更新文本
        if (this.activeEl === el) {
            const updated = this.entries.get(el)!
            this.updateDOMText(updated.text)
        }
    }

    /** 取消注册 */
    unregister(el: HTMLElement) {
        // 如果取消注册的是当前激活的，隐藏它
        if (this.activeEl === el) {
            this.hide()
        }
        this.entries.delete(el)
    }

    /** 显示 tooltip */
    show(el: HTMLElement) {
        const entry = this.entries.get(el)
        if (!entry || !entry.text) return

        // 清除之前的 timer
        if (this.showTimer) {
            clearTimeout(this.showTimer)
            this.showTimer = null
        }

        // 如果显示的是同一个，直接返回
        if (this.activeEl === el && this.state.visible) return

        // 先隐藏之前的
        if (this.activeEl && this.activeEl !== el) {
            this.hide()
        }

        const delay = entry.delay

        this.showTimer = setTimeout(() => {
            if (this.entries.get(el) !== entry) return

            this.activeEl = el
            this.needsReposition = true

            // 初始化全局监听（如果还没初始化）
            this.ensureInitialized()

            // 创建/更新 DOM
            this.updateDOMText(entry.text)

            // 需要两帧：第一帧插入DOM，第二帧获取尺寸
            requestAnimationFrame(() => {
                if (this.activeEl !== el) return
                this.ensureTooltipVisible()

                requestAnimationFrame(() => {
                    if (this.activeEl === el) {
                        this.reposition(el, entry)
                    }
                })
            })

            this.showTimer = null
        }, delay)
    }

    /** 隐藏 tooltip */
    hide() {
        if (!this.state.visible && !this.tooltipEl) return

        this.state.visible = false
        this.state.opacity = 0

        // 更新 DOM
        if (this.tooltipEl) {
            this.tooltipEl.style.opacity = '0'
            this.tooltipEl.style.pointerEvents = 'none'
        }

        this.activeEl = null
        this.needsReposition = false

        if (this.showTimer) {
            clearTimeout(this.showTimer)
            this.showTimer = null
        }
    }

    /** 更新文本内容 */
    private updateDOMText(text: string) {
        this.state.text = text
        if (this.textEl) {
            this.textEl.textContent = text
        }
    }

    /** 确保 tooltip DOM 存在且可见 */
    private ensureTooltipVisible() {
        const container = this.getContainer()
        if (!container) return

        this.container = container

        if (!this.tooltipEl) {
            // 创建 tooltip DOM
            this.tooltipEl = document.createElement('div')
            this.tooltipEl.className = 'mt-tooltip'
            this.tooltipEl.style.cssText = `
                position: fixed;
                z-index: 9999;
                pointer-events: none;
                user-select: none;
                min-width: 100px;
                padding: 6px 10px;
                background: var(--glass-bg-soft, #2d2d44);
                border: 1px solid var(--glass-border, rgba(255,255,255,0.08));
                border-radius: var(--radius-md, 8px);
                box-shadow:
                    0 1px 0 rgba(255, 255, 255, 0.05) inset,
                    0 8px 24px rgba(0, 0, 0, 0.5),
                    0 2px 8px rgba(0, 0, 0, 0.35);
                backdrop-filter: var(--glass-blur, blur(12px));
                -webkit-backdrop-filter: var(--glass-blur, blur(12px));
                text-align: left;
                opacity: 0;
                --anim-x: 0px;
                --anim-y: 2px;
                --base-transform: translateX(0);
                transition: opacity 160ms cubic-bezier(0.16, 1, 0.3, 1),
                            transform 160ms cubic-bezier(0.16, 1, 0.3, 1);
                transform: var(--base-transform) translate(var(--anim-x), var(--anim-y));
                will-change: transform, opacity;
                visibility: hidden;
            `
            container.appendChild(this.tooltipEl)

            // 创建文本 DOM
            this.textEl = document.createElement('div')
            this.textEl.className = 'mt-tooltip__text'
            this.textEl.style.cssText = `
                font-size: 12px;
                font-weight: 500;
                line-height: 1.4;
                color: var(--text-primary, #e4e4e7);
                letter-spacing: 0.01em;
                word-break: break-all;
                overflow-wrap: break-word;
            `
            this.tooltipEl.appendChild(this.textEl)
        }
    }

    /** 重新定位所有活跃的 tooltip */
    reposition(el: HTMLElement, entry: TooltipEntry) {
        const container = this.container
        if (!container || !this.tooltipEl) return

        const vw = window.innerWidth
        const vh = window.innerHeight

        const rect = el.getBoundingClientRect()
        const anchorLeft = Math.max(0, rect.left)
        const anchorTop = Math.max(0, rect.top)
        const anchorWidth = rect.width
        const anchorHeight = rect.height

        // 计算最大宽度：窗口宽度 - 2*sidePadding
        const sidePadding = entry.sidePadding
        const maxWidth = vw - sidePadding * 2

        // 应用 maxWidth 限制并允许换行
        this.tooltipEl.style.width = 'auto'
        this.tooltipEl.style.maxWidth = `${maxWidth}px`
        if (this.textEl) this.textEl.style.whiteSpace = 'normal'

        // 强制重排，获取应用限制后的实际宽度
        void this.tooltipEl.offsetHeight
        const tooltipWidth = this.tooltipEl.offsetWidth
        const tooltipHeight = this.tooltipEl.offsetHeight

        if (tooltipWidth === 0 || tooltipHeight === 0) {
            requestAnimationFrame(() => {
                this.reposition(el, entry)
            })
            return
        }

        let placement = entry.placement
        let left = 0
        let top = 0

        // 基础定位
        if (placement === 'top' || placement === 'bottom') {
            const anchorCenterX = anchorLeft + anchorWidth / 2

            const spaceBelow = vh - (anchorTop + anchorHeight)
            const spaceAbove = anchorTop

            if (placement === 'bottom' && spaceBelow < tooltipHeight + entry.offsetY && spaceAbove > spaceBelow) {
                placement = 'top'
            } else if (placement === 'top' && spaceAbove < tooltipHeight + entry.offsetY && spaceBelow > spaceAbove) {
                placement = 'bottom'
            }

            left = anchorCenterX + entry.offsetX

            if (placement === 'top') {
                top = anchorTop - tooltipHeight - entry.offsetY
            } else {
                top = anchorTop + anchorHeight + entry.offsetY
            }
        } else {
            const anchorCenterY = anchorTop + anchorHeight / 2

            const spaceRight = vw - (anchorLeft + anchorWidth)
            const spaceLeft = anchorLeft

            if (placement === 'right' && spaceRight < tooltipWidth + entry.offsetX && spaceLeft > spaceRight) {
                placement = 'left'
            } else if (placement === 'left' && spaceLeft < tooltipWidth + entry.offsetX && spaceRight > spaceLeft) {
                placement = 'right'
            }

            top = anchorCenterY + entry.offsetY

            if (placement === 'left') {
                left = anchorLeft - tooltipWidth - entry.offsetX
            } else {
                left = anchorLeft + anchorWidth + entry.offsetX
            }
        }

        // 水平/垂直居中
        const hasHCenter = placement === 'top' || placement === 'bottom'
        const hasVCenter = placement === 'left' || placement === 'right'

        let realLeft = hasHCenter ? left - tooltipWidth / 2 : left
        let realTop = hasVCenter ? top - tooltipHeight / 2 : top

        // 严格边界约束（clamp）
        const minLeft = sidePadding
        const maxLeft = vw - sidePadding - tooltipWidth
        const minTop = sidePadding
        const maxTop = vh - sidePadding - tooltipHeight

        realLeft = Math.min(maxLeft, Math.max(minLeft, realLeft))
        realTop = Math.min(maxTop, Math.max(minTop, realTop))

        // 计算动画原点
        const centerX = realLeft + tooltipWidth / 2
        const centerY = realTop + tooltipHeight / 2
        const leftDist = centerX
        const rightDist = vw - centerX
        const topDist = centerY
        const bottomDist = vh - centerY

        const thresholdX = vw * 0.25
        const thresholdY = vh * 0.25
        const minDist = Math.min(leftDist, rightDist, topDist, bottomDist)

        let animationOrigin: 'left' | 'right' | 'top' | 'bottom' | 'default' = 'default'
        if (minDist < thresholdX || minDist < thresholdY) {
            if (minDist === leftDist && leftDist < thresholdX) animationOrigin = 'left'
            else if (minDist === rightDist && rightDist < thresholdX) animationOrigin = 'right'
            else if (minDist === topDist && topDist < thresholdY) animationOrigin = 'top'
            else if (minDist === bottomDist && bottomDist < thresholdY) animationOrigin = 'bottom'
        }

        // 更新 CSS 类
        const cls = this.tooltipEl.className
        const baseCls = cls.replace(/mt-tooltip--[a-z]+/g, '').trim()
        this.tooltipEl.className = `${baseCls} mt-tooltip--${placement} mt-tooltip--from-${animationOrigin}`.trim()

        // 更新位置并显示 - 直接使用 realLeft/realTop
        this.tooltipEl.style.left = `${realLeft}px`
        this.tooltipEl.style.top = `${realTop}px`
        this.tooltipEl.style.opacity = '1'
        this.tooltipEl.style.visibility = 'visible'

        // 更新状态
        this.state.placement = placement
        this.state.x = realLeft
        this.state.y = realTop
        this.state.opacity = 1
        this.state.animationOrigin = animationOrigin
        this.state.visible = true
        this.needsReposition = false
    }

    /** 获取或创建全局容器 */
    private getContainer(): HTMLElement {
        if (this.container) return this.container

        let container = document.getElementById('mt-tooltip-container')
        if (!container) {
            container = document.createElement('div')
            container.id = 'mt-tooltip-container'
            container.style.cssText = `
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        pointer-events: none;
      `
            document.body.appendChild(container)
            this.container = container
        }
        return container
    }

    /** 确保全局监听已初始化 */
    private ensureInitialized() {
        if (this.initialized) return
        this.initialized = true

        this.boundOnScroll = () => {
            this.needsReposition = true
            this.scheduleReposition()
        }
        this.boundOnResize = () => {
            this.needsReposition = true
            this.scheduleReposition()
        }

        window.addEventListener('scroll', this.boundOnScroll, true)
        window.addEventListener('resize', this.boundOnResize)
    }

    /** 批量重新定位 */
    private scheduleReposition() {
        if (this.repositionTimer) return
        this.repositionTimer = requestAnimationFrame(() => {
            if (this.activeEl && this.needsReposition && this.tooltipEl) {
                const entry = this.entries.get(this.activeEl)
                if (entry) {
                    this.reposition(this.activeEl, entry)
                }
            }
            this.repositionTimer = null
        })
    }

    /** 清理 */
    destroy() {
        this.hide()
        this.entries.clear()

        if (this.repositionTimer) {
            cancelAnimationFrame(this.repositionTimer)
            this.repositionTimer = null
        }

        if (this.boundOnScroll) {
            window.removeEventListener('scroll', this.boundOnScroll)
            this.boundOnScroll = null
        }
        if (this.boundOnResize) {
            window.removeEventListener('resize', this.boundOnResize)
            this.boundOnResize = null
        }

        const container = document.getElementById('mt-tooltip-container')
        if (container) {
            container.remove()
            this.container = null
        }
        this.tooltipEl = null
        this.textEl = null

        this.initialized = false
    }
}

// 全局单例
export const tooltipRenderer = new TooltipRenderer()
