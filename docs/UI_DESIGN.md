# MonoTools UI 设计规范与 PrimeVue 定制

> 版本: v1.0  
> 日期: 2026-07-01  
> 设计基础: Linear Design System (DESIGN.md)

---

## 1. 设计概述

MonoTools 采用 **Linear 深色设计系统** 作为默认主题，基于 PrimeVue 4.x 组件库进行深度定制。所有视觉元素遵循以下核心特征：

- **近乎纯黑的画布**: `#010102` 作为页面背景
- **四级表面层级**: 通过 surface-1 ~ surface-4 构建卡片和面板层级
- **单一薰衣草强调色**: `#5e6ad2` 仅用于主按钮、焦点环、品牌标识
- **1px 发丝边框**: 替代阴影表达深度
- **负字距大标题**: display 级别使用 `-3.0px` 至 `-0.6px` 字距
- **Inter 字体族**: 作为 Linear 自定义字体的开源替代

---

## 2. 色彩系统

### 2.1 CSS 变量定义

```css
/* ============================================
   MonoTools Design Tokens
   由主题插件动态注入，以下为默认深色主题
   ============================================ */

:root {
  /* ---- Brand & Accent ---- */
  --mt-primary: #5e6ad2;
  --mt-primary-hover: #828fff;
  --mt-primary-focus: #5e69d1;
  --mt-on-primary: #ffffff;
  --mt-brand-secure: #7a7fad;

  /* ---- Ink (Text) ---- */
  --mt-ink: #f7f8f8;
  --mt-ink-muted: #d0d6e0;
  --mt-ink-subtle: #8a8f98;
  --mt-ink-tertiary: #62666d;

  /* ---- Surfaces ---- */
  --mt-canvas: #010102;
  --mt-surface-1: #0f1011;
  --mt-surface-2: #141516;
  --mt-surface-3: #18191a;
  --mt-surface-4: #191a1b;

  /* ---- Borders (Hairlines) ---- */
  --mt-hairline: #23252a;
  --mt-hairline-strong: #34343a;
  --mt-hairline-tertiary: #3e3e44;

  /* ---- Inverse (for light elements on dark) ---- */
  --mt-inverse-canvas: #ffffff;
  --mt-inverse-surface-1: #f5f6f6;
  --mt-inverse-ink: #000000;

  /* ---- Semantic ---- */
  --mt-success: #27a644;
  --mt-overlay: rgba(0, 0, 0, 0.6);

  /* ---- Typography ---- */
  --mt-font-display: 'Inter', 'SF Pro Display', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  --mt-font-body: 'Inter', 'SF Pro Text', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  --mt-font-mono: 'JetBrains Mono', 'SF Mono', 'Menlo', ui-monospace, monospace;

  /* ---- Spacing ---- */
  --mt-space-xxs: 4px;
  --mt-space-xs: 8px;
  --mt-space-sm: 12px;
  --mt-space-md: 16px;
  --mt-space-lg: 24px;
  --mt-space-xl: 32px;
  --mt-space-2xl: 48px;
  --mt-space-section: 96px;

  /* ---- Radius ---- */
  --mt-radius-xs: 4px;
  --mt-radius-sm: 6px;
  --mt-radius-md: 8px;
  --mt-radius-lg: 12px;
  --mt-radius-xl: 16px;
  --mt-radius-2xl: 24px;
  --mt-radius-pill: 9999px;

  /* ---- Shadows (极少使用) ---- */
  --mt-shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.1);
  --mt-shadow-md: 0 4px 12px rgba(0, 0, 0, 0.15);
}
```

### 2.2 PrimeVue 主题覆盖

PrimeVue 4 使用 `@primeuix/themes` 进行主题定制。创建 `themes/monotools.ts`：

```typescript
import { definePreset } from '@primeuix/themes'
import Aura from '@primeuix/themes/aura'

export const MonoToolsPreset = definePreset(Aura, {
  semantic: {
    primary: {
      50: '#eef0ff',
      100: '#d0d4ff',
      200: '#a8b0ff',
      300: '#828fff',
      400: '#5e6ad2',
      500: '#5e6ad2',
      600: '#4a54a8',
      700: '#3a4282',
      800: '#2a3060',
      900: '#1c2040',
      950: '#101224',
    },
    colorScheme: {
      dark: {
        surface: {
          0: '#010102',
          50: '#0a0b0c',
          100: '#0f1011',
          200: '#141516',
          300: '#18191a',
          400: '#191a1b',
          500: '#23252a',
          600: '#34343a',
          700: '#3e3e44',
          800: '#4a4a50',
          900: '#56565c',
          950: '#62666d',
        },
        primary: {
          color: '#5e6ad2',
          contrastColor: '#ffffff',
          hoverColor: '#828fff',
          activeColor: '#5e69d1',
        },
        text: {
          color: '#f7f8f8',
          mutedColor: '#d0d6e0',
          hoverColor: '#ffffff',
        },
        content: {
          background: '#0f1011',
          hoverBackground: '#141516',
          borderColor: '#23252a',
        },
        overlay: {
          background: '#141516',
          borderColor: '#23252a',
        },
        formField: {
          background: '#0f1011',
          borderColor: '#23252a',
          hoverBorderColor: '#34343a',
          focusBorderColor: '#5e6ad2',
          color: '#f7f8f8',
          placeholderColor: '#8a8f98',
        },
      },
    },
  },
  components: {
    button: {
      root: {
        borderRadius: '8px',
        padding: '8px 14px',
        fontFamily: 'var(--mt-font-body)',
        fontSize: '14px',
        fontWeight: '500',
      },
    },
    inputtext: {
      root: {
        background: '#0f1011',
        borderColor: '#23252a',
        hoverBorderColor: '#34343a',
        focusBorderColor: '#5e6ad2',
        color: '#f7f8f8',
        borderRadius: '8px',
        padding: '8px 12px',
        fontFamily: 'var(--mt-font-body)',
      },
    },
    card: {
      root: {
        background: '#0f1011',
        borderColor: '#23252a',
        borderRadius: '12px',
        padding: '24px',
      },
    },
    dialog: {
      root: {
        background: '#141516',
        borderColor: '#23252a',
        borderRadius: '16px',
        color: '#f7f8f8',
      },
    },
    listbox: {
      root: {
        background: '#0f1011',
        borderColor: '#23252a',
        borderRadius: '8px',
      },
      option: {
        focusBackground: '#18191a',
        selectedBackground: 'rgba(94, 106, 210, 0.15)',
        selectedColor: '#f7f8f8',
        padding: '10px 14px',
      },
    },
  },
})
```

在 `main.ts` 中应用：

```typescript
import { createApp } from 'vue'
import PrimeVue from 'primevue/config'
import { MonoToolsPreset } from './themes/monotools'
import App from './App.vue'

const app = createApp(App)

app.use(PrimeVue, {
  theme: {
    preset: MonoToolsPreset,
    options: {
      darkModeSelector: '.mt-dark',
      cssLayer: {
        name: 'primevue',
        order: 'tailwind-base, primevue, tailwind-utilities',
      },
    },
  },
})

app.mount('#app')
```

---

## 3. 排版系统

### 3.1 字体栈

```css
/* 引入 Inter 字体 */
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap');

/* 全局字体设置 */
body {
  font-family: var(--mt-font-body);
  font-size: 16px;
  line-height: 1.5;
  color: var(--mt-ink);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
```

### 3.2 排版层级

| Token        | Size | Weight | Line Height | Letter Spacing | 用途               |
| ------------ | ---- | ------ | ----------- | -------------- | ------------------ |
| `display-xl` | 80px | 600    | 1.05        | -3.0px         | 欢迎页大标题       |
| `display-lg` | 56px | 600    | 1.10        | -1.8px         | 页面标题           |
| `display-md` | 40px | 600    | 1.15        | -1.0px         | 区块标题           |
| `headline`   | 28px | 600    | 1.20        | -0.6px         | 卡片标题、CTA      |
| `card-title` | 22px | 500    | 1.25        | -0.4px         | 功能卡片标题       |
| `subhead`    | 20px | 400    | 1.40        | -0.2px         | 引导段落           |
| `body-lg`    | 18px | 400    | 1.50        | -0.1px         | 重要正文           |
| `body`       | 16px | 400    | 1.50        | -0.05px        | 默认正文           |
| `body-sm`    | 14px | 400    | 1.50        | 0              | 卡片正文、元信息   |
| `caption`    | 12px | 400    | 1.40        | 0              | 标签、状态         |
| `button`     | 14px | 500    | 1.20        | 0              | 按钮文字           |
| `eyebrow`    | 13px | 500    | 1.30        | 0.4px          | 分类标签（正字距） |
| `mono`       | 13px | 400    | 1.50        | 0              | 代码、路径         |

### 3.3 Vue 排版组件

```vue
<!-- components/typography/MtDisplay.vue -->
<template>
  <component :is="tag" :class="[`mt-display-${size}`, className]">
    <slot />
  </component>
</template>

<script setup lang="ts">
defineProps<{
  size: 'xl' | 'lg' | 'md'
  tag?: string
  className?: string
}>()
</script>

<style scoped>
.mt-display-xl {
  font-size: 80px;
  font-weight: 600;
  line-height: 1.05;
  letter-spacing: -3.0px;
  color: var(--mt-ink);
}

.mt-display-lg {
  font-size: 56px;
  font-weight: 600;
  line-height: 1.10;
  letter-spacing: -1.8px;
  color: var(--mt-ink);
}

.mt-display-md {
  font-size: 40px;
  font-weight: 600;
  line-height: 1.15;
  letter-spacing: -1.0px;
  color: var(--mt-ink);
}

/* 响应式缩放 */
@media (max-width: 768px) {
  .mt-display-xl { font-size: 36px; letter-spacing: -1.0px; }
  .mt-display-lg { font-size: 32px; letter-spacing: -0.8px; }
  .mt-display-md { font-size: 24px; letter-spacing: -0.6px; }
}
</style>
```

---

## 4. 组件设计规范

### 4.1 搜索框 (SearchBox)

```vue
<!-- components/search/SearchBox.vue -->
<template>
  <div class="search-box" :class="{ 'is-active': isFocused }">
    <!-- 输入区域 -->
    <div class="search-input-wrapper">
      <i class="search-icon">
        <SearchIcon :size="18" />
      </i>
      <input
        ref="inputRef"
        v-model="query"
        class="search-input"
        :placeholder="placeholder"
        @input="onInput"
        @keydown="onKeydown"
        @focus="isFocused = true"
        @blur="isFocused = false"
      />
      <div v-if="query" class="search-clear" @click="clearQuery">
        <XIcon :size="14" />
      </div>
      <div class="search-mode-badge">
        {{ modeBadge }}
      </div>
    </div>
    
    <!-- 结果区域 -->
    <TransitionGroup name="results" tag="div" class="search-results">
      <ResultItem
        v-for="(item, index) in results"
        :key="item.id"
        :item="item"
        :index="index"
        :selected="selectedIndex === index"
        @select="onSelect"
        @action="onAction"
      />
    </TransitionGroup>
    
    <!-- 底部状态栏 -->
    <div class="search-footer">
      <span class="footer-hint">{{ resultCount }} 个结果</span>
      <span class="footer-shortcuts">
        <kbd>↑↓</kbd> 导航 <kbd>Enter</kbd> 打开 <kbd>Esc</kbd> 关闭
      </span>
    </div>
  </div>
</template>
```

```css
.search-box {
  width: 800px;
  max-width: 90vw;
  background: var(--mt-surface-1);
  border: 1px solid var(--mt-hairline);
  border-radius: var(--mt-radius-xl);
  padding: var(--mt-space-md);
  box-shadow: 0 24px 48px rgba(0, 0, 0, 0.4);
  overflow: hidden;
}

.search-input-wrapper {
  display: flex;
  align-items: center;
  gap: var(--mt-space-sm);
  padding: var(--mt-space-sm) var(--mt-space-md);
  background: var(--mt-canvas);
  border: 1px solid var(--mt-hairline);
  border-radius: var(--mt-radius-md);
  transition: border-color 0.15s ease;
}

.search-input-wrapper:focus-within {
  border-color: var(--mt-primary-focus);
  box-shadow: 0 0 0 2px rgba(94, 106, 210, 0.25);
}

.search-input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: var(--mt-ink);
  font-family: var(--mt-font-body);
  font-size: 16px;
  line-height: 1.5;
}

.search-input::placeholder {
  color: var(--mt-ink-subtle);
}

.search-mode-badge {
  padding: 2px 8px;
  background: var(--mt-surface-2);
  border-radius: var(--mt-radius-pill);
  font-size: 12px;
  color: var(--mt-ink-subtle);
  font-family: var(--mt-font-mono);
}

/* 结果项动画 */
.results-enter-active,
.results-leave-active {
  transition: all 0.15s ease;
}

.results-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.results-leave-to {
  opacity: 0;
  transform: translateX(-10px);
}

/* 交错动画 */
.result-item:nth-child(1) { animation-delay: 0ms; }
.result-item:nth-child(2) { animation-delay: 30ms; }
.result-item:nth-child(3) { animation-delay: 60ms; }
/* ... 最多前 10 项 */
```

### 4.2 结果项 (ResultItem)

```vue
<!-- components/search/ResultItem.vue -->
<template>
  <div
    class="result-item"
    :class="{ 'is-selected': selected }"
    @mouseenter="$emit('hover', index)"
    @click="$emit('select', item)"
  >
    <div class="result-icon">
      <img v-if="item.icon" :src="item.icon" />
      <component v-else-if="item.iconComponent" :is="item.iconComponent" />
      <FileIcon v-else :size="20" />
    </div>
    
    <div class="result-content">
      <div class="result-title" v-html="highlightedTitle" />
      <div class="result-subtitle">{{ item.subtitle }}</div>
    </div>
    
    <div class="result-meta">
      <span class="result-source">{{ item.source }}</span>
      <div v-if="selected" class="result-actions">
        <kbd v-for="action in item.actions" :key="action.id">
          {{ action.shortcut }}
        </kbd>
      </div>
    </div>
  </div>
</template>
```

```css
.result-item {
  display: flex;
  align-items: center;
  gap: var(--mt-space-sm);
  padding: 10px 14px;
  border-radius: var(--mt-radius-md);
  cursor: pointer;
  transition: background-color 0.1s ease;
}

.result-item:hover,
.result-item.is-selected {
  background: var(--mt-surface-2);
}

.result-icon {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--mt-surface-3);
  border-radius: var(--mt-radius-sm);
  flex-shrink: 0;
}

.result-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--mt-ink);
  line-height: 1.4;
}

.result-subtitle {
  font-size: 12px;
  color: var(--mt-ink-subtle);
  line-height: 1.4;
  margin-top: 2px;
}

.result-source {
  font-size: 11px;
  color: var(--mt-ink-tertiary);
  padding: 2px 6px;
  background: var(--mt-surface-3);
  border-radius: var(--mt-radius-xs);
}
```

### 4.3 卡片组件 (MtCard)

基于 PrimeVue Card 封装：

```vue
<!-- components/common/MtCard.vue -->
<template>
  <Card :class="['mt-card', `mt-card--${variant}`, { 'mt-card--hoverable': hoverable }]">
    <template #header v-if="$slots.header">
      <slot name="header" />
    </template>
    <template #title v-if="$slots.title || title">
      <h3 class="mt-card-title">{{ title }}</h3>
    </template>
    <template #subtitle v-if="subtitle">
      <p class="mt-card-subtitle">{{ subtitle }}</p>
    </template>
    <template #content>
      <slot />
    </template>
    <template #footer v-if="$slots.footer">
      <slot name="footer" />
    </template>
  </Card>
</template>

<script setup lang="ts">
import Card from 'primevue/card'

defineProps<{
  title?: string
  subtitle?: string
  variant?: 'default' | 'elevated' | 'featured'
  hoverable?: boolean
}>()
</script>

<style scoped>
.mt-card {
  background: var(--mt-surface-1);
  border: 1px solid var(--mt-hairline);
  border-radius: var(--mt-radius-lg);
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.mt-card--elevated {
  background: var(--mt-surface-2);
  border-color: var(--mt-hairline-strong);
}

.mt-card--featured {
  background: var(--mt-surface-2);
  border: 1px solid var(--mt-primary);
  border-opacity: 0.3;
}

.mt-card--hoverable:hover {
  transform: translateY(-2px);
  border-color: var(--mt-hairline-strong);
}

.mt-card-title {
  font-size: 22px;
  font-weight: 500;
  line-height: 1.25;
  letter-spacing: -0.4px;
  color: var(--mt-ink);
  margin: 0 0 8px;
}

.mt-card-subtitle {
  font-size: 14px;
  color: var(--mt-ink-subtle);
  line-height: 1.5;
  margin: 0 0 16px;
}

:deep(.p-card-content) {
  padding: 0;
}

:deep(.p-card-body) {
  padding: 24px;
}
</style>
```

### 4.4 按钮组件 (MtButton)

基于 PrimeVue Button 封装：

```vue
<!-- components/common/MtButton.vue -->
<template>
  <Button
    :class="['mt-button', `mt-button--${variant}`, { 'mt-button--block': block }]"
    :severity="primeSeverity"
    :text="variant === 'tertiary'"
    :outlined="variant === 'secondary'"
  >
    <slot />
  </Button>
</template>

<script setup lang="ts">
import Button from 'primevue/button'
import { computed } from 'vue'

const props = defineProps<{
  variant?: 'primary' | 'secondary' | 'tertiary' | 'inverse'
  block?: boolean
}>()

const primeSeverity = computed(() => {
  switch (props.variant) {
    case 'primary': return 'primary'
    case 'inverse': return 'secondary'
    default: return 'secondary'
  }
})
</script>

<style scoped>
.mt-button {
  font-family: var(--mt-font-body);
  font-size: 14px;
  font-weight: 500;
  line-height: 1.2;
  padding: 8px 14px;
  border-radius: var(--mt-radius-md);
  transition: all 0.15s ease;
}

.mt-button--primary {
  background: var(--mt-primary);
  color: var(--mt-on-primary);
  border: none;
}

.mt-button--primary:hover {
  background: var(--mt-primary-hover);
}

.mt-button--secondary {
  background: var(--mt-surface-1);
  color: var(--mt-ink);
  border: 1px solid var(--mt-hairline);
}

.mt-button--secondary:hover {
  background: var(--mt-surface-2);
  border-color: var(--mt-hairline-strong);
}

.mt-button--tertiary {
  color: var(--mt-ink);
  background: transparent;
}

.mt-button--tertiary:hover {
  background: var(--mt-surface-2);
}

.mt-button--inverse {
  background: var(--mt-inverse-canvas);
  color: var(--mt-inverse-ink);
  border: none;
}

.mt-button--block {
  width: 100%;
  justify-content: center;
}
</style>
```

---

## 5. 布局规范

### 5.1 搜索框窗口布局

```
┌────────────────────────────────────────┐
│              屏幕中央偏上               │
│                                        │
│     ┌────────────────────────────┐   │
│     │      SearchBox (800px)       │   │
│     │  ┌────────────────────────┐  │   │
│     │  │     输入框 (56px)       │  │   │
│     │  └────────────────────────┘  │   │
│     │                                │   │
│     │  ┌────────────────────────┐    │   │
│     │  │     结果列表           │    │   │
│     │  │  ┌──────────────────┐ │    │   │
│     │  │  │ 结果项 1 (48px)   │ │    │   │
│     │  │  │ 结果项 2          │ │    │   │
│     │  │  │ ...               │ │    │   │
│     │  │  └──────────────────┘ │    │   │
│     │  └────────────────────────┘    │   │
│     │                                │   │
│     │  ┌────────────────────────┐    │   │
│     │  │  底部状态栏 (32px)      │    │   │
│     │  └────────────────────────┘    │   │
│     └────────────────────────────────┘   │
│                                        │
└────────────────────────────────────────┘
```

**尺寸规范**:
- 窗口宽度: 800px（桌面），100%（移动端 < 768px）
- 窗口高度: 自适应，最大 520px
- 输入框高度: 48px
- 结果项高度: 48px
- 结果列表最大高度: 360px（约 7.5 个结果项）
- 圆角: 16px（窗口），12px（卡片）

### 5.2 设置面板布局

```
┌─────────────────────────────────────────────────┐
│  MonoTools 设置                                  │
├──────────┬──────────────────────────────────────┤
│          │                                      │
│  通用     │   ┌────────────────────────────┐   │
│  搜索     │   │      设置表单区域            │   │
│  工作区   │   │                            │   │
│  插件     │   │   [表单控件]  [表单控件]      │   │
│  主题     │   │                            │   │
│  快捷键   │   │   [保存] [取消]              │   │
│  关于     │   └────────────────────────────┘   │
│          │                                      │
└──────────┴──────────────────────────────────────┘
```

- 侧边栏宽度: 200px
- 内容区最大宽度: 640px
- 表单间距: 24px

---

## 6. 动画规范

### 6.1 搜索框唤出动画

```css
@keyframes search-enter {
  0% {
    opacity: 0;
    transform: scale(0.92) translateY(-16px);
  }
  60% {
    opacity: 1;
    transform: scale(1.01) translateY(2px);
  }
  100% {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.search-box-enter {
  animation: search-enter 0.25s cubic-bezier(0.16, 1, 0.3, 1) forwards;
}
```

### 6.2 搜索框隐藏动画

```css
@keyframes search-exit {
  0% {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
  100% {
    opacity: 0;
    transform: scale(0.96) translateY(-8px);
  }
}

.search-box-exit {
  animation: search-exit 0.15s ease-in forwards;
}
```

### 6.3 结果项交错动画

```css
@keyframes result-enter {
  0% {
    opacity: 0;
    transform: translateY(8px);
  }
  100% {
    opacity: 1;
    transform: translateY(0);
  }
}

.result-item {
  opacity: 0;
  animation: result-enter 0.2s ease-out forwards;
}

/* 动态延迟通过 style 绑定 */
.result-item[data-index="0"] { animation-delay: 0ms; }
.result-item[data-index="1"] { animation-delay: 30ms; }
.result-item[data-index="2"] { animation-delay: 60ms; }
/* JS 生成最多 10 项的延迟 */
```

### 6.4 卡片悬停动画

```css
.mt-card--hoverable {
  transition: 
    transform 0.2s cubic-bezier(0.16, 1, 0.3, 1),
    border-color 0.15s ease;
}

.mt-card--hoverable:hover {
  transform: translateY(-4px);
}
```

### 6.5 按钮点击反馈

```css
.mt-button {
  transition: transform 0.1s ease, background-color 0.15s ease;
}

.mt-button:active {
  transform: scale(0.97);
}
```

---

## 7. 图标系统

使用 `lucide-vue-next` 作为图标库，与 Linear 风格一致（细线、几何、无填充）。

```typescript
// 图标配置
import { createIconsConfig } from 'lucide-vue-next'

export const icons = createIconsConfig({
  strokeWidth: 1.5,
  size: 20,
  defaultClass: 'mt-icon',
})
```

```css
.mt-icon {
  color: var(--mt-ink-subtle);
  transition: color 0.15s ease;
}

/* 选中/悬停时图标变色 */
.result-item.is-selected .mt-icon,
.result-item:hover .mt-icon {
  color: var(--mt-ink);
}
```

---

## 8. 响应式策略

| 断点       | 宽度    | 调整                           |
| ---------- | ------- | ------------------------------ |
| Desktop-XL | ≥1440px | 默认布局                       |
| Desktop    | ≥1280px | 默认布局                       |
| Tablet     | ≥1024px | 设置面板侧边栏折叠为图标       |
| Mobile-Lg  | ≥768px  | 搜索框宽度 100%，结果列表全宽  |
| Mobile     | <768px  | 单栏，字体缩放，触摸目标 ≥44px |

```css
@media (max-width: 768px) {
  .search-box {
    width: 100vw;
    max-width: 100vw;
    border-radius: 0;
    border: none;
  }
  
  .mt-display-xl { font-size: 36px; }
  .mt-display-lg { font-size: 32px; }
  .mt-display-md { font-size: 24px; }
}
```

---

## 9. 毛玻璃效果（可选增强）

部分主题可能启用毛玻璃效果：

```css
.glass-panel {
  background: rgba(15, 16, 17, 0.72);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.08);
}

/* 注意：WebView2 在 Windows 上支持 backdrop-filter */
```

---

## 10. PrimeVue 组件使用清单

| 组件         | 用途                 | 定制程度       |
| ------------ | -------------------- | -------------- |
| Button       | 所有按钮             | 完全定制样式   |
| InputText    | 搜索框、表单输入     | 完全定制样式   |
| Card         | 工作区卡片、功能卡片 | 完全定制样式   |
| Dialog       | 确认对话框、插件详情 | 定制样式       |
| Listbox      | 设置选项列表         | 定制样式       |
| ToggleSwitch | 开关设置             | 定制颜色       |
| Select       | 下拉选择             | 定制样式       |
| Tabs         | 设置面板分类         | 定制样式       |
| Toast        | 通知提示             | 定制位置和样式 |
| Tooltip      | 按钮提示             | 定制样式       |
| Skeleton     | 加载占位             | 定制颜色       |