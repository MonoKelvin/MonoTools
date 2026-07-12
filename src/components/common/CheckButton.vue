<script setup lang="ts">
/**
 * CheckButton — 扁平化动画勾选按钮 (v2)
 *
 * 视觉: 默认仅 1px 细描边, 选中时填充实色 + 对勾"绘制"动画
 * 颜色: 黑白灰, 选中态用近白色 (深色主题) / 近黑色 (浅色主题)
 * 动效:
 *   - 描边→填充: 280ms ease-out (颜色 + 背景)
 *   - 对勾: stroke-dasharray 320ms ease-out (从 24 绘制到 0, 模拟手写)
 *   - 整体: cubic-bezier(0.34, 1.56, 0.64, 1) 弹性入场
 *   - 背景: 选中时 220ms 缓出过渡
 */
import { computed } from 'vue'
import { Check } from '@lucide/vue'

const props = withDefaults(
  defineProps<{
    modelValue: boolean
    /** 选中时填充色, 默认使用主题前景色 */
    activeColor?: string
    size?: number
  }>(),
  {
    activeColor: 'currentColor',
    size: 14,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: boolean): void
}>()

function toggle() {
  emit('update:modelValue', !props.modelValue)
}

const stroke = computed(() => 2.6)
</script>

<template>
  <button
    type="button"
    role="checkbox"
    :aria-checked="modelValue"
    class="check-btn"
    :class="{ 'check-btn--checked': modelValue }"
    :style="{
      '--cb-size': size + 'px',
      '--cb-check-size': Math.max(8, size - 4) + 'px',
    }"
    @click.stop="toggle"
    @mousedown.stop
  >
    <span class="check-btn__box">
      <Transition name="check-draw">
        <Check
          v-if="modelValue"
          :size="size - 4"
          :stroke-width="stroke"
          class="check-btn__icon"
        />
      </Transition>
    </span>
  </button>
</template>

<style scoped>
.check-btn {
  --cb-size: 14px;
  --cb-check-size: 10px;
  flex-shrink: 0;
  width: var(--cb-size);
  height: var(--cb-size);
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  cursor: pointer;
  border-radius: 5px;
  transition: transform 120ms var(--ease-out, cubic-bezier(0.16, 1, 0.3, 1));
  -webkit-tap-highlight-color: transparent;
}

.check-btn:active {
  transform: scale(0.9);
}

.check-btn__box {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border-default, rgba(255, 255, 255, 0.18));
  border-radius: 5px;
  background: transparent;
  color: var(--text-quaternary, #71717a);
  transition:
    background 240ms cubic-bezier(0.16, 1, 0.3, 1),
    border-color 240ms cubic-bezier(0.16, 1, 0.3, 1),
    color 240ms cubic-bezier(0.16, 1, 0.3, 1),
    box-shadow 240ms cubic-bezier(0.16, 1, 0.3, 1);
  position: relative;
}

.check-btn:hover .check-btn__box {
  border-color: var(--border-hover, rgba(255, 255, 255, 0.36));
  color: var(--text-tertiary, #a1a1aa);
}

.check-btn--checked .check-btn__box {
  background: #f5f1e8;
  border-color: #f5f1e8;
  color: #15151a;
  box-shadow:
    0 0 0 1px rgba(245, 241, 232, 0.15),
    0 1px 2px rgba(0, 0, 0, 0.3),
    0 0 12px rgba(245, 241, 232, 0.12);
  animation: cb-pop 360ms cubic-bezier(0.34, 1.56, 0.64, 1);
}

.check-btn--checked:hover .check-btn__box {
  background: #ffffff;
  border-color: #ffffff;
  box-shadow:
    0 0 0 1px rgba(255, 255, 255, 0.2),
    0 1px 2px rgba(0, 0, 0, 0.3),
    0 0 14px rgba(255, 255, 255, 0.18);
}

@keyframes cb-pop {
  0% {
    transform: scale(0.85);
  }
  55% {
    transform: scale(1.10);
  }
  100% {
    transform: scale(1);
  }
}

/* 对勾: "绘制" 动画 - 模拟笔尖从起点到终点 */
.check-draw-enter-active {
  transition:
    opacity 200ms cubic-bezier(0.16, 1, 0.3, 1),
    transform 280ms cubic-bezier(0.34, 1.2, 0.64, 1);
}

.check-draw-enter-from {
  opacity: 0;
  transform: scale(0.3) rotate(-18deg);
}

.check-draw-leave-active {
  transition:
    opacity 100ms cubic-bezier(0.4, 0, 1, 1),
    transform 120ms cubic-bezier(0.4, 0, 1, 1);
}

.check-draw-leave-to {
  opacity: 0;
  transform: scale(0.6);
}

.check-btn__icon {
  display: block;
  /* 强制让 lucide 的 svg 走 stroke 描边路径 */
  stroke-linecap: round;
  stroke-linejoin: round;
  /* 利用 stroke-dasharray + stroke-dashoffset 实现"绘制" */
  stroke-dasharray: 26;
  stroke-dashoffset: 26;
  animation: check-draw-stroke 360ms cubic-bezier(0.16, 1, 0.3, 1) 80ms forwards;
}

@keyframes check-draw-stroke {
  0% {
    stroke-dashoffset: 26;
    opacity: 0.6;
  }
  60% {
    opacity: 1;
  }
  100% {
    stroke-dashoffset: 0;
    opacity: 1;
  }
}

/* 焦点态 (键盘) */
.check-btn:focus-visible .check-btn__box {
  outline: 2px solid var(--accent, #f5f1e8);
  outline-offset: 2px;
}
</style>
