<script setup lang="ts">
/**
 * OverlayPage — 浮窗页面基类
 *
 * 用于所有需要"系统级浮窗"行为的页面（搜索浮窗、设置面板、命令面板等）。
 * 封装：
 *  - 全屏透明背景，让 Win11 Mica / Win10 backdrop-filter 透出
 *  - 容器淡入 + 微缩放进场动效 (220ms, 配合 --ease-out)
 *  - 自动适配 Windows 版本：Win10 用 CSS backdrop-filter，Win11 透明
 *  - 顶部细线 + 阴影做层级（Raycast 风格 hairline border）
 *
 * 未来新增浮窗类页面（SettingsPage、PluginsPage 等）直接 <OverlayPage> 包裹。
 */
</script>

<template>
  <div class="overlay-page">
    <div class="overlay-page__container">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.overlay-page {
  width: 100%;
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: flex-start;
  background: transparent;
  overflow: hidden;
  position: relative;
}

.overlay-page__container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: linear-gradient(180deg, rgba(255,255,255,0.018) 0%, rgba(255,255,255,0) 100%);
  border-top: 1px solid var(--border-subtle);
  box-shadow: var(--shadow-xl);
  overflow: hidden;
  animation: overlay-fade-in 220ms var(--ease-out);
}

/* 全局屏蔽 webview 右键菜单 (Raycast / Linear 桌面应用规范).
   所有右键交互由 SearchPage / SearchInput / ContextMenu 自行处理. */
.os-no-contextmenu {
  -webkit-user-select: none;
  user-select: none;
}

.os-win10 .overlay-page__container {
  background: var(--glass-bg);
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
}

@keyframes overlay-fade-in {
  from {
    opacity: 0;
    transform: translateY(-8px) scale(0.985);
    filter: blur(6px);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
    filter: blur(0);
  }
}
</style>
