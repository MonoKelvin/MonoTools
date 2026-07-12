<script setup lang="ts">
/**
 * AppResultItem —— 专门用于"应用"类搜索结果.
 *
 * 与通用 ResultItem 的差异:
 * - **不显示副标题 (路径)**: 应用结果路径属于内部细节, 不向用户暴露.
 * - **图标优先**: 图标分辨率更高 (28x28), 占视觉重点.
 * - **三态图标**: 静态 SVG (立即) / 后端 PNG (异步) / Lucide 通用兜底 / monogram 兜底.
 * - **更大标题字号 (14px / 600)**: 强化应用名识别度.
 * - **省略号截断**: 标题超出时自动 ... 显示.
 * - **内置玻璃风格工具提示**: 通过 v-tooltip 显示完整应用名.
 *
 * 与 ResultItem 共享选中态 / 悬停态 / 快捷键 ↵ 行为, 可直接在列表中替换.
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { AppWindow, CornerDownLeft } from '@lucide/vue'
import type { SearchResult } from '@/types/search'
import { useAppIcon, type IconState } from '@/composables/useAppIcon'

const props = defineProps<{
  result: SearchResult
  active?: boolean
  index: number
}>()

// 事件全部由父容器 (VirtualGroupedResults 的行 div) 统一处理, 这里只做展示.
// 仅声明 emit 留作扩展点: 右键菜单等场景需要时再使用.
const emit = defineEmits<{
  (e: 'select', item: SearchResult): void
  (e: 'open', item: SearchResult): void
  (e: 'contextmenu', ev: MouseEvent, item: SearchResult): void
}>()
// 抑制 "声明但未使用" 警告 —— 保留 emit 以便父级透传扩展事件.
void emit

const { loadIcon } = useAppIcon()
/**
 * 初始即用 Lucide 通用应用图标, 避免图标在加载完成前的"空白"闪烁.
 * PNG / SVG / monogram 加载完成后自动切换为真实图标, 实现无感替换.
 */
const iconState = ref<IconState>({ kind: 'component', value: AppWindow })
const imgReady = ref(false)

/**
 * imgReady 兜底 timer: 在以下场景, `<img>` 的 @load 事件可能丢失,
 * 导致 imgReady 永远 = false → opacity 0 → 看上去是空白:
 * - 虚拟列表 v-for 复用 DOM, src 字符串相同时 Chromium 短路不发 load
 * - WebView2 内部 image state machine 与 DOM patch 顺序耦合
 * - happy-dom 测试环境根本不实现 img loading
 * - 缓存命中 + 相同 IconState 引用, Vue setter 跳过更新
 *
 * 因此: 每次进入 png/svg 路径时, 启动 350ms 兜底 timer.
 * 若 @load 在这之前已触发, 提前 clearTimeout 取消兜底.
 * 若 @load 丢失, 兜底强制显示 (浏览器解码失败也会以 broken image 形式出现,
 * 至少 opacity = 1 让用户知道有东西在尝试渲染).
 */
let imgFallbackTimer: ReturnType<typeof setTimeout> | null = null
function clearImgFallback() {
  if (imgFallbackTimer) {
    clearTimeout(imgFallbackTimer)
    imgFallbackTimer = null
  }
}
function scheduleImgFallback() {
  clearImgFallback()
  imgFallbackTimer = setTimeout(() => {
    if (!imgReady.value) {
      // eslint-disable-next-line no-console
      console.warn(
        `[AppResultItem:img] @load timeout (350ms), force ready for id=${props.result?.id}`,
      )
      imgReady.value = true
    }
  }, 350)
}

/** 提取 src 字符串以便日志打印 (兼容 png/svg). */
function srcOf(state: IconState | undefined): string | undefined {
  if (!state) return undefined
  if (state.kind === 'png' || state.kind === 'svg') return state.value
  return undefined
}

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
  // 清理上一次的兜底 timer
  clearImgFallback()
  // 诊断: 记录 input
  // eslint-disable-next-line no-console
  console.log(
    `[AppResultItem:refresh] id=${props.result?.id} title="${props.result?.title}" ` +
      `action=${JSON.stringify(props.result?.action)} ` +
      `resultType=${props.result?.resultType}`,
  )
  const next = await loadIcon(props.result)
  if (myToken !== loadToken) {
    // eslint-disable-next-line no-console
    console.log(`[AppResultItem:refresh] token mismatch for id=${props.result?.id}, discarding`)
    return
  }

  // 优化: 如果 next 与当前 iconState 完全相同 (引用相等 / 字符串相等),
  // 跳过赋值 —— 避免 Vue setter 短路导致 <img>.src 不更新,
  // 进而在 src 字符串相同时 Chromium 跳过 @load.
  const prev = iconState.value
  const isSame =
    prev === next ||
    (prev?.kind === next.kind &&
      (prev.kind === 'component' ? prev.value === (next as any).value : prev.kind === 'monogram' ? prev.letter === (next as any).letter : prev.value === (next as any).value))
  if (isSame && imgReady.value) {
    // eslint-disable-next-line no-console
    console.log(
      `[AppResultItem:refresh] iconState unchanged for id=${props.result?.id}, skip`,
    )
    return
  }

  iconState.value = next
  // 诊断: 记录 output (用 next 自身, 不再读 iconState.value 避免混淆)
  // eslint-disable-next-line no-console
  console.log(
    `[AppResultItem:refresh] -> kind=${next.kind} ` +
      `src.length=${srcOf(next)?.length ?? 'N/A'} ` +
      `srcHead="${srcOf(next)?.slice(0, 60) ?? 'component'}"`,
  )
  // png / svg 用 <img> 加载, 完成后通过 @load 标记 ready
  if (next.kind === 'component' || next.kind === 'monogram') {
    imgReady.value = true
  } else {
    // 先置 false 让 opacity: 0 占位, 等待 @load 或兜底 timer
    imgReady.value = false
    // 关键: 等待 Vue 把 src 真正写到 DOM, 再启动兜底 timer.
    // 否则 timer 触发时, src 还没赋值, 计时意义不对.
    nextTick(() => {
      // 验证 DOM 元素拿到了正确的 src
      const img = document.querySelector(
        `[data-app-result-id="${props.result?.id}"] img`,
      ) as HTMLImageElement | null
      // eslint-disable-next-line no-console
      console.log(
        `[AppResultItem:refresh] nextTick dom-check id=${props.result?.id} ` +
          `img.src.head="${img?.src?.slice(0, 60) ?? 'NOT_FOUND'}" ` +
          `naturalWidth=${img?.naturalWidth ?? 'N/A'}`,
      )
      // 如果 naturalWidth > 0 说明已经解码完成 (浏览器可能同步解码 data URL),
      // 直接置 ready, 不用等 @load.
      if (img && img.naturalWidth > 0) {
        imgReady.value = true
        return
      }
      scheduleImgFallback()
    })
  }
}

onMounted(refreshIcon)
watch(() => props.result?.id, refreshIcon)
onBeforeUnmount(() => {
  clearImgFallback()
})

function onImgLoad(ev: Event) {
  // eslint-disable-next-line no-console
  console.log(
    `[AppResultItem:img] @load id=${props.result?.id} ` +
      `naturalWidth=${(ev.target as HTMLImageElement)?.naturalWidth ?? 'N/A'} ` +
      `srcHead="${srcOf(iconState.value)?.slice(0, 60) ?? 'N/A'}"`,
  )
  imgReady.value = true
  clearImgFallback()
}
function onImgError(ev: Event) {
  // eslint-disable-next-line no-console
  console.warn(
    `[AppResultItem:img] @error id=${props.result?.id} ` +
      `srcHead="${srcOf(iconState.value)?.slice(0, 60) ?? 'N/A'}"`,
    ev,
  )
  // 图片加载失败, 降级到 Lucide 通用图标, 避免破图
  iconState.value = { kind: 'component', value: AppWindow }
  imgReady.value = true
  clearImgFallback()
}
</script>

<template>
  <div
    :class="['app-result-item', { 'app-result-item--active': active }]"
    v-tooltip="tooltipOptions"
    :data-app-result-id="result?.id"
  >
    <div class="app-result-item__icon">
      <img
        v-if="iconState.kind === 'svg' || iconState.kind === 'png'"
        :src="iconState.value"
        class="app-result-item__img"
        :class="{ 'app-result-item__img--ready': imgReady }"
        @load="onImgLoad"
        @error="onImgError"
        decoding="async"
        draggable="false"
        alt=""
      />
      <div
        v-else-if="iconState.kind === 'monogram'"
        class="app-result-item__monogram"
        :style="{ background: iconState.color }"
        :data-letter="iconState.letter"
      >
        {{ iconState.letter }}
      </div>
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
      <CornerDownLeft :size="15" :stroke-width="1.8" class="app-result-item__enter" />
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

/* === Monogram 单字母占位符 (无真实图标时) === */
.app-result-item__monogram {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 7px;
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: 0.02em;
  line-height: 1;
  text-transform: uppercase;
  user-select: none;
  pointer-events: none;
  transition:
    color var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
}

.app-result-item:hover .app-result-item__monogram {
  color: var(--text-primary);
  transform: scale(1.04);
}

.app-result-item--active .app-result-item__monogram {
  color: var(--accent);
  filter: drop-shadow(0 0 6px var(--accent-glow));
  transform: scale(1.06);
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
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  margin-left: auto;
  width: 22px;
  height: 22px;
  border-radius: 6px;
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

.app-result-item__enter {
  color: var(--text-muted);
  opacity: 0.85;
  transition:
    color var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out),
    opacity var(--dur-fast) var(--ease-out);
}

.app-result-item--active .app-result-item__enter {
  color: var(--accent);
  opacity: 1;
  transform: scale(1.08);
}
</style>
