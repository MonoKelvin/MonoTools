<script setup lang="ts">
import { computed, ref, watch, nextTick, onMounted, onBeforeUnmount } from 'vue'

export type TooltipPlacement = 'top' | 'bottom' | 'left' | 'right'

interface TooltipProps {
  visible: boolean
  title?: string
  subtitle?: string
  path?: string
  anchor?: HTMLElement | null
  mouseX?: number
  mouseY?: number
  placement?: TooltipPlacement
  offsetX?: number
  offsetY?: number
  maxWidth?: number
  sidePadding?: number
}

const props = withDefaults(defineProps<TooltipProps>(), {
  visible: false,
  title: '',
  subtitle: '',
  path: '',
  anchor: null,
  mouseX: undefined,
  mouseY: undefined,
  placement: 'bottom',
  offsetX: 0,
  offsetY: 6,
  maxWidth: 0,
  sidePadding: 20,
})

const tooltipRef = ref<HTMLElement | null>(null)
const tooltipStyle = ref<{ left: string; top: string; opacity: string; pointerEvents: string }>({
  left: '0px',
  top: '-9999px',
  opacity: '0',
  pointerEvents: 'none',
})
const actualPlacement = ref<TooltipPlacement>('bottom')
const isPositioned = ref(false)
const animationOrigin = ref<'left' | 'right' | 'top' | 'bottom' | 'default'>('default')
let resizeObserver: ResizeObserver | null = null
let rafId: number | null = null
let hideRafId: number | null = null

const hasContent = computed(() => {
  return !!(props.title || props.subtitle || props.path)
})

const showTooltip = computed(() => {
  return props.visible && hasContent.value && isPositioned.value
})

function computePosition() {
  if (!tooltipRef.value) return

  const el = tooltipRef.value
  const vw = window.innerWidth
  const vh = window.innerHeight
  const tooltipMaxWidth = props.maxWidth > 0
    ? Math.min(props.maxWidth, vw - props.sidePadding * 2)
    : vw - props.sidePadding * 2

  el.style.maxWidth = `${tooltipMaxWidth}px`

  const tooltipWidth = el.offsetWidth
  const tooltipHeight = el.offsetHeight

  if (tooltipWidth === 0 || tooltipHeight === 0) return

  let anchorLeft = 0
  let anchorTop = 0
  let anchorWidth = 0
  let anchorHeight = 0

  if (props.mouseX != null && props.mouseY != null) {
    anchorLeft = props.mouseX
    anchorTop = props.mouseY
    anchorWidth = 0
    anchorHeight = 0
  } else if (props.anchor) {
    const rect = props.anchor.getBoundingClientRect()
    anchorLeft = rect.left
    anchorTop = rect.top
    anchorWidth = rect.width
    anchorHeight = rect.height
  } else {
    return
  }

  let placement = props.placement
  let left = 0
  let top = 0

  if (placement === 'top' || placement === 'bottom') {
    const anchorCenterX = anchorLeft + anchorWidth / 2

    const spaceBelow = vh - (anchorTop + anchorHeight)
    const spaceAbove = anchorTop

    if (placement === 'bottom' && spaceBelow < tooltipHeight + props.offsetY && spaceAbove > spaceBelow) {
      placement = 'top'
    } else if (placement === 'top' && spaceAbove < tooltipHeight + props.offsetY && spaceBelow > spaceAbove) {
      placement = 'bottom'
    }

    left = anchorCenterX + props.offsetX

    if (placement === 'top') {
      top = anchorTop - tooltipHeight - props.offsetY
    } else {
      top = anchorTop + anchorHeight + props.offsetY
    }
  } else {
    const anchorCenterY = anchorTop + anchorHeight / 2

    const spaceRight = vw - (anchorLeft + anchorWidth)
    const spaceLeft = anchorLeft

    if (placement === 'right' && spaceRight < tooltipWidth + props.offsetX && spaceLeft > spaceRight) {
      placement = 'left'
    } else if (placement === 'left' && spaceLeft < tooltipWidth + props.offsetX && spaceRight > spaceLeft) {
      placement = 'right'
    }

    top = anchorCenterY + props.offsetY

    if (placement === 'left') {
      left = anchorLeft - tooltipWidth - props.offsetX
    } else {
      left = anchorLeft + anchorWidth + props.offsetX
    }
  }

  const isMouseMode = props.mouseX != null && props.mouseY != null
  const hasHCenter = (placement === 'top' || placement === 'bottom') && !isMouseMode
  const hasVCenter = placement === 'left' || placement === 'right'

  let realLeft: number
  let realTop: number

  if (hasHCenter) {
    realLeft = left - tooltipWidth / 2
  } else {
    realLeft = left
  }

  if (hasVCenter) {
    realTop = top - tooltipHeight / 2
  } else {
    realTop = top
  }

  let realRight = realLeft + tooltipWidth
  let realBottom = realTop + tooltipHeight

  if (realLeft < props.sidePadding) {
    realLeft = props.sidePadding
    realRight = realLeft + tooltipWidth
  }
  if (realRight > vw - props.sidePadding) {
    realRight = vw - props.sidePadding
    realLeft = realRight - tooltipWidth
  }
  if (realTop < props.sidePadding) {
    realTop = props.sidePadding
    realBottom = realTop + tooltipHeight
  }
  if (realBottom > vh - props.sidePadding) {
    realBottom = vh - props.sidePadding
    realTop = realBottom - tooltipHeight
  }

  if (hasHCenter) {
    left = realLeft + tooltipWidth / 2
  } else {
    left = realLeft
  }

  if (hasVCenter) {
    top = realTop + tooltipHeight / 2
  } else {
    top = realTop
  }

  actualPlacement.value = placement
  isPositioned.value = true

  const centerX = realLeft + tooltipWidth / 2
  const centerY = realTop + tooltipHeight / 2
  const leftDist = centerX
  const rightDist = vw - centerX
  const topDist = centerY
  const bottomDist = vh - centerY

  const thresholdX = vw * 0.25
  const thresholdY = vh * 0.25

  let origin: 'left' | 'right' | 'top' | 'bottom' | 'default' = 'default'

  const minDist = Math.min(leftDist, rightDist, topDist, bottomDist)

  if (minDist < thresholdX || minDist < thresholdY) {
    if (minDist === leftDist && leftDist < thresholdX) {
      origin = 'left'
    } else if (minDist === rightDist && rightDist < thresholdX) {
      origin = 'right'
    } else if (minDist === topDist && topDist < thresholdY) {
      origin = 'top'
    } else if (minDist === bottomDist && bottomDist < thresholdY) {
      origin = 'bottom'
    }
  }

  animationOrigin.value = origin

  tooltipStyle.value = {
    left: `${left}px`,
    top: `${top}px`,
    opacity: '1',
    pointerEvents: 'none',
  }
}

function scheduleShow() {
  if (!props.visible || !hasContent.value) return

  isPositioned.value = false

  if (hideRafId) {
    cancelAnimationFrame(hideRafId)
    hideRafId = null
  }
  if (rafId) cancelAnimationFrame(rafId)

  nextTick(() => {
    computePosition()
    rafId = requestAnimationFrame(() => {
      computePosition()
    })
  })
}

function hideTooltip() {
  isPositioned.value = false
  if (rafId) {
    cancelAnimationFrame(rafId)
    rafId = null
  }
  tooltipStyle.value = {
    left: '0px',
    top: '-9999px',
    opacity: '0',
    pointerEvents: 'none',
  }
}

function setupResizeObserver() {
  if (!tooltipRef.value || resizeObserver) return

  resizeObserver = new ResizeObserver(() => {
    if (props.visible && hasContent.value && isPositioned.value) {
      computePosition()
    }
  })
  resizeObserver.observe(tooltipRef.value)
}

watch(() => props.visible, (val) => {
  if (val) {
    scheduleShow()
  } else {
    hideTooltip()
  }
}, { flush: 'post' })

watch(() => [props.title, props.subtitle, props.path], () => {
  if (props.visible && hasContent.value) {
    scheduleShow()
  }
}, { flush: 'post' })

watch(() => [props.mouseX, props.mouseY], () => {
  if (props.visible && props.mouseX != null && props.mouseY != null && isPositioned.value) {
    computePosition()
  }
}, { flush: 'post' })

watch(() => props.placement, () => {
  if (props.visible) {
    scheduleShow()
  }
}, { flush: 'post' })

watch(() => props.anchor, () => {
  if (props.visible && props.anchor) {
    scheduleShow()
  }
}, { flush: 'post' })

function handleScroll() {
  if (props.visible && props.anchor && isPositioned.value) {
    computePosition()
  }
}

function handleResize() {
  if (props.visible && isPositioned.value) {
    scheduleShow()
  }
}

onMounted(() => {
  if (props.visible && hasContent.value) {
    scheduleShow()
  } else {
    hideTooltip()
  }
  setupResizeObserver()
  window.addEventListener('resize', handleResize)
  window.addEventListener('scroll', handleScroll, true)
})

onBeforeUnmount(() => {
  if (resizeObserver) {
    resizeObserver.disconnect()
    resizeObserver = null
  }
  if (rafId) {
    cancelAnimationFrame(rafId)
    rafId = null
  }
  if (hideRafId) {
    cancelAnimationFrame(hideRafId)
    hideRafId = null
  }
  window.removeEventListener('resize', handleResize)
  window.removeEventListener('scroll', handleScroll, true)
})
</script>

<template>
  <Teleport to="body">
    <div
      ref="tooltipRef"
      class="mt-tooltip"
      :class="[
        `mt-tooltip--${actualPlacement}`,
        `mt-tooltip--from-${animationOrigin}`,
        { 'mt-tooltip--mouse': mouseX != null && mouseY != null },
        { 'mt-tooltip--visible': showTooltip }
      ]"
      :style="tooltipStyle"
    >
      <div v-if="title" class="mt-tooltip__title">{{ title }}</div>
      <div v-if="subtitle" class="mt-tooltip__subtitle">{{ subtitle }}</div>
      <div v-if="path" class="mt-tooltip__path">{{ path }}</div>
    </div>
  </Teleport>
</template>

<style>
.mt-tooltip {
  position: fixed;
  z-index: 9999;
  pointer-events: none;
  user-select: none;
  min-width: 100px;
  width: max-content;
  max-width: 300px;
  padding: 6px 10px;
  background: var(--glass-bg-soft);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-md);
  box-shadow:
    0 1px 0 rgba(255, 255, 255, 0.05) inset,
    0 8px 24px rgba(0, 0, 0, 0.5),
    0 2px 8px rgba(0, 0, 0, 0.35);
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  text-align: left;
  opacity: 0;
  --anim-x: 0px;
  --anim-y: 2px;
  --base-transform: translateX(0);
  transition: opacity 120ms var(--ease-out), transform 120ms var(--ease-out);
  transform: var(--base-transform) translate(var(--anim-x), var(--anim-y));
}

.mt-tooltip--visible {
  opacity: 1;
  transform: var(--base-transform) translate(0, 0);
}

.mt-tooltip--top,
.mt-tooltip--bottom {
  --base-transform: translateX(-50%);
}

.mt-tooltip--left,
.mt-tooltip--right {
  --base-transform: translateY(-50%);
}

.mt-tooltip--mouse {
  --base-transform: translateX(0);
}

/* Default: slide up */
.mt-tooltip--from-default {
  --anim-x: 0px;
  --anim-y: 2px;
}

.mt-tooltip--top.mt-tooltip--from-default,
.mt-tooltip--bottom.mt-tooltip--from-default {
  --anim-x: 0px;
  --anim-y: 2px;
}

/* From left: slide right */
.mt-tooltip--from-left {
  --anim-x: -8px;
  --anim-y: 0px;
}

.mt-tooltip--top.mt-tooltip--from-left,
.mt-tooltip--bottom.mt-tooltip--from-left {
  --anim-x: -8px;
  --anim-y: 0px;
}

/* From right: slide left */
.mt-tooltip--from-right {
  --anim-x: 8px;
  --anim-y: 0px;
}

.mt-tooltip--top.mt-tooltip--from-right,
.mt-tooltip--bottom.mt-tooltip--from-right {
  --anim-x: 8px;
  --anim-y: 0px;
}

/* From top: slide down */
.mt-tooltip--from-top {
  --anim-x: 0px;
  --anim-y: -8px;
}

.mt-tooltip--top.mt-tooltip--from-top,
.mt-tooltip--bottom.mt-tooltip--from-top {
  --anim-x: 0px;
  --anim-y: -8px;
}

/* From bottom: slide up */
.mt-tooltip--from-bottom {
  --anim-x: 0px;
  --anim-y: 8px;
}

.mt-tooltip--top.mt-tooltip--from-bottom,
.mt-tooltip--bottom.mt-tooltip--from-bottom {
  --anim-x: 0px;
  --anim-y: 8px;
}

.mt-tooltip__title {
  font-size: 12.5px;
  font-weight: 600;
  line-height: 1.4;
  color: var(--text-primary);
  letter-spacing: 0.01em;
  margin-bottom: 3px;
  word-break: break-word;
  overflow-wrap: break-word;
}

.mt-tooltip__subtitle {
  font-size: 11px;
  font-weight: 500;
  line-height: 1.4;
  color: var(--text-secondary);
  margin-bottom: 2px;
  word-break: break-word;
  overflow-wrap: break-word;
}

.mt-tooltip__path {
  font-size: 10.5px;
  font-weight: 500;
  line-height: 1.4;
  color: var(--text-tertiary);
  font-family: var(--font-mono);
  word-break: break-all;
  overflow-wrap: break-word;
}

.mt-tooltip__subtitle:last-child,
.mt-tooltip__path:last-child {
  margin-bottom: 0;
}
</style>
