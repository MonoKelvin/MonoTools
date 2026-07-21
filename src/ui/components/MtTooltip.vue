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
const tooltipStyle = ref<{ left: string; top: string }>({ left: '0px', top: '-9999px' })
const actualPlacement = ref<TooltipPlacement>('bottom')
const maxWidthStyle = ref('200px')

function updatePosition() {
  if (!props.visible || !tooltipRef.value) return

  const vw = window.innerWidth
  const vh = window.innerHeight
  const tooltipMaxWidth = props.maxWidth > 0
    ? Math.min(props.maxWidth, vw - props.sidePadding * 2)
    : vw - props.sidePadding * 2

  tooltipRef.style.maxWidth = `${tooltipMaxWidth}px`
  maxWidthStyle.value = `${tooltipMaxWidth}px`

  const tooltipWidth = tooltipRef.offsetWidth
  const tooltipHeight = tooltipRef.offsetHeight

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

    const halfWidth = tooltipWidth / 2
    if (left - halfWidth < props.sidePadding) {
      left = props.sidePadding + halfWidth
    }
    if (left + halfWidth > vw - props.sidePadding) {
      left = vw - props.sidePadding - halfWidth
    }

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

    const halfHeight = tooltipHeight / 2
    if (top - halfHeight < props.sidePadding) {
      top = props.sidePadding + halfHeight
    }
    if (top + halfHeight > vh - props.sidePadding) {
      top = vh - props.sidePadding - halfHeight
    }

    if (placement === 'left') {
      left = anchorLeft - tooltipWidth - props.offsetX
    } else {
      left = anchorLeft + anchorWidth + props.offsetX
    }
  }

  actualPlacement.value = placement

  tooltipStyle.value = {
    left: `${left}px`,
    top: `${top}px`,
  }
}

function scheduleUpdate() {
  if (!props.visible) return

  nextTick(() => {
    updatePosition()
    requestAnimationFrame(() => {
      updatePosition()
    })
  })
}

watch(() => props.visible, (val) => {
  if (val) {
    scheduleUpdate()
  }
})

watch(() => [props.mouseX, props.mouseY], () => {
  if (props.visible && props.mouseX != null && props.mouseY != null) {
    updatePosition()
  }
}, { deep: false })

watch(() => props.placement, () => {
  if (props.visible) {
    scheduleUpdate()
  }
})

watch(() => [props.offsetX, props.offsetY, props.maxWidth, props.sidePadding], () => {
  if (props.visible) {
    updatePosition()
  }
}, { deep: false })

function handleScroll() {
  if (props.visible && props.anchor) {
    updatePosition()
  }
}

function handleResize() {
  if (props.visible) {
    updatePosition()
  }
}

onMounted(() => {
  if (props.visible) {
    scheduleUpdate()
  }
  window.addEventListener('resize', handleResize)
  window.addEventListener('scroll', handleScroll, true)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', handleResize)
  window.removeEventListener('scroll', handleScroll, true)
})

const hasContent = computed(() => {
  return !!(props.title || props.subtitle || props.path)
})
</script>

<template>
  <Teleport to="body">
    <Transition name="mt-tooltip-fade">
      <div
        v-if="visible && hasContent"
        ref="tooltipRef"
        class="mt-tooltip"
        :class="[
          `mt-tooltip--${actualPlacement}`,
          { 'mt-tooltip--mouse': mouseX != null && mouseY != null }
        ]"
        :style="tooltipStyle"
      >
        <div v-if="title" class="mt-tooltip__title">{{ title }}</div>
        <div v-if="subtitle" class="mt-tooltip__subtitle">{{ subtitle }}</div>
        <div v-if="path" class="mt-tooltip__path">{{ path }}</div>
      </div>
    </Transition>
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
}

.mt-tooltip--top,
.mt-tooltip--bottom {
  transform: translateX(-50%);
}

.mt-tooltip--left,
.mt-tooltip--right {
  transform: none;
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

.mt-tooltip-fade-enter-active,
.mt-tooltip-fade-leave-active {
  transition: opacity 120ms var(--ease-out), transform 120ms var(--ease-out);
}

.mt-tooltip-fade-enter-from,
.mt-tooltip-fade-leave-to {
  opacity: 0;
}

.mt-tooltip--top.mt-tooltip-fade-enter-from,
.mt-tooltip--top.mt-tooltip-fade-leave-to {
  transform: translateX(-50%) translateY(-2px);
}

.mt-tooltip--bottom.mt-tooltip-fade-enter-from,
.mt-tooltip--bottom.mt-tooltip-fade-leave-to {
  transform: translateX(-50%) translateY(2px);
}

.mt-tooltip--left.mt-tooltip-fade-enter-from,
.mt-tooltip--left.mt-tooltip-fade-leave-to {
  transform: translateX(-2px);
}

.mt-tooltip--right.mt-tooltip-fade-enter-from,
.mt-tooltip--right.mt-tooltip-fade-leave-to {
  transform: translateX(2px);
}
</style>
