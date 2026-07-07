<template>
  <div :class="['app-root', themeClass]">
    <router-view />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useThemeStore } from '@/stores/theme'

const themeStore = useThemeStore()

const themeClass = computed(() => ({
  'theme-dark': themeStore.mode === 'dark',
  'theme-light': themeStore.mode === 'light',
}))

onMounted(async () => {
  await themeStore.applyTheme()
})
</script>

<style scoped>
.app-root {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
}
</style>
