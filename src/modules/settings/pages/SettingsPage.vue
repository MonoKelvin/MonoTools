<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
// ★ 触发所有模块的 settings 注册 (side-effect import)
import '@/modules/settings'
import SettingsBackBar from '../components/SettingsBackBar.vue'
import SettingsGroup from '../components/SettingsGroup.vue'
import LoadingState from '@/ui/components/LoadingState.vue'
import { MtEmptyState } from '@/ui/components'
import type { SettingValue } from '../types'
import { useSettingsFramework } from '../composables/useSettingsFramework'

const router = useRouter()
const { loading, saving, values, groups, updateValue } = useSettingsFramework()

function handleChange(key: string, value: SettingValue) {
  updateValue(key, value)
}

function handleBack() {
  router.push('/')
}

function onKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    handleBack()
  }
}

onMounted(() => {
  document.addEventListener('keydown', onKeyDown)
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeyDown)
})
</script>

<template>
  <div class="settings-page">
    <SettingsBackBar />
    <div class="settings-page__content">
      <LoadingState v-if="loading" message="加载设置..." />
      <MtEmptyState
        v-else-if="groups.length === 0"
        title="暂无设置项"
        hint="请确保至少一个模块已注册设置"
        icon="inbox"
      />
      <div v-else class="settings-page__groups">
        <SettingsGroup
          v-for="group in groups"
          :key="group.id"
          :group="group"
          :values="values"
          :loading="loading"
          @update:item-value="handleChange"
        />
        <Transition name="fade">
          <div v-if="saving" class="settings-page__saving-indicator glass">
            已保存
          </div>
        </Transition>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--canvas);
  overflow: hidden;
}

.settings-page__content {
  flex: 1;
  overflow-y: auto;
  padding: var(--sp-5);
}

.settings-page__groups {
  max-width: 600px;
}

.settings-page__saving-indicator {
  position: fixed;
  bottom: var(--sp-5);
  right: var(--sp-5);
  padding: var(--sp-2) var(--sp-4);
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--text-tertiary);
  background: var(--glass-bg);
  backdrop-filter: var(--glass-blur);
  border-radius: var(--radius-md);
  border: 1px solid var(--glass-border);
}

/* ========== Page Animations ========== */
@keyframes settings-page-in {
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

@keyframes settings-page-out {
  from {
    opacity: 1;
    transform: translateY(0) scale(1);
    filter: blur(0);
  }
  to {
    opacity: 0;
    transform: translateY(8px) scale(0.985);
    filter: blur(6px);
  }
}

.settings-page-enter-active {
  animation: settings-page-in 220ms var(--ease-out) both;
}

.settings-page-leave-active {
  animation: settings-page-out 160ms var(--ease-out) both;
}
</style>
