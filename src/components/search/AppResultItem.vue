<script setup lang="ts">
/**
 * AppResultItem —— 专门用于"应用"类搜索结果.
 *
 * 与通用 ResultItem 的差异:
 * - **不显示副标题 (路径)**: 应用结果路径属于内部细节, 不向用户暴露.
 * - **图标优先**: 图标分辨率更高 (28x28), 占视觉重点.
 * - **三态图标**: 静态 SVG (立即) / 后端 PNG (异步) / Lucide 通用兜底.
 * - **更大标题字号 (14px / 600)**: 强化应用名识别度.
 * - **省略号截断**: 标题超出时自动 ... 显示.
 * - **内置玻璃风格工具提示**: 通过 v-tooltip 显示完整应用名.
 * - **懒加载图片**: 首屏外的图标使用 loading="lazy", 减少初始带宽.
 *
 * 与 ResultItem 共享选中态 / 悬停态 / 快捷键 ↵ 行为, 可直接在列表中替换.
 */
import { computed, onMounted, ref, watch } from 'vue'
import { AppWindow } from '@lucide/vue'
import type { SearchResult } from '@/types/search'
import { useAppIcon, type IconState } from '@/composables/useAppIcon'

const props = defineProps<{
  result: SearchResult
  active?: boolean
  index: number
}>()

const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
  (e: 'mouseover', i: number): void
  (e: 'contextmenu', ev: MouseEvent, item: SearchResult): void
}>()

const { loadIcon } = useAppIcon()
/**
 * 初始即用 Lucide 通用应用图标, 避免图标在加载完成前的"空白"闪烁.
 * PNG / SVG 加载完成后自动切换为真实图标, 实现无感替换.
 */
const iconState = ref<IconState>({ kind: 'component', value: AppWindow })
const imgReady = ref(false)

/**
 * 工具提示: 选中态(active)下不显示, 避免键盘导航时 tooltip 跟随高亮
 * 持续闪现造成视觉干扰. 未选中且鼠标悬停时才显示完整应用名.
 */
const tooltipOptions = computed(() => {
  if (props.active) return undefined
  return {
    value: props.result.title || '',
    class: 'app-tooltip',
    showDelay: 360,
    fitContent: true,
    position: 'bottom' as const,
    autoHide: true,
    escape: true,
  }
})

let loadToken = 0

async function refreshIcon() {
  // 用 token 防止 race: 快速切换结果时, 旧请求不应覆盖新结果
  const myToken = ++loadToken
  const next = await loadIcon(props.result)
  if (myToken !== loadToken) return
  iconState.value = next
  // png / svg 用 <img> 加载, 完成后通过 @load 标记 ready
  if (next.kind === 'component') {
    imgReady.value = true
  } else {
    imgReady.value = false
  }
}

onMounted(refreshIcon)
watch(() => props.result?.id, refreshIcon)

function onImgLoad() { imgReady.value = true }
function onImgError() {
  // 图片加载失败, 降级到 Lucide 通用图标, 避免破图
  iconState.value = { kind: 'component', value: AppWindow }
  imgReady.value = true
}

function onClick() { emit('select', props.result) }
function onEnter() { emit('mouseover', props.index) }
function onContextMenu(e: MouseEvent) { emit('contextmenu', e, props.result) }
</script>

<template>
  <div
    :class="['app-result-item', { 'app-result-item--active': active }]"
    @click="onClick"
    @mouseenter="onEnter"
    @contextmenu.prevent="onContextMenu"
    v-tooltip="tooltipOptions"
  >
    <div class="app-result-item__icon">
      <img
        v-if="iconState.kind === 'svg' || iconState.kind === 'png'"
        :src="iconState.value"
        class="app-result-item__img"
        :class="{ 'app-result-item__img--ready': imgReady }"
        @load="onImgLoad"
        @error="onImgError"
        loading="lazy"
        decoding="async"
        draggable="false"
        alt=""
      />
      <component
        v-else
        :is="iconState.value"
        :size="18"
        :stroke-width="1.7"
        class="app-result-item__lucide"
      />
    </div>

    <div class="app-result-item__title">{{ result.title }}</div>

    <div class="app-result-item__meta">
      <span class="kbd">↵</span>
    </div>
  </div>
</template>

<style scoped>
.app-result-item {
  display: flex;
  align-items: center;
  gap: var(--sp-4);
  padding: 7px 12px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition:
    background var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out);
  user-select: none;
  background: transparent;
  position: relative;
  overflow: hidden;
  border: none;
}

.app-result-item:hover {
  background: var(--list-hover-bg);
}

.app-result-item--active {
  background: var(--list-selected-bg);
}

.app-result-item--active::before {
  content: '';
  position: absolute;
  left: 0;
  top: 8px;
  bottom: 8px;
  width: 2px;
  border-radius: 2px;
  background: var(--accent);
  box-shadow: 0 0 8px var(--accent-glow);
  animation: app-active-bar-in 280ms var(--ease-spring);
}

@keyframes app-active-bar-in {
  0% { transform: scaleY(0.4); opacity: 0; }
  100% { transform: scaleY(1); opacity: 1; }
}

.app-result-item__icon {
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 7px;
  background: transparent;
  color: var(--text-tertiary);
  overflow: hidden;
  position: relative;
  transition:
    color var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
}

.app-result-item:hover .app-result-item__icon {
  color: var(--text-secondary);
  transform: scale(1.04);
}

.app-result-item--active .app-result-item__icon {
  color: var(--accent);
  filter: drop-shadow(0 0 6px var(--accent-glow));
  transform: scale(1.06);
}

.app-result-item__img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  padding: 2px;
  opacity: 0;
  transition: opacity 220ms var(--ease-out);
  pointer-events: none;
}

.app-result-item__img--ready {
  opacity: 1;
}

.app-result-item__lucide {
  pointer-events: none;
}

.app-result-item__title {
  flex: 1;
  min-width: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.35;
  letter-spacing: -0.005em;
  text-rendering: optimizeLegibility;
  transition: color var(--dur-fast) var(--ease-out);
}

.app-result-item__meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  margin-left: auto;
  opacity: 0;
  transform: translateX(6px);
  transition:
    opacity var(--dur-normal) var(--ease-out),
    transform var(--dur-normal) var(--ease-out);
}

.app-result-item:hover .app-result-item__meta,
.app-result-item--active .app-result-item__meta {
  opacity: 1;
  transform: translateX(0);
}

.kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--text-muted);
  background: transparent;
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  line-height: 1;
  transition:
    color var(--dur-fast) var(--ease-out),
    border-color var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out);
}

.app-result-item--active .kbd {
  background: var(--accent-soft);
  border-color: var(--accent);
  color: var(--accent);
}
</style>
