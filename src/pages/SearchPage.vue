<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useSearchStore } from '@/stores/search'
import { useSettingsStore } from '@/stores/settings'
import { hotkeyApi } from '@/services'
import { isTauri } from '@/services/env'

import SearchInput from '@/components/common/SearchInput.vue'
import CategoryTabs from '@/components/search/CategoryTabs.vue'
import SearchResults from '@/components/search/SearchResults.vue'
import ActionBar from '@/components/search/ActionBar.vue'

const router = useRouter()
const search = useSearchStore()
const settings = useSettingsStore()

const inputRef = ref<InstanceType<typeof SearchInput> | null>(null)

const onEnter = async () => {
  const item = await search.executeSelected()
  if (item) {
    // close handled by store
  }
}
const onUp = () => search.selectPrev()
const onDown = () => search.selectNext()
const onEscape = () => {
  search.hide()
}

const onSelect = (item: any) => search.executeItem(item)
const onHover = (idx: number) => {
  search.selectedIndex = idx
}

const onQueryChange = (val: string) => search.setQuery(val)

const onCategorySelect = (cat: any) => search.setCategory(cat)

const openSettings = () => router.push({ name: 'settings' })

const tryRegisterHotkey = async () => {
  if (!isTauri) return
  try {
    await hotkeyApi.register(settings.settings.hotkey)
  } catch (err) {
    console.warn('注册热键失败', err)
  }
}

onMounted(async () => {
  await settings.load()
  await tryRegisterHotkey()
})
</script>

<template>
  <div class="search-overlay" @click.self="search.hide()">
    <div class="search-container" @click.stop>
      <SearchInput
        ref="inputRef"
        :model-value="search.query"
        @update:model-value="onQueryChange"
        @enter="onEnter"
        @arrow-down="onDown"
        @arrow-up="onUp"
        @escape="onEscape"
        autofocus
      />
      <CategoryTabs
        :active="search.activeCategory"
        @select="onCategorySelect"
      />
      <SearchResults
        :results="search.filteredResults"
        :loading="search.loading"
        :selected-index="search.selectedIndex"
        @select="onSelect"
        @hover="onHover"
      >
        <template #empty>
          <span v-if="!search.query">输入关键字开始搜索...</span>
          <span v-else>没有找到结果，试试别的关键字</span>
        </template>
      </SearchResults>
      <ActionBar />
    </div>
  </div>
</template>

<style scoped>
.search-overlay {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding-top: 14vh;
  z-index: 10;
  background: transparent;
}
.search-container {
  width: 720px;
  max-width: calc(100vw - 32px);
  background: var(--bg-overlay);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  backdrop-filter: blur(28px) saturate(180%);
  -webkit-backdrop-filter: blur(28px) saturate(180%);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  max-height: 540px;
  animation: slideUp var(--duration-normal) var(--ease-out);
}
@keyframes slideUp {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
