<template>
  <div class="theme-setting">
    <div class="setting-header">
      <h2 class="setting-title">主题</h2>
      <p class="setting-description">选择应用的外观主题，也可以导入自定义主题</p>
    </div>

    <div class="theme-list">
      <div
        v-for="theme in themes"
        :key="theme.id"
        class="theme-card"
        :class="{
          'theme-card--active': theme.isEnabled,
          'theme-card--dark': theme.themeType === 'dark',
          'theme-card--light': theme.themeType === 'light'
        }"
        @click="applyTheme(theme.id)"
      >
        <div class="theme-card__preview">
          <div
            v-if="theme.screenshots && theme.screenshots.length > 0"
            class="theme-card__screenshot"
          >
            <img :src="theme.screenshots[0]" :alt="theme.name" />
          </div>
          <div v-else class="theme-card__placeholder">
            <div class="theme-card__colors">
              <div class="theme-color" :style="{ backgroundColor: getThemeColor(theme) }"></div>
              <div class="theme-color" :style="{ backgroundColor: getThemeColor2(theme) }"></div>
            </div>
          </div>
          <div v-if="theme.isEnabled" class="theme-card__badge">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path
                d="M13.5 4.5L6 12L2.5 8.5"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </div>
        </div>

        <div class="theme-card__info">
          <div class="theme-card__name">{{ theme.name }}</div>
          <div class="theme-card__desc">{{ theme.description }}</div>
          <div class="theme-card__meta">
            <span class="theme-card__author">{{ theme.author }}</span>
            <span class="theme-card__type">{{
              theme.themeType === 'dark'
                ? '深色'
                : theme.themeType === 'light'
                  ? '浅色'
                  : '跟随系统'
            }}</span>
            <span v-if="theme.isBuiltin" class="theme-card__builtin">内置</span>
          </div>
          <div v-if="theme.tags && theme.tags.length > 0" class="theme-card__tags">
            <span v-for="tag in theme.tags" :key="tag" class="theme-tag">
              {{ tag }}
            </span>
          </div>
        </div>
      </div>

      <div class="theme-card theme-card--import" @click="importTheme">
        <div class="theme-card__preview">
          <div class="theme-card__placeholder theme-card__placeholder--import">
            <svg
              width="48"
              height="48"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="17 8 12 3 7 8" />
              <line x1="12" y1="3" x2="12" y2="15" />
            </svg>
          </div>
        </div>
        <div class="theme-card__info">
          <div class="theme-card__name">导入主题</div>
          <div class="theme-card__desc">从本地文件夹导入主题</div>
        </div>
      </div>
    </div>

    <div class="theme-actions">
      <div v-if="enabledTheme" class="theme-current">
        <span class="theme-current__label">当前主题：</span>
        <span class="theme-current__name">{{ enabledTheme.name }}</span>
        <button
          v-if="!enabledTheme.isBuiltin"
          class="btn btn-sm btn-danger"
          @click="deleteTheme(enabledTheme.id)"
        >
          删除主题
        </button>
      </div>
      <div v-else class="theme-current theme-current--none">未启用主题，使用默认样式</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'

const themes = ref<any[]>([])
const enabledTheme = ref<any>(null)
const loading = ref(false)

onMounted(async () => {
  await loadThemes()
})

async function loadThemes(): Promise<void> {
  loading.value = true
  try {
    const result = await window.monotools.loadThemes()
    if (result.success && result.themes) {
      themes.value = result.themes
      // 找到当前启用的主题
      enabledTheme.value = themes.value.find((t: any) => t.isEnabled) || null
    }
  } catch (error) {
    console.error('加载主题失败:', error)
  } finally {
    loading.value = false
  }
}

async function applyTheme(themeId: string): Promise<void> {
  try {
    await window.monotools.applyTheme(themeId)
    await loadThemes()
  } catch (error) {
    console.error('应用主题失败:', error)
  }
}

async function importTheme(): Promise<void> {
  try {
    const result = await window.monotools.invoke('theme:getThemeDir')
    if (result.success && result.userThemeDir) {
      // 打开文件夹选择对话框（需要主进程支持）
      const input = document.createElement('input')
      input.type = 'file'
      input.webkitdirectory = true
      input.onchange = async (e: any) => {
        const files = e.target.files
        if (files && files.length > 0) {
          const folderPath = files[0].path
          try {
            const importResult = await window.monotools.invoke('theme:import', folderPath)
            if (importResult.success) {
              await loadThemes()
            }
          } catch (error) {
            console.error('导入主题失败:', error)
          }
        }
      }
      input.click()
    }
  } catch (error) {
    console.error('打开主题目录失败:', error)
  }
}

async function deleteTheme(themeId: string): Promise<void> {
  if (!confirm('确定要删除这个主题吗？')) return

  try {
    const result = await window.monotools.invoke('theme:delete', themeId)
    if (result.success) {
      await loadThemes()
    }
  } catch (error) {
    console.error('删除主题失败:', error)
  }
}

function getThemeColor(theme: any): string {
  // 返回主题的第一颜色
  return theme.themeType === 'dark' ? '#010102' : '#fafafa'
}

function getThemeColor2(theme: any): string {
  // 返回主题的第二颜色
  return theme.themeType === 'dark' ? '#0f1011' : '#ffffff'
}
</script>

<style scoped>
.theme-setting {
  padding: var(--spacing-lg, 24px);
}

.setting-header {
  margin-bottom: var(--spacing-xl, 32px);
}

.setting-title {
  font-size: var(--display-md-size, 40px);
  font-weight: var(--display-md-weight, 600);
  line-height: var(--display-md-line-height, 1.15);
  letter-spacing: var(--display-md-letter-spacing, -1px);
  color: var(--color-ink, #333333);
  margin-bottom: var(--spacing-sm, 12px);
}

.setting-description {
  font-size: var(--body-size, 16px);
  color: var(--color-ink-muted, #666666);
  line-height: 1.5;
}

.theme-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: var(--spacing-lg, 24px);
  margin-bottom: var(--spacing-xl, 32px);
}

.theme-card {
  position: relative;
  border: 2px solid var(--color-hairline, #e5e7eb);
  border-radius: var(--rounded-lg, 12px);
  overflow: hidden;
  cursor: pointer;
  transition: all 0.2s;
  background: var(--color-canvas-elevated, #ffffff);
}

.theme-card:hover {
  border-color: var(--color-primary, #0284c7);
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.theme-card--active {
  border-color: var(--color-primary, #0284c7);
  box-shadow: 0 0 0 3px rgba(2, 132, 199, 0.1);
}

.theme-card__preview {
  position: relative;
  height: 160px;
  overflow: hidden;
}

.theme-card__screenshot {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.theme-card__placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-hairline-soft, #f5f5f5);
}

.theme-card__placeholder--import {
  color: var(--color-ink-muted, #666666);
}

.theme-card__colors {
  display: flex;
  gap: var(--spacing-sm, 12px);
}

.theme-color {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  border: 2px solid rgba(255, 255, 255, 0.5);
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.theme-card__badge {
  position: absolute;
  top: var(--spacing-sm, 12px);
  right: var(--spacing-sm, 12px);
  background: var(--color-primary, #0284c7);
  color: white;
  border-radius: 50%;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.theme-card__info {
  padding: var(--spacing-md, 16px);
}

.theme-card__name {
  font-size: var(--body-lg-size, 18px);
  font-weight: 600;
  color: var(--color-ink, #333333);
  margin-bottom: var(--spacing-xs, 8px);
}

.theme-card__desc {
  font-size: var(--body-sm-size, 14px);
  color: var(--color-ink-muted, #666666);
  margin-bottom: var(--spacing-sm, 12px);
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.theme-card__meta {
  display: flex;
  gap: var(--spacing-sm, 12px);
  font-size: var(--caption-size, 12px);
  color: var(--color-ink-subtle, #999999);
  margin-bottom: var(--spacing-xs, 8px);
}

.theme-card__builtin {
  background: var(--color-surface1, #f5f5f5);
  padding: 2px 6px;
  border-radius: 4px;
}

.theme-card__tags {
  display: flex;
  gap: var(--spacing-xs, 8px);
  flex-wrap: wrap;
}

.theme-tag {
  display: inline-block;
  padding: 2px 8px;
  background: var(--color-surface1, #f5f5f5);
  border-radius: 4px;
  font-size: var(--caption-size, 12px);
  color: var(--color-ink-muted, #666666);
}

.theme-actions {
  padding-top: var(--spacing-lg, 24px);
  border-top: 1px solid var(--color-hairline, #e5e7eb);
}

.theme-current {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm, 12px);
  font-size: var(--body-sm-size, 14px);
  color: var(--color-ink-muted, #666666);
}

.theme-current__name {
  font-weight: 600;
  color: var(--color-ink, #333333);
}

.theme-current--none {
  color: var(--color-ink-subtle, #999999);
  font-style: italic;
}
</style>
