<script setup lang="ts">
/**
 * ThemeToggle — 单键主题切换 (Sun ↔ Moon, Fluent 风格)
 *
 * 设计原则:
 * - 一个圆形按钮, 点击循环切换 light / dark
 * - 图标采用旋转 + 渐显过渡 (Raycast 风格), 不用变透明度生硬切换
 * - 鼠标悬停时显示一个微弱的光晕, 表明可点击
 * - 内部用两个图标堆叠, 各自带 rotate 进入, 形成"翻牌"质感
 */
import { computed, ref, onBeforeUnmount } from 'vue'
import { useThemeStore } from '@/core/stores/theme'
import { FONT_SIZES, ICON_CONFIG } from '@/core/config'
import MtTooltip from '@/ui/components/MtTooltip.vue'

const themeStore = useThemeStore()

const isLight = computed(() => themeStore.mode === 'light')

const tooltipText = computed(() =>
  `当前主题：${isLight.value ? '浅色' : '深色'}（点击切换）`,
)

const tooltipVisible = ref(false)
const btnEl = ref<HTMLElement | null>(null)
let tooltipTimer: ReturnType<typeof setTimeout> | null = null

function onBtnEnter() {
  clearTooltipTimer()
  tooltipTimer = setTimeout(() => {
    tooltipVisible.value = true
  }, ICON_CONFIG.tooltipDelayMs)
}

function onBtnLeave() {
  clearTooltipTimer()
  tooltipVisible.value = false
}

function clearTooltipTimer() {
  if (tooltipTimer) {
    clearTimeout(tooltipTimer)
    tooltipTimer = null
  }
}

onBeforeUnmount(() => {
  clearTooltipTimer()
})

async function toggle() {
  // 二态切换; 'auto' 视作 dark 的下一步回到 light
  const next = isLight.value ? 'dark' : 'light'
  await themeStore.setMode(next)
}
</script>

<template>
  <button
    ref="btnEl"
    class="theme-toggle"
    type="button"
    @click="toggle"
    @mouseenter="onBtnEnter"
    @mouseleave="onBtnLeave"
    :aria-label="tooltipText"
  >
    <span class="theme-toggle__track" :data-mode="themeStore.mode">
      <!-- 太阳 (浅色) -->
      <span class="theme-toggle__icon theme-toggle__icon--sun" aria-hidden="true">
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <circle cx="12" cy="12" r="4" />
          <path d="M12 2v2" />
          <path d="M12 20v2" />
          <path d="m4.93 4.93 1.41 1.41" />
          <path d="m17.66 17.66 1.41 1.41" />
          <path d="M2 12h2" />
          <path d="M20 12h2" />
          <path d="m6.34 17.66-1.41 1.41" />
          <path d="m19.07 4.93-1.41 1.41" />
        </svg>
      </span>
      <!-- 月亮 (深色) -->
      <span class="theme-toggle__icon theme-toggle__icon--moon" aria-hidden="true">
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        </svg>
      </span>
    </span>
    <span class="theme-toggle-label">
      {{ isLight ? '浅色' : '深色' }}
    </span>
  </button>
  <MtTooltip
    :visible="tooltipVisible"
    :title="tooltipText"
    :anchor="btnEl"
    placement="bottom"
    :offset-y="4"
  />
</template>

<style scoped>
/* === 容器 ============================================================= */
.theme-toggle {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-2);
  padding: 4px 8px 4px 4px;
  background: transparent;
  border: 1px solid transparent;
  border-radius: var(--radius-full);
  cursor: pointer;
  font-family: inherit;
  color: var(--text-tertiary);
  transition:
    background var(--dur-fast) var(--ease-out),
    border-color var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out);
}

.theme-toggle:hover {
  background: var(--list-hover-bg);
  border-color: var(--border-subtle);
  color: var(--text-secondary);
}

.theme-toggle:active .theme-toggle__track {
  transform: scale(0.94);
}

.theme-toggle:focus-visible {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-glow);
}

/* === 轨道 (圆形 + 微高光) ============================================= */
.theme-toggle__track {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: var(--surface);
  border: 1px solid var(--border-subtle);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
  flex-shrink: 0;
  transition:
    background var(--dur-normal) var(--ease-out),
    border-color var(--dur-normal) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
}

.theme-toggle:hover .theme-toggle__track {
  background: var(--surface-overlay);
  border-color: var(--border-default);
}

/* === 图标堆叠 + 旋转过渡 ============================================= */
/* 太阳和月亮完全重叠, 各自根据 data-mode 决定 rotate / opacity  */
.theme-toggle__icon {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  transition:
    transform 360ms var(--ease-spring),
    opacity var(--dur-normal) var(--ease-out);
  will-change: transform, opacity;
}

/* 默认 (dark) — 月亮显示, 太阳旋出 */
.theme-toggle__icon--sun {
  transform: rotate(-90deg) scale(0.4);
  opacity: 0;
  color: var(--accent-warm);
}
.theme-toggle__icon--moon {
  transform: rotate(0) scale(1);
  opacity: 1;
  color: var(--text-secondary);
}

/* light — 太阳显示, 月亮旋出 */
.theme-toggle__track[data-mode='light'] .theme-toggle__icon--sun {
  transform: rotate(0) scale(1);
  opacity: 1;
}
.theme-toggle__track[data-mode='light'] .theme-toggle__icon--moon {
  transform: rotate(90deg) scale(0.4);
  opacity: 0;
}

/* auto 模式: 沿用 dark 视觉, 不引入新状态 */
.theme-toggle__track[data-mode='auto'] .theme-toggle__icon--sun {
  transform: rotate(-90deg) scale(0.4);
  opacity: 0;
}
.theme-toggle__track[data-mode='auto'] .theme-toggle__icon--moon {
  transform: rotate(0) scale(1);
  opacity: 1;
}

/* === 标签 ============================================================= */
.theme-toggle-label {
  font-size: v-bind('FONT_SIZES.sm + "px"');
  font-weight: 500;
  color: inherit;
  letter-spacing: 0.02em;
  line-height: 1;
}

@media (prefers-reduced-motion: reduce) {
  .theme-toggle__icon {
    transition: opacity var(--dur-fast) linear;
    transform: none !important;
  }
}
</style>
