<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useToast } from '@/components'

const { info, error, confirm } = useToast()

const appVersion = ref('')
const isCheckingUpdate = ref(false)
const autoCheckUpdate = ref(true)
const currentYear = new Date().getFullYear()

onMounted(async () => {
  await getAppVersion()
  await loadAutoCheckSetting()
})

async function getAppVersion(): Promise<void> {
  try {
    appVersion.value = await window.monotools.internal.getAppVersion()
  } catch (err) {
    console.error('获取版本失败:', err)
    appVersion.value = '未知'
  }
}

async function handleCheckUpdate(): Promise<void> {
  if (isCheckingUpdate.value) return
  isCheckingUpdate.value = true

  try {
    const result = await window.monotools.internal.updaterCheckUpdate()
    if (result.hasUpdate) {
      const shouldUpdate = await confirm({
        title: '发现新版本',
        message: `发现新版本 ${result.latestVersion}，是否立即更新？\n\n更新内容：\n${result.updateInfo?.releaseNotes || '无'}`,
        type: 'info',
        confirmText: '立即更新',
        cancelText: '稍后'
      })
      if (shouldUpdate) {
        await window.monotools.internal.updaterStartUpdate(result.updateInfo)
      }
    } else {
      if (result.error) {
        error('检查更新出错: ' + result.error)
      } else {
        info('当前已是最新版本')
      }
    }
  } catch (err: any) {
    console.error('检查更新失败:', err)
    error('检查更新失败: ' + (err.message || '未知错误'))
  } finally {
    isCheckingUpdate.value = false
  }
}

function openGithub(): void {
  window.monotools.shellOpenExternal('https://github.com/lzx8589561/ZTools')
}

function openMonoTools(): void {
  window.monotools.shellOpenExternal('https://github.com/MonoKelvin/MonoTools')
}

async function loadAutoCheckSetting(): Promise<void> {
  try {
    const data = await window.monotools.internal.dbGet('settings-general')
    if (data) {
      autoCheckUpdate.value = data.autoCheckUpdate ?? true
    }
  } catch (err) {
    console.error('加载自动更新设置失败:', err)
  }
}

async function handleAutoCheckUpdateChange(): Promise<void> {
  try {
    // 更新数据库中的设置
    const data = (await window.monotools.internal.dbGet('settings-general')) || {}
    data.autoCheckUpdate = autoCheckUpdate.value
    await window.monotools.internal.dbPut('settings-general', data)
    // 通知主进程
    await window.monotools.internal.updaterSetAutoCheck(autoCheckUpdate.value)
  } catch (err) {
    console.error('更新自动检查更新设置失败:', err)
  }
}
</script>
<template>
  <div class="content-panel">
    <!-- 右上角自动检查更新 -->
    <div class="auto-update-row">
      <label class="auto-update-label" for="auto-check-update">自动检查更新</label>
      <label class="toggle">
        <input
          id="auto-check-update"
          v-model="autoCheckUpdate"
          type="checkbox"
          @change="handleAutoCheckUpdateChange"
        />
        <span class="toggle-slider"></span>
      </label>
    </div>

    <div class="about-container">
      <!-- Logo -->
      <div class="about-logo">
        <img src="/logo.png" alt="MonoTools" draggable="false" />
      </div>

      <!-- 应用名称 -->
      <h1 class="about-title">MonoTools</h1>

      <!-- 版本号 -->
      <div class="about-version">v{{ appVersion }}</div>

      <!-- 信息卡片 -->
      <div class="about-cards">
        <div class="about-card">
          <div class="card-icon card-icon-author">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
              <path
                d="M20 21V19C20 17.9391 19.5786 16.9217 18.8284 16.1716C18.0783 15.4214 17.0609 15 16 15H8C6.93913 15 5.92172 15.4214 5.17157 16.1716C4.42143 16.9217 4 17.9391 4 19V21"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <circle
                cx="12"
                cy="7"
                r="4"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </div>
          <div class="card-label">作者</div>
          <div class="card-value">MonoKelvin</div>
        </div>

        <!-- ZTools 原项目 -->
        <div class="about-card clickable" @click="openGithub">
          <div class="card-icon card-icon-github">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
              <path
                d="M9 19C4 20.5 4 16.5 2 16M16 22V18.13C16.0375 17.6532 15.9731 17.1738 15.811 16.7238C15.6489 16.2738 15.3929 15.8634 15.06 15.52C18.2 15.17 21.5 13.98 21.5 8.52C21.4997 7.12383 20.9627 5.7812 20 4.77C20.4559 3.54851 20.4236 2.19835 19.91 1C19.91 1 18.73 0.650001 16 2.48C13.708 1.85882 11.292 1.85882 9 2.48C6.27 0.650001 5.09 1 5.09 1C4.57638 2.19835 4.54414 3.54851 5 4.77C4.03013 5.7887 3.49252 7.14346 3.5 8.55C3.5 13.97 6.8 15.16 9.94 15.55C9.611 15.89 9.35726 16.2954 9.19531 16.7399C9.03335 17.1844 8.96681 17.658 9 18.13V22"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </div>
          <div class="card-label">ZTools 原项目</div>
          <div class="card-value">
            lzx8589561
            <svg class="external-icon" width="12" height="12" viewBox="0 0 24 24" fill="none">
              <path
                d="M18 13V19C18 19.5304 17.7893 20.0391 17.4142 20.4142C17.0391 20.7893 16.5304 21 16 21H5C4.46957 21 3.96086 20.7893 3.58579 20.4142C3.21071 20.0391 3 19.5304 3 19V8C3 7.46957 3.21071 6.96086 3.58579 6.58579C3.96086 6.21071 4.46957 6 5 6H11"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M15 3H21V9"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M10 14L21 3"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </div>
        </div>

        <!-- 本项目 -->
        <div class="about-card clickable" @click="openMonoTools">
          <div class="card-icon card-icon-github">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
              <path
                d="M9 19C4 20.5 4 16.5 2 16M16 22V18.13C16.0375 17.6532 15.9731 17.1738 15.811 16.7238C15.6489 16.2738 15.3929 15.8634 15.06 15.52C18.2 15.17 21.5 13.98 21.5 8.52C21.4997 7.12383 20.9627 5.7812 20 4.77C20.4559 3.54851 20.4236 2.19835 19.91 1C19.91 1 18.73 0.650001 16 2.48C13.708 1.85882 11.292 1.85882 9 2.48C6.27 0.650001 5.09 1 5.09 1C4.57638 2.19835 4.54414 3.54851 5 4.77C4.03013 5.7887 3.49252 7.14346 3.5 8.55C3.5 13.97 6.8 15.16 9.94 15.55C9.611 15.89 9.35726 16.2954 9.19531 16.7399C9.03335 17.1844 8.96681 17.658 9 18.13V22"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </div>
          <div class="card-label">本项目</div>
          <div class="card-value">
            MonoTools
            <svg class="external-icon" width="12" height="12" viewBox="0 0 24 24" fill="none">
              <path
                d="M18 13V19C18 19.5304 17.7893 20.0391 17.4142 20.4142C17.0391 20.7893 16.5304 21 16 21H5C4.46957 21 3.96086 20.7893 3.58579 20.4142C3.21071 20.0391 3 19.5304 3 19V8C3 7.46957 3.21071 6.96086 3.58579 6.58579C3.96086 6.21071 4.46957 6 5 6H11"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M15 3H21V9"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M10 14L21 3"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </div>
        </div>
      </div>

      <!-- 核心技术 -->
      <div class="about-techs">
        <span class="tech-tag">Electron 41</span>
        <span class="tech-tag">Vue 3</span>
        <span class="tech-tag">TypeScript</span>
        <span class="tech-tag">LMDB</span>
        <span class="tech-tag">Fuse.js</span>
      </div>

      <!-- 检查更新按钮 -->
      <button class="check-update-btn" :disabled="isCheckingUpdate" @click="handleCheckUpdate">
        <svg v-if="!isCheckingUpdate" width="18" height="18" viewBox="0 0 24 24" fill="none">
          <path
            d="M23 4V10H17"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
          <path
            d="M20.49 15C19.84 16.8399 18.6096 18.4187 16.9842 19.4985C15.3588 20.5783 13.4315 21.1006 11.4952 20.9866C9.55886 20.8726 7.70756 20.1286 6.22015 18.8667C4.73274 17.6047 3.6894 15.8932 3.24965 13.9901C2.8099 12.087 2.99716 10.0952 3.78477 8.30479C4.57238 6.51435 5.91702 5.02117 7.61403 4.04909C9.31105 3.07702 11.27 2.67856 13.2091 2.91282C15.1481 3.14708 16.9632 4.0015 18.36 5.35L23 10"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        <svg v-else class="spinning" width="18" height="18" viewBox="0 0 24 24" fill="none">
          <path d="M12 2V6" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
          <path
            d="M12 18V22"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            opacity="0.3"
          />
          <path
            d="M4.93 4.93L7.76 7.76"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          />
          <path
            d="M16.24 16.24L19.07 19.07"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            opacity="0.3"
          />
          <path
            d="M2 12H6"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            opacity="0.5"
          />
          <path
            d="M18 12H22"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            opacity="0.5"
          />
          <path
            d="M4.93 19.07L7.76 16.24"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            opacity="0.3"
          />
          <path
            d="M16.24 7.76L19.07 4.93"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          />
        </svg>
        {{ isCheckingUpdate ? '检查中...' : '检查更新' }}
      </button>

      <!-- 版权信息 -->
      <div class="about-copyright">
        Copyright &copy; 2025-{{ currentYear }} MonoTools. All rights reserved.
      </div>
    </div>
  </div>
</template>

<style scoped>
.content-panel {
  position: relative;
  height: 100%;
}

.about-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 40px 32px 32px;
  box-sizing: border-box;
}

.about-logo {
  width: 96px;
  height: 96px;
  margin-bottom: 16px;
}

.about-logo img {
  width: 100%;
  height: 100%;
  border-radius: 22px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.12);
}

.about-title {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-color);
  margin: 0 0 6px;
}

.about-version {
  font-size: 14px;
  color: var(--text-secondary);
  margin-bottom: 28px;
}

/* 卡片区域 */
.about-cards {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
  width: fit-content;
  max-width: 560px;
  margin: 0 auto 24px;
}

.about-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 18px 12px 16px;
  border-radius: 12px;
  border: 1px solid var(--control-border);
  background: var(--card-bg);
  transition: all 0.2s;
}

.about-card.clickable {
  cursor: pointer;
}

.about-card.clickable:hover {
  border-color: var(--primary-color);
  background: var(--active-bg);
}

.card-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 10px;
}

.card-icon-author {
  background: #e0f2fe;
  color: #0284c7;
}

.card-icon-qq {
  background: #dcfce7;
  color: #16a34a;
}

.card-icon-github {
  background: #f3f4f6;
  color: #374151;
}

.card-icon-sponsor {
  background: #fce7f3;
  color: #ec4899;
}

.card-label {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}

.card-value {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-color);
  display: flex;
  align-items: center;
  gap: 4px;
}

.external-icon {
  color: var(--text-secondary);
}

/* 核心技术标签 */
.about-techs {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: center;
  margin-bottom: 28px;
}

.tech-tag {
  padding: 5px 14px;
  border-radius: 16px;
  font-size: 13px;
  color: var(--text-secondary);
  border: 1px solid var(--control-border);
  background: var(--card-bg);
}

/* 检查更新按钮 */
.check-update-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 32px;
  border-radius: 10px;
  border: none;
  background: #10b981;
  color: var(--text-on-primary);
  font-size: 15px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  margin-bottom: 24px;
}

.check-update-btn:hover:not(:disabled) {
  opacity: 0.9;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
}

.check-update-btn:active:not(:disabled) {
  transform: translateY(0);
}

.check-update-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.spinning {
  animation: spin 1s linear infinite;
}

/* 自动检查更新 */
.auto-update-row {
  position: absolute;
  top: 16px;
  right: 20px;
  display: flex;
  align-items: center;
  gap: 8px;
  z-index: 1;
}

.auto-update-label {
  font-size: 13px;
  color: var(--text-secondary);
}

/* 版权信息 */
.about-copyright {
  font-size: 12px;
  color: var(--text-secondary);
  opacity: 0.6;
}

/* 暗色模式适配 */
@media (prefers-color-scheme: dark) {
  .card-icon-author {
    background: rgba(2, 132, 199, 0.15);
  }

  .card-icon-qq {
    background: rgba(22, 163, 74, 0.15);
  }

  .card-icon-github {
    background: rgba(255, 255, 255, 0.1);
    color: #d1d5db;
  }

  .card-icon-sponsor {
    background: rgba(236, 72, 153, 0.15);
  }
}
</style>
